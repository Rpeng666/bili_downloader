use futures::StreamExt;
use futures_util::stream::TryStreamExt;
use indicatif::ProgressBar;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::common::client::client::BiliClient;
use crate::downloader::models::{DownloadProgress, TaskStatus};

use super::error::DownloadError;
use dashmap::DashMap;
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
    pub async fn add_task(&self, url: &str, output: &Path) -> Result<String, DownloadError> {
        let task_id = uuid::Uuid::new_v4().to_string();
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|_| DownloadError::SemaphoreError)?;
        let size = get_remote_file_size(url).await?;

        debug!("开始添加下载任务: {}", task_id);

        let task = DownloadProgress {
            task_id: task_id.clone(),
            url: url.to_string(),
            output_path: output.to_path_buf(),
            total_size: size,
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
        // 启动下载任务
        tokio::spawn(async move {
            Self::run(tasks, task_id_clone.clone(), download_client).await;
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
                // task_guard.total_size,
                task_guard.url.clone(),
            )
        };

        info!("开始下载任务: {}", task_id);

        // 更新任务状态为下载中
        {
            let mut task_guard = task_lock.lock().await;
            task_guard.status = TaskStatus::Downloading;
        }

        let response = download_client
            .get_raw_response(url.as_str())
            .await
            .unwrap();

        let total_size = response
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .ok_or_else(|| DownloadError::InvalidState("无法获取文件大小".to_string()))
            .unwrap()
            .to_str();

        // 创建进度条
        let pb = ProgressBar::new(total_size.unwrap().parse::<u64>().unwrap());
        pb.set_style(
            indicatif::ProgressStyle::with_template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})",
                )
                .unwrap()
                .progress_chars("#>-")
        );

        debug!("开始下载文件: {}", url);

        let mut file = tokio::fs::File::create(&output_path).await.unwrap();
        let mut stream = response.bytes_stream().map_err(std::io::Error::other);
        // let mut stream_reader = StreamReader::new(stream);

        // 下载并显示进度
        let mut downloaded = 0u64;
        while let Some(item) = stream.next().await {
            let chunk = match item {
                Ok(chunk) => chunk,
                Err(error) => {
                    error!("下载任务失败: {}", error);
                    pb.finish_with_message("下载失败");
                    panic!("下载任务 {} 失败，无法获取数据流", task_id);
                }
            };

            tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
                .await
                .unwrap();
            downloaded += chunk.len() as u64;
            pb.set_position(downloaded);
        }

        // 更新任务状态为完成
        {
            let mut task_guard = task_lock.lock().await;
            task_guard.status = TaskStatus::Completed;
        }

        pb.finish_with_message("下载完成");
    }
}

async fn get_remote_file_size(url: &str) -> Result<u64, DownloadError> {
    let client = reqwest::Client::new();
    let resp = client.head(url).send().await?;

    // 打印响应头
    debug!("Url: {}", url);
    debug!("Response Status: {}", resp.status());
    debug!("Response Headers: {:?}", resp.headers());

    resp.headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|ct_len| ct_len.to_str().ok())
        .and_then(|ct_len| ct_len.parse().ok())
        .ok_or(DownloadError::InvalidUrl("无法获取文件大小".to_string()))
}
