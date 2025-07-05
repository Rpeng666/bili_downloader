use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::common::client::client::BiliClient;
use crate::downloader::models::{DownloadProgress, FileType, TaskStatus};

use super::error::DownloadError;
use chardetng::EncodingDetector;
use dashmap::DashMap;
use flate2::read::{DeflateDecoder, GzDecoder};
use indicatif::ProgressBar;
use tokio::sync::{Mutex, Semaphore};

use tracing::{debug, error, info, warn};

#[derive(Clone)]
pub struct DownloadCore {
    tasks: Arc<Mutex<DashMap<String, Arc<Mutex<DownloadProgress>>>>>, // task_id -> Task
    state_file: PathBuf,
    semaphore: Arc<Semaphore>, // 控制并发数
    download_client: BiliClient,
}

impl DownloadCore {
    pub fn new(
        max_concurrent: usize,
        state_file: impl AsRef<Path>,
        download_client: &BiliClient,
    ) -> Self {
        let state_file = state_file.as_ref().to_path_buf();
        Self {
            tasks: Arc::new(Mutex::new(DashMap::new())),
            state_file: state_file,
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            download_client: download_client.clone(),
        }
    }

    // 添加新的下载任务
    pub async fn add_task(
        &self,
        url: &str,
        output: &Path,
        file_type: &FileType,
    ) -> Result<String, DownloadError> {
        let task_id = uuid::Uuid::new_v4().to_string();
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|_| DownloadError::SemaphoreError)?;

        debug!("开始添加下载任务: {}, 文件类型: {:?}", task_id, file_type);

        // 根据文件类型选择下载策略
        let strategy = DownloadStrategy::for_file_type(&file_type);

        // 检查内容并获取大小（仅对需要的类型）
        // 获取内容信息（统一调用，避免重复）
        let content_info = get_content_info(&self.download_client, url).await?;

        // 根据策略处理文件大小
        let total_size = match strategy {
            DownloadStrategy::BinaryStream { .. } => {
                // 对于二进制流，检查内容类型和大小
                if content_info.content_type.contains("text/html") {
                    warn!("URL 返回 HTML 内容，可能不是文件下载链接: {}", url);
                    return Err(DownloadError::InvalidUrl("URL 返回 HTML 内容".to_string()));
                }

                match content_info.content_length {
                    Some(size) => {
                        debug!("获取到文件大小: {:.1}MB", size as f64 / 1024.0 / 1024.0);
                        size
                    }
                    None => {
                        warn!("无法获取文件大小，URL: {}", url);
                        // 对于无法获取大小的情况，返回0，让下载逻辑自行处理
                        0
                    }
                }
            }
            DownloadStrategy::TextContent { .. } | DownloadStrategy::Image { .. } => {
                content_info.content_length.unwrap_or(0)
            }
        };

        let task = DownloadProgress {
            task_id: task_id.clone(),
            url: url.to_string(),
            output_path: output.to_path_buf(),
            total_size,
            downloaded: 0,
            status: TaskStatus::Queued,
        };

        let tasks = self.tasks.lock().await;
        let task_id_clone = task_id.clone();
        // 检查任务是否已存在
        if tasks.contains_key(&task_id_clone) {
            return Err(DownloadError::TaskAlreadyExists(task_id_clone.clone()));
        }
        tasks.insert(task_id_clone.clone(), Arc::new(Mutex::new(task)));
        let tasks = Arc::clone(&self.tasks);
        let download_client = self.download_client.clone();
        let file_type_owned = file_type.clone();

        // 启动下载任务，传递文件类型和策略
        tokio::spawn(async move {
            Self::run(
                tasks,
                task_id_clone.clone(),
                download_client,
                file_type_owned,
                strategy,
                content_info,
            )
            .await;
        });

        Ok(task_id)
    }

    // 获取任务状态
    pub async fn get_task_status(&self, task_id: &str) -> Option<TaskStatus> {
        let task = {
            let tasks = self.tasks.lock().await;
            let task = tasks.get(task_id).expect("任务不存在");
            Arc::clone(&task)
        };

        Some(task.lock().await.status.clone())
    }

    async fn run(
        tasks: Arc<Mutex<DashMap<String, Arc<Mutex<DownloadProgress>>>>>,
        task_id: String,
        download_client: BiliClient,
        file_type: FileType,
        strategy: DownloadStrategy,
        content_info: DownloadContent,
    ) {
        // 获取任务锁
        let task_lock = {
            let tasks = tasks.lock().await;
            let task = tasks.get(&task_id).expect("任务不存在");
            Arc::clone(&task)
        };

        // 一次性获取所需数据
        let (task_id, output_path, url) = {
            let task_guard = task_lock.lock().await;
            (
                task_guard.task_id.clone(),
                task_guard.output_path.clone(),
                task_guard.url.clone(),
            )
        };

        info!("开始下载任务: {}, 类型: {:?}", task_id, file_type);

        // 更新任务状态为下载中
        {
            let mut task_guard = task_lock.lock().await;
            task_guard.status = TaskStatus::Downloading;
        }

        // 根据下载策略执行不同的下载逻辑
        let result = match strategy {
            DownloadStrategy::BinaryStream {
                show_progress,
                chunk_size,
            } => {
                Self::download_binary_stream(
                    &download_client,
                    &url,
                    &output_path,
                    show_progress,
                    chunk_size,
                    &task_lock,
                    &file_type,
                )
                .await
            }
            DownloadStrategy::TextContent {
                expected_content_type,
            } => {
                Self::download_text_content(
                    &download_client,
                    &url,
                    &output_path,
                    expected_content_type,
                    &content_info,
                )
                .await
            }
            DownloadStrategy::Image { validate_format } => {
                Self::download_image(&download_client, &url, &output_path, validate_format).await
            }
        };

        // 更新任务状态
        {
            let mut task_guard = task_lock.lock().await;
            match result {
                Ok(_) => {
                    task_guard.status = TaskStatus::Completed;
                    info!("✅ 下载任务完成: {}", task_id);
                }
                Err(DownloadError::RateLimited(msg)) => {
                    // 风控错误，跳过任务而不是失败
                    task_guard.status = TaskStatus::Skipped(msg.clone());
                    warn!("⏭️ 下载任务已跳过: {}, 原因: {}", task_id, msg);
                    info!("💡 提示: 这通常是临时的风控限制，建议稍后重试");
                }
                Err(e) => {
                    task_guard.status = TaskStatus::Error(e.to_string());
                    error!("❌ 下载任务失败: {}, 错误: {}", task_id, e);
                }
            }
        }
    }

    // 二进制流下载方法（用于视频、音频等大文件）
    async fn download_binary_stream(
        download_client: &BiliClient,
        url: &str,
        output_path: &Path,
        show_progress: bool,
        _chunk_size: usize,
        task_lock: &Arc<Mutex<DownloadProgress>>,
        file_type: &FileType,
    ) -> Result<(), DownloadError> {
        debug!("开始下载二进制文件: {}", url);

        // 首先检查是否存在部分下载的文件
        let mut start_pos = 0u64;
        if output_path.exists() {
            debug!("检测到部分下载文件: {:?}", output_path);
            start_pos = tokio::fs::metadata(output_path)
                .await
                .map_err(|e| DownloadError::IoError(e.to_string()))?
                .len();
            if start_pos > 0 {
                info!("检测到部分下载文件，从 {} 字节处继续下载", start_pos);
            }
        }

        // 获取文件总大小（从任务信息中获取，避免重复网络请求）
        let total_size = {
            let task_guard = task_lock.lock().await;
            task_guard.total_size
        };

        // 如果文件已经完整下载，直接返回
        if start_pos >= total_size && total_size > 0 {
            info!("文件已完整下载，跳过: {}", output_path.display());
            return Ok(());
        }

        // 创建进度条
        let pb = if show_progress && total_size > 0 {
            let pb = ProgressBar::new(total_size);
            pb.set_style(
                indicatif::ProgressStyle::with_template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta}) {msg}",
                )
                .unwrap()
                .progress_chars("#>-")
            );
            pb.set_position(start_pos);
            Some(pb)
        } else {
            None
        };

        // 使用重试机制下载
        const MAX_RETRIES: usize = 20; // 增加到20次重试
        const RETRY_DELAY_SECONDS: u64 = 2; // 减少延时到2秒

        for attempt in 1..=MAX_RETRIES {
            match Self::download_with_resume(
                download_client,
                url,
                output_path,
                start_pos,
                total_size,
                file_type,
                &pb,
                task_lock,
            )
            .await
            {
                Ok(_) => {
                    if let Some(pb) = pb {
                        pb.finish_with_message("下载完成");
                    }
                    info!("文件下载成功: {}", output_path.display());
                    return Ok(());
                }
                Err(DownloadError::StreamError(msg)) if attempt < MAX_RETRIES => {
                    // 计算下载进度
                    let current_pos = if output_path.exists() {
                        tokio::fs::metadata(output_path)
                            .await
                            .map_err(|e| DownloadError::IoError(e.to_string()))?
                            .len()
                    } else {
                        start_pos
                    };

                    let progress_percent = if total_size > 0 {
                        (current_pos as f64 / total_size as f64 * 100.0).round() as u32
                    } else {
                        0
                    };

                    warn!(
                        "下载中断 (尝试 {}/{}，进度 {}%): {}",
                        attempt, MAX_RETRIES, progress_percent, msg
                    );

                    // 更新断点位置
                    start_pos = current_pos;
                    if start_pos > 0 {
                        info!(
                            "断点续传从 {} 字节处继续 ({:.1}MB)",
                            start_pos,
                            start_pos as f64 / 1024.0 / 1024.0
                        );

                        if let Some(pb) = &pb {
                            pb.set_position(start_pos);
                            pb.set_message(format!(
                                "重试 {}/{} ({}%)",
                                attempt + 1,
                                MAX_RETRIES,
                                progress_percent
                            ));
                        }
                    }

                    // 如果已经下载了相当一部分，减少延时
                    let delay = if progress_percent > 50 {
                        1 // 下载过半时，快速重试
                    } else if progress_percent > 10 {
                        2 // 有一定进度时，中等延时
                    } else {
                        RETRY_DELAY_SECONDS // 刚开始时，正常延时
                    };

                    tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
                }
                Err(DownloadError::RateLimited(_)) => {
                    // 风控错误，直接返回，不再重试
                    if let Some(pb) = pb {
                        pb.finish_with_message("访问受限");
                    }
                    return Err(DownloadError::RateLimited(format!(
                        "下载过程中遇到访问限制，URL: {}",
                        url
                    )));
                }
                Err(e) => {
                    if let Some(pb) = pb {
                        pb.finish_with_message("下载失败");
                    }
                    return Err(e);
                }
            }
        }

        if let Some(pb) = pb {
            pb.finish_with_message("重试次数用尽");
        }
        Err(DownloadError::StreamError(format!(
            "下载失败，已重试 {} 次",
            MAX_RETRIES
        )))
    }

    // 带断点续传的下载方法
    async fn download_with_resume(
        download_client: &BiliClient,
        url: &str,
        output_path: &Path,
        start_pos: u64,
        total_size: u64,
        file_type: &FileType,
        pb: &Option<ProgressBar>,
        task_lock: &Arc<Mutex<DownloadProgress>>,
    ) -> Result<(), DownloadError> {
        use futures::StreamExt;
        use tokio::io::AsyncWriteExt;

        // 对于音频和视频文件，构建带特殊请求头的请求
        let mut request_builder = match file_type {
            FileType::Video | FileType::Audio => {
                let mut builder = download_client.inner.get(url);
                for (key, value) in BiliClient::get_video_download_headers(url).iter() {
                    builder = builder.header(key, value);
                }
                builder
            }
            _ => download_client.inner.get(url),
        };

        // 对于B站的音频流，总是使用Range请求，因为某些音频流不支持完整GET请求
        let should_use_range = start_pos > 0 || matches!(file_type, FileType::Audio);

        if should_use_range {
            let range_header = if start_pos > 0 {
                format!("bytes={}-", start_pos)
            } else {
                format!("bytes=0-") // 从头开始但使用Range请求
            };

            request_builder = request_builder.header(reqwest::header::RANGE, range_header.clone());

            if start_pos > 0 {
                debug!("使用断点续传，起始位置: {} 字节", start_pos);
                debug!("发送Range请求头: {}", range_header);
            } else {
                debug!("使用Range请求从头下载（音频流兼容性）: {}", range_header);
            }
        } else {
            debug!("发送完整文件请求（无Range头）");
        }

        let response = request_builder
            .send()
            .await
            .map_err(DownloadError::HttpError)?;

        // 记录详细的请求和响应信息
        debug!("下载请求详情:");
        debug!("  URL: {}", url);
        debug!("  文件类型: {:?}", file_type);
        debug!("  起始位置: {} 字节", start_pos);
        debug!("  Range请求: {}", should_use_range);
        debug!("响应详情:");
        debug!("  状态码: {}", response.status());
        debug!(
            "  Content-Length: {:?}",
            response.headers().get("content-length")
        );
        debug!(
            "  Content-Range: {:?}",
            response.headers().get("content-range")
        );
        debug!(
            "  Accept-Ranges: {:?}",
            response.headers().get("accept-ranges")
        );
        debug!(
            "  Content-Type: {:?}",
            response.headers().get("content-type")
        );

        // 检查响应状态
        Self::check_response_status(&response, url)?;

        // 如果是206响应，解析Content-Range
        if response.status() == reqwest::StatusCode::PARTIAL_CONTENT {
            if let Some(content_range) = response.headers().get("content-range") {
                if let Ok(range_str) = content_range.to_str() {
                    debug!("服务器返回部分内容: {}", range_str);
                    // 解析 "bytes 1048576-2097151/123456789" 格式
                    if let Some(parts) = range_str.strip_prefix("bytes ") {
                        if let Some((range_part, total_part)) = parts.split_once('/') {
                            if let Some((start, end)) = range_part.split_once('-') {
                                debug!("  实际返回范围: {} - {}", start, end);
                                debug!("  文件总大小: {}", total_part);
                            }
                        }
                    }
                }
            }
        }

        // 打开文件用于写入
        let mut file = if start_pos > 0 {
            // 断点续传模式：追加写入
            tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(output_path)
                .await
                .map_err(|e| DownloadError::IoError(e.to_string()))?
        } else {
            // 新下载：覆盖写入
            tokio::fs::File::create(output_path)
                .await
                .map_err(|e| DownloadError::IoError(e.to_string()))?
        };

        let mut stream = response.bytes_stream();
        let mut downloaded = start_pos;

        // 设置下载超时和心跳检测
        const CHUNK_TIMEOUT_SECONDS: u64 = 60; // 增加到60秒超时
        const HEARTBEAT_INTERVAL: usize = 100; // 每100个chunk输出一次日志
        let mut chunk_count = 0;

        loop {
            let chunk_result = tokio::time::timeout(
                tokio::time::Duration::from_secs(CHUNK_TIMEOUT_SECONDS),
                stream.next(),
            )
            .await;

            let chunk_option = match chunk_result {
                Ok(opt) => opt,
                Err(_) => {
                    warn!(
                        "下载超时 ({}秒)，已下载 {:.1}MB",
                        CHUNK_TIMEOUT_SECONDS,
                        downloaded as f64 / 1024.0 / 1024.0
                    );
                    return Err(DownloadError::StreamError(format!(
                        "下载超时 ({}秒)，网络连接可能中断",
                        CHUNK_TIMEOUT_SECONDS
                    )));
                }
            };

            let chunk = match chunk_option {
                Some(Ok(chunk)) => chunk,
                Some(Err(error)) => {
                    warn!(
                        "流读取错误，已下载 {:.1}MB: {}",
                        downloaded as f64 / 1024.0 / 1024.0,
                        error
                    );
                    return Err(DownloadError::StreamError(format!("流读取错误: {}", error)));
                }
                None => {
                    // 流结束
                    debug!(
                        "下载流正常结束，总计 {:.1}MB",
                        downloaded as f64 / 1024.0 / 1024.0
                    );
                    break;
                }
            };

            // 写入数据
            file.write_all(&chunk)
                .await
                .map_err(|e| DownloadError::IoError(e.to_string()))?;

            downloaded += chunk.len() as u64;
            chunk_count += 1;

            // 定期输出下载状态（避免日志过多）
            if chunk_count % HEARTBEAT_INTERVAL == 0 {
                debug!(
                    "下载中: {:.1}MB/{:.1}MB ({:.1}%)",
                    downloaded as f64 / 1024.0 / 1024.0,
                    total_size as f64 / 1024.0 / 1024.0,
                    if total_size > 0 {
                        (downloaded as f64 / total_size as f64) * 100.0
                    } else {
                        0.0
                    }
                );
            }

            // 更新进度
            if let Some(pb) = pb {
                pb.set_position(downloaded);
            }

            // 更新任务进度（减少锁争用）
            if chunk_count % 50 == 0 {
                // 每50个chunk更新一次任务进度
                let mut task_guard = task_lock.lock().await;
                task_guard.downloaded = downloaded;
            }

            // 检查是否下载完成
            if total_size > 0 && downloaded >= total_size {
                debug!(
                    "下载完成，达到预期大小: {:.1}MB",
                    total_size as f64 / 1024.0 / 1024.0
                );
                break;
            }
        }

        // 确保数据写入磁盘
        file.flush()
            .await
            .map_err(|e| DownloadError::IoError(e.to_string()))?;

        // 最终更新任务进度
        {
            let mut task_guard = task_lock.lock().await;
            task_guard.downloaded = downloaded;
        }

        // 验证下载完整性
        if total_size > 0 && downloaded < total_size {
            let missing_mb = (total_size - downloaded) as f64 / 1024.0 / 1024.0;
            warn!(
                "下载不完整: {:.1}MB/{:.1}MB ({:.1}%)，还差 {:.1}MB",
                downloaded as f64 / 1024.0 / 1024.0,
                total_size as f64 / 1024.0 / 1024.0,
                (downloaded as f64 / total_size as f64) * 100.0,
                missing_mb
            );
            return Err(DownloadError::StreamError(format!(
                "下载不完整: {:.1}MB/{:.1}MB，还差 {:.1}MB",
                downloaded as f64 / 1024.0 / 1024.0,
                total_size as f64 / 1024.0 / 1024.0,
                missing_mb
            )));
        }

        debug!("下载完成: {:.1}MB", downloaded as f64 / 1024.0 / 1024.0);
        Ok(())
    }

    // 文本内容下载方法（用于弹幕、字幕等）
    async fn download_text_content(
        download_client: &BiliClient,
        url: &str,
        output_path: &Path,
        expected_content_type: Option<String>,
        content_info: &DownloadContent,
    ) -> Result<(), DownloadError> {
        // 验证内容类型
        if let Some(expected) = expected_content_type {
            if !content_info.content_type.contains(&expected) {
                warn!(
                    "内容类型不匹配，期望: {}, 实际: {}",
                    expected, content_info.content_type
                );
            }
        }

        debug!("开始下载文本内容: {}", url);

        let response = download_client
            .get_raw_response(url)
            .await
            .map_err(|e| DownloadError::InvalidState(e.to_string()))?;

        // 检查响应状态
        Self::check_response_status(&response, url)?;

        // 检查响应状态并处理特殊情况
        Self::check_response_status(&response, url)
            .map_err(|e| DownloadError::InvalidState(format!("响应状态检查失败: {}", e)))?;

        // 检查内容编码
        let content_encoding = response
            .headers()
            .get("content-encoding")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        debug!("Content-Encoding: {:?}", content_encoding);

        // 获取原始字节数据
        let raw_bytes = response.bytes().await.map_err(DownloadError::HttpError)?;

        // 解压缩内容（如果需要）
        let decompressed_bytes = Self::decompress_content(&raw_bytes, content_encoding.as_deref())?;

        // 自动探测编码
        let mut detector = EncodingDetector::new();
        detector.feed(&decompressed_bytes, true);
        let encoding = detector.guess(None, true);

        // 解码内容
        let (decoded, _, had_errors) = encoding.decode(&decompressed_bytes);

        if had_errors {
            warn!("文本解码过程中发现错误，可能存在字符丢失");
        }

        debug!("文本内容长度: {} 字节", decoded.len());

        // 写入文件
        tokio::fs::write(output_path, decoded.into_owned())
            .await
            .map_err(|e| DownloadError::IoError(e.to_string()))?;

        debug!("文本内容下载完成: {}", output_path.display());
        Ok(())
    }

    // 图片下载方法
    async fn download_image(
        download_client: &BiliClient,
        url: &str,
        output_path: &Path,
        _validate_format: bool,
    ) -> Result<(), DownloadError> {
        debug!("开始下载图片: {}", url);

        let response = download_client
            .get_raw_response(url)
            .await
            .map_err(|e| DownloadError::InvalidState(e.to_string()))?;

        // 检查响应状态
        Self::check_response_status(&response, url)?;

        // 检查响应状态并处理特殊情况
        Self::check_response_status(&response, url)
            .map_err(|e| DownloadError::InvalidState(format!("响应状态检查失败: {}", e)))?;

        // 检查内容编码（图片通常不会压缩，但以防万一）
        let content_encoding = response
            .headers()
            .get("content-encoding")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // 获取字节数据
        let raw_bytes = response.bytes().await.map_err(DownloadError::HttpError)?;

        // 解压缩内容（如果需要）
        let final_bytes = if content_encoding.is_some() {
            Self::decompress_content(&raw_bytes, content_encoding.as_deref())?
        } else {
            raw_bytes.to_vec()
        };

        // 写入文件
        tokio::fs::write(output_path, &final_bytes)
            .await
            .map_err(|e| DownloadError::IoError(e.to_string()))?;

        debug!("图片下载完成: {}", output_path.display());
        Ok(())
    }

    // 解压缩内容的辅助函数
    fn decompress_content(
        bytes: &[u8],
        content_encoding: Option<&str>,
    ) -> Result<Vec<u8>, DownloadError> {
        match content_encoding {
            Some("deflate") => {
                debug!("检测到 deflate 压缩，开始解压缩");
                let mut decoder = DeflateDecoder::new(bytes);
                let mut decompressed = Vec::new();
                decoder.read_to_end(&mut decompressed).map_err(|e| {
                    DownloadError::InvalidState(format!("deflate 解压缩失败: {}", e))
                })?;
                Ok(decompressed)
            }
            Some("gzip") => {
                debug!("检测到 gzip 压缩，开始解压缩");
                let mut decoder = GzDecoder::new(bytes);
                let mut decompressed = Vec::new();
                decoder
                    .read_to_end(&mut decompressed)
                    .map_err(|e| DownloadError::InvalidState(format!("gzip 解压缩失败: {}", e)))?;
                Ok(decompressed)
            }
            Some("br") => {
                debug!("检测到 brotli 压缩，reqwest 应该自动处理");
                // Brotli 通常由 reqwest 自动处理，如果到这里说明可能需要手动处理
                // 但我们暂时返回原始数据，因为 reqwest 通常会处理这种情况
                Ok(bytes.to_vec())
            }
            Some(encoding) => {
                warn!("未知的内容编码: {}", encoding);
                Ok(bytes.to_vec())
            }
            None => {
                debug!("无压缩编码");
                Ok(bytes.to_vec())
            }
        }
    }

    // 检查响应状态并处理特殊情况
    fn check_response_status(response: &reqwest::Response, url: &str) -> Result<(), DownloadError> {
        let status = response.status();
        debug!("Response Status: {}", status);

        match status {
            // 成功状态码
            reqwest::StatusCode::OK => {
                debug!("✅ 响应状态正常: {} (完整内容)", status);
                Ok(())
            }
            reqwest::StatusCode::PARTIAL_CONTENT => {
                debug!("✅ 响应状态正常: {} (部分内容/断点续传)", status);
                Ok(())
            }
            // 风控相关错误
            reqwest::StatusCode::FORBIDDEN => {
                warn!("🚫 检测到 403 Forbidden 状态码，可能触发了风控机制");

                // 进行详细的403错误分析
                let analysis = Self::analyze_403_error(url, response);
                warn!("{}", analysis);

                warn!("💡 建议：等待一段时间后重试，或检查 cookies 是否有效");
                Err(DownloadError::RateLimited(format!(
                    "访问被拒绝 (403 Forbidden)，URL: {}，可能触发了风控机制。{}",
                    url, analysis
                )))
            }
            reqwest::StatusCode::TOO_MANY_REQUESTS => {
                warn!("⚠️ 检测到 429 Too Many Requests 状态码，请求过于频繁");
                Err(DownloadError::RateLimited(format!(
                    "请求过于频繁 (429 Too Many Requests)，URL: {}，请降低请求频率",
                    url
                )))
            }
            reqwest::StatusCode::UNAUTHORIZED => {
                warn!("🔐 检测到 401 Unauthorized 状态码，认证失败");
                Err(DownloadError::RateLimited(format!(
                    "认证失败 (401 Unauthorized)，URL: {}，请检查登录状态",
                    url
                )))
            }
            // 范围请求错误
            reqwest::StatusCode::RANGE_NOT_SATISFIABLE => {
                warn!("📏 检测到 416 Range Not Satisfiable，范围请求无效");
                Err(DownloadError::InvalidState(format!(
                    "范围请求无效 (416 Range Not Satisfiable)，URL: {}，可能文件已完整下载",
                    url
                )))
            }
            // 404 Not Found 错误
            reqwest::StatusCode::NOT_FOUND => {
                warn!("🔍 检测到 404 Not Found 状态码");

                // 检查是否是B站音频流URL
                if url.contains("bilivideo.com") || url.contains("bilivideo.cn") {
                    if url.contains("audio") || url.contains("30280") || url.contains("30232") {
                        warn!("💡 这是B站音频流URL，可能需要Range请求才能正常访问");
                        warn!("💡 建议：尝试使用Range请求头 'bytes=0-' 进行下载");
                    }
                }

                Err(DownloadError::InvalidState(format!(
                    "资源不存在 (404 Not Found)，URL: {}。可能原因：1. URL已失效 2. 需要Range请求头 3. 权限不足",
                    url
                )))
            }
            // 其他成功状态
            status if status.is_success() => {
                debug!("✅ 响应状态正常: {}", status);
                Ok(())
            }
            // 其他错误
            _ => {
                warn!("❌ 非成功状态码: {}", status);
                Err(DownloadError::InvalidState(format!(
                    "HTTP 请求失败，状态码: {}，URL: {}",
                    status, url
                )))
            }
        }
    }

    // 详细分析403错误的原因
    fn analyze_403_error(url: &str, _response: &reqwest::Response) -> String {
        let mut analysis = Vec::new();

        // 检查URL类型
        if url.contains("bilivideo.com") || url.contains("bilivideo.cn") {
            analysis.push("✓ 这是B站CDN地址，需要特殊的请求头");
            analysis.push("❌ 可能缺少必需的请求头：Origin, Sec-Fetch-Dest, Sec-Fetch-Mode");
        }

        // 常见的403原因分析
        analysis.push("🔍 可能的原因分析：");
        analysis.push("  1. Cookie过期或无效 - 请重新登录获取新的Cookie");
        analysis.push("  2. 缺少关键请求头 - 确保包含Origin、Referer等");
        analysis.push("  3. 请求频率过高 - B站实施了频率限制");
        analysis.push("  4. IP被临时封禁 - 更换网络或等待解封");
        analysis.push("  5. 高清视频需要大会员 - 检查账号权限");
        analysis.push("  6. 地区限制 - 某些内容可能有地理限制");

        // 解决建议
        analysis.push("💡 解决建议：");
        analysis.push("  • 检查并更新Cookie (SESSDATA等关键字段)");
        analysis.push("  • 降低请求频率，添加延时");
        analysis.push("  • 确保User-Agent模拟真实浏览器");
        analysis.push("  • 尝试使用不同的视频质量");
        analysis.push("  • 等待10-30分钟后重试");

        format!("403 Forbidden 详细分析:\n{}", analysis.join("\n"))
    }
}

// 下载策略，用于处理不同类型的下载内容
#[derive(Debug, Clone)]
pub enum DownloadStrategy {
    // 二进制流下载（视频、音频等大文件）
    BinaryStream {
        show_progress: bool,
        chunk_size: usize,
    },
    // 文本内容下载（弹幕、字幕等小文件）
    TextContent {
        expected_content_type: Option<String>,
    },
    // 图片下载
    Image {
        validate_format: bool,
    },
}

impl DownloadStrategy {
    // 根据文件类型选择合适的下载策略
    pub fn for_file_type(file_type: &FileType) -> Self {
        match file_type {
            FileType::Video | FileType::Audio => DownloadStrategy::BinaryStream {
                show_progress: true,
                chunk_size: 8192,
            },
            FileType::Danmaku => DownloadStrategy::TextContent {
                expected_content_type: Some("text/xml".to_string()),
            },
            FileType::Subtitle => DownloadStrategy::TextContent {
                expected_content_type: Some("text/plain".to_string()),
            },
            FileType::Image => DownloadStrategy::Image {
                validate_format: true,
            },
            FileType::Other(_) => DownloadStrategy::BinaryStream {
                show_progress: false,
                chunk_size: 4096,
            },
        }
    }
}

// 下载内容信息
#[derive(Debug)]
pub struct DownloadContent {
    pub content_type: String,
    pub content_length: Option<u64>,
    pub is_text: bool,
}

// 获取内容信息（内容类型、大小等）
async fn get_content_info(
    client: &BiliClient,
    url: &str,
) -> Result<DownloadContent, DownloadError> {
    // 检查是否是B站的视频/音频流URL，如果是则使用专用请求头
    let resp = if url.contains("bilivideo.com") || url.contains("bilivideo.cn") {
        debug!("检测到B站CDN地址，使用视频下载专用请求头");
        client
            .inner
            .head(url)
            .headers(BiliClient::get_video_download_headers(url))
            .send()
            .await
            .map_err(DownloadError::HttpError)?
    } else {
        client
            .inner
            .head(url)
            .send()
            .await
            .map_err(DownloadError::HttpError)?
    };

    // 检查状态码
    let status = resp.status();
    debug!("URL: {}", url);
    debug!("Response Status: {}", status);

    // 如果HEAD请求失败，尝试用GET请求获取部分内容来获取信息
    if status == reqwest::StatusCode::NOT_FOUND || status == reqwest::StatusCode::METHOD_NOT_ALLOWED
    {
        warn!(
            "HEAD 请求失败 (状态码: {})，尝试使用 GET 请求获取内容信息",
            status
        );
        return get_content_info_via_get(client, url).await;
    }

    // 特别处理风控情况
    match status {
        reqwest::StatusCode::FORBIDDEN => {
            warn!("🚫 HEAD 请求遇到 403 Forbidden，可能触发了风控机制");
            return Err(DownloadError::RateLimited(format!(
                "获取内容信息时访问被拒绝 (403 Forbidden)，URL: {}",
                url
            )));
        }
        reqwest::StatusCode::TOO_MANY_REQUESTS => {
            warn!("⚠️ HEAD 请求遇到 429 Too Many Requests");
            return Err(DownloadError::RateLimited(format!(
                "获取内容信息时请求过于频繁 (429 Too Many Requests)，URL: {}",
                url
            )));
        }
        _ if !status.is_success() => {
            warn!("❌ HEAD 请求失败，状态码: {}", status);
            return Err(DownloadError::InvalidState(format!(
                "获取内容信息失败，状态码: {}，URL: {}",
                status, url
            )));
        }
        _ => {
            debug!("✅ HEAD 请求成功");
        }
    }
    debug!("Response Headers: {:?}", resp.headers());
    debug!(
        "Content-Encoding: {:?}",
        resp.headers().get("Content-Encoding")
    );

    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|ct| ct.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();

    let content_length = resp
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|ct_len| ct_len.to_str().ok())
        .and_then(|ct_len| ct_len.parse().ok());

    let is_text = content_type.starts_with("text/")
        || content_type.contains("xml")
        || content_type.contains("json");

    debug!("Content Type: {}", content_type);
    debug!("Content Length: {:?}", content_length);
    debug!("Is Text: {}", is_text);

    Ok(DownloadContent {
        content_type,
        content_length,
        is_text,
    })
}

// 通过GET请求获取内容信息（当HEAD请求失败时的备用方案）
async fn get_content_info_via_get(
    client: &BiliClient,
    url: &str,
) -> Result<DownloadContent, DownloadError> {
    debug!("使用 GET 请求获取内容信息: {}", url);

    // 构建请求，使用Range头只请求前1024字节来获取头信息
    let resp = if url.contains("bilivideo.com") || url.contains("bilivideo.cn") {
        client
            .inner
            .get(url)
            .headers(BiliClient::get_video_download_headers(url))
            .header(reqwest::header::RANGE, "bytes=0-1023") // 只请求前1KB
            .send()
            .await
            .map_err(DownloadError::HttpError)?
    } else {
        client
            .inner
            .get(url)
            .header(reqwest::header::RANGE, "bytes=0-1023") // 只请求前1KB
            .send()
            .await
            .map_err(DownloadError::HttpError)?
    };

    let status = resp.status();
    debug!("GET 请求状态: {}", status);

    // 检查状态码（206 Partial Content 或 200 OK 都是正常的）
    match status {
        reqwest::StatusCode::OK | reqwest::StatusCode::PARTIAL_CONTENT => {
            debug!("✅ GET 请求成功");
        }
        reqwest::StatusCode::FORBIDDEN => {
            warn!("🚫 GET 请求遇到 403 Forbidden");
            return Err(DownloadError::RateLimited(format!(
                "GET 请求访问被拒绝 (403 Forbidden)，URL: {}",
                url
            )));
        }
        _ => {
            warn!("❌ GET 请求失败，状态码: {}", status);
            return Err(DownloadError::InvalidState(format!(
                "GET 请求失败，状态码: {}，URL: {}",
                status, url
            )));
        }
    }

    debug!("GET Response Headers: {:?}", resp.headers());

    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|ct| ct.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();

    // 对于部分内容请求，尝试从Content-Range头获取总大小
    let content_length = if let Some(content_range) = resp.headers().get("content-range") {
        if let Ok(range_str) = content_range.to_str() {
            // Content-Range: bytes 0-1023/123456789
            if let Some(total_size_str) = range_str.split('/').nth(1) {
                total_size_str.parse().ok()
            } else {
                None
            }
        } else {
            None
        }
    } else {
        // 如果没有Content-Range，尝试从Content-Length获取
        resp.headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|ct_len| ct_len.to_str().ok())
            .and_then(|ct_len| ct_len.parse().ok())
    };

    let is_text = content_type.starts_with("text/")
        || content_type.contains("xml")
        || content_type.contains("json");

    debug!("Content Type (via GET): {}", content_type);
    debug!("Content Length (via GET): {:?}", content_length);
    debug!("Is Text: {}", is_text);

    Ok(DownloadContent {
        content_type,
        content_length,
        is_text,
    })
}
