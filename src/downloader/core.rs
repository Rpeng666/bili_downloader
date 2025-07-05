use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::io::Read;

use crate::common::client::client::BiliClient;
use crate::downloader::models::{DownloadProgress, FileType, TaskStatus};

use super::error::DownloadError;
use chardetng::EncodingDetector;
use dashmap::DashMap;
use flate2::read::{DeflateDecoder, GzDecoder};
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
        let (total_size, content_info) = match strategy {
            DownloadStrategy::BinaryStream { .. } => {
                let size = get_remote_file_size(url).await?;
                let content_info = get_content_info(url).await?;
                (size, content_info)
            }
            DownloadStrategy::TextContent { .. } | DownloadStrategy::Image { .. } => {
                let content_info = get_content_info(url).await?;
                let size = content_info.content_length.unwrap_or(0);
                (size, content_info)
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
    ) -> Result<(), DownloadError> {
        use futures::StreamExt;
        use indicatif::ProgressBar;

        let response = download_client
            .get_raw_response(url)
            .await
            .map_err(|e| DownloadError::InvalidState(e.to_string()))?;

        // 检查响应状态
        Self::check_response_status(&response, url)?;

        // 检查响应状态并处理特殊情况
        Self::check_response_status(&response, url).map_err(|e| {
            DownloadError::InvalidState(format!("响应状态检查失败: {}", e))
        })?;

        let total_size = response
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|ct_len| ct_len.to_str().ok())
            .and_then(|ct_len| ct_len.parse().ok())
            .unwrap_or(0u64);

        // 创建进度条（仅在需要时）
        let pb = if show_progress && total_size > 0 {
            let pb = ProgressBar::new(total_size);
            pb.set_style(
                indicatif::ProgressStyle::with_template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})",
                )
                .unwrap()
                .progress_chars("#>-")
            );
            Some(pb)
        } else {
            None
        };

        debug!("开始下载二进制文件: {}", url);

        let mut file = tokio::fs::File::create(output_path)
            .await
            .map_err(DownloadError::IoError)?;

        let mut stream = response.bytes_stream();

        // 下载并显示进度
        let mut downloaded = 0u64;
        while let Some(chunk_result) = stream.next().await {
            let chunk = match chunk_result {
                Ok(chunk) => chunk,
                Err(error) => {
                    if let Some(pb) = &pb {
                        pb.finish_with_message("下载失败");
                    }
                    return Err(DownloadError::StreamError(error.to_string()));
                }
            };

            tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
                .await
                .map_err(DownloadError::IoError)?;

            downloaded += chunk.len() as u64;

            // 更新进度条和任务进度
            if let Some(pb) = &pb {
                pb.set_position(downloaded);
            }

            // 更新任务进度
            {
                let mut task_guard = task_lock.lock().await;
                task_guard.downloaded = downloaded;
            }
        }

        if let Some(pb) = pb {
            pb.finish_with_message("下载完成");
        }

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
        Self::check_response_status(&response, url).map_err(|e| {
            DownloadError::InvalidState(format!("响应状态检查失败: {}", e))
        })?;

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
            .map_err(DownloadError::IoError)?;

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
        Self::check_response_status(&response, url).map_err(|e| {
            DownloadError::InvalidState(format!("响应状态检查失败: {}", e))
        })?;

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
            .map_err(DownloadError::IoError)?;

        debug!("图片下载完成: {}", output_path.display());
        Ok(())
    }

    // 解压缩内容的辅助函数
    fn decompress_content(bytes: &[u8], content_encoding: Option<&str>) -> Result<Vec<u8>, DownloadError> {
        match content_encoding {
            Some("deflate") => {
                debug!("检测到 deflate 压缩，开始解压缩");
                let mut decoder = DeflateDecoder::new(bytes);
                let mut decompressed = Vec::new();
                decoder.read_to_end(&mut decompressed)
                    .map_err(|e| DownloadError::InvalidState(format!("deflate 解压缩失败: {}", e)))?;
                Ok(decompressed)
            },
            Some("gzip") => {
                debug!("检测到 gzip 压缩，开始解压缩");
                let mut decoder = GzDecoder::new(bytes);
                let mut decompressed = Vec::new();
                decoder.read_to_end(&mut decompressed)
                    .map_err(|e| DownloadError::InvalidState(format!("gzip 解压缩失败: {}", e)))?;
                Ok(decompressed)
            },
            Some("br") => {
                debug!("检测到 brotli 压缩，reqwest 应该自动处理");
                // Brotli 通常由 reqwest 自动处理，如果到这里说明可能需要手动处理
                // 但我们暂时返回原始数据，因为 reqwest 通常会处理这种情况
                Ok(bytes.to_vec())
            },
            Some(encoding) => {
                warn!("未知的内容编码: {}", encoding);
                Ok(bytes.to_vec())
            },
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
            reqwest::StatusCode::FORBIDDEN => {
                warn!("🚫 检测到 403 Forbidden 状态码，可能触发了风控机制");
                warn!("💡 建议：等待一段时间后重试，或检查 cookies 是否有效");
                Err(DownloadError::RateLimited(format!(
                    "访问被拒绝 (403 Forbidden)，URL: {}，可能触发了风控机制，建议稍后重试", 
                    url
                )))
            },
            reqwest::StatusCode::TOO_MANY_REQUESTS => {
                warn!("⚠️ 检测到 429 Too Many Requests 状态码，请求过于频繁");
                Err(DownloadError::RateLimited(format!(
                    "请求过于频繁 (429 Too Many Requests)，URL: {}，请降低请求频率", 
                    url
                )))
            },
            reqwest::StatusCode::UNAUTHORIZED => {
                warn!("🔐 检测到 401 Unauthorized 状态码，认证失败");
                Err(DownloadError::RateLimited(format!(
                    "认证失败 (401 Unauthorized)，URL: {}，请检查登录状态", 
                    url
                )))
            },
            status if status.is_success() => {
                debug!("✅ 响应状态正常: {}", status);
                Ok(())
            },
            _ => {
                warn!("❌ 非成功状态码: {}", status);
                Err(DownloadError::InvalidState(format!(
                    "HTTP 请求失败，状态码: {}，URL: {}", 
                    status, url
                )))
            }
        }
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
async fn get_content_info(url: &str) -> Result<DownloadContent, DownloadError> {
    let client = reqwest::Client::new();
    let resp = client
        .head(url)
        .send()
        .await
        .map_err(DownloadError::HttpError)?;

    // 检查状态码
    let status = resp.status();
    debug!("URL: {}", url);
    debug!("Response Status: {}", status);
    
    // 特别处理风控情况
    match status {
        reqwest::StatusCode::FORBIDDEN => {
            warn!("🚫 HEAD 请求遇到 403 Forbidden，可能触发了风控机制");
            return Err(DownloadError::RateLimited(format!(
                "获取内容信息时访问被拒绝 (403 Forbidden)，URL: {}", 
                url
            )));
        },
        reqwest::StatusCode::TOO_MANY_REQUESTS => {
            warn!("⚠️ HEAD 请求遇到 429 Too Many Requests");
            return Err(DownloadError::RateLimited(format!(
                "获取内容信息时请求过于频繁 (429 Too Many Requests)，URL: {}", 
                url
            )));
        },
        _ if !status.is_success() => {
            warn!("❌ HEAD 请求失败，状态码: {}", status);
            return Err(DownloadError::InvalidState(format!(
                "获取内容信息失败，状态码: {}，URL: {}", 
                status, url
            )));
        },
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

async fn get_remote_file_size(url: &str) -> Result<u64, DownloadError> {
    let content_info = get_content_info(url).await?;

    // 检查是否为HTML内容（可能是错误页面）
    if content_info.content_type.contains("text/html") {
        warn!("URL 返回 HTML 内容，可能不是文件下载链接: {}", url);
        return Err(DownloadError::InvalidUrl("URL 返回 HTML 内容".to_string()));
    }

    content_info
        .content_length
        .ok_or(DownloadError::InvalidUrl("无法获取文件大小".to_string()))
}
