use std::path::{Path, PathBuf};
use futures::task;
use futures::StreamExt;
use indicatif::ProgressBar;
use bincode;
use std::sync::Arc;

use super::{error::DownloadError, task::{DownloadTask, TaskStatus}};
use dashmap::DashMap;
use tokio::sync::{Mutex, Semaphore};


#[derive(Clone)]
pub struct DownloadManager{
    tasks: Arc<Mutex<DashMap<String, Arc<Mutex<DownloadTask>>>>>, // task_id -> Task
    state_file: PathBuf,
    semaphore: Arc<Semaphore>, // 控制并发数
}

impl DownloadManager {
    pub fn new(max_concurrent: usize, state_file: impl AsRef<Path>) -> Self {
        let state_file = state_file.as_ref().to_path_buf();
        Self { 
            tasks: Arc::new(Mutex::new(DashMap::new())), 
            state_file: state_file, 
            semaphore: Arc::new(Semaphore::new(max_concurrent)) 
        }
    }

    // // 从磁盘加载未完成的任务
    // pub async fn load_previous_state(&self) -> Result<(), DownloadError> {
    //     let data = tokio::fs::read(&self.state_file).await?;
    //     let tasks: Vec<DownloadTask> = bincode::decode_from_slice(&data, bincode::config::standard())
    //         .map_err(|e| DownloadError::InvalidState(e.to_string()))?
    //         .0;

    //     for task in tasks {
    //         if task.status != TaskStatus::Completed {
    //             self.tasks.lock().await.insert(task.task_id.clone(), task);
    //         }
    //     }

    //     Ok(())
    // }

    // 添加新的下载任务
    pub async fn add_task(&self, url: &str, output: &Path) -> Result<String, DownloadError> {
        let task_id = uuid::Uuid::new_v4().to_string();
        let _permit = self.semaphore.acquire().await.map_err(|_| DownloadError::SemaphoreError)?;
        let size = get_remote_file_size(url).await?;

        let task =DownloadTask {
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
        // 启动下载任务
        tokio::spawn(async move {
            Self::run(tasks, task_id_clone.clone()).await;
        });
        
        Ok(task_id)
    }

    // // 暂停和恢复
    // pub async fn pause(&self, task_id: &str) {
    //     if let Some(mut task) = self.tasks.lock().await.get_mut(task_id) {
    //         task.status = TaskStatus::Paused;
    //     }
    // }

    // // 恢复下载任务
    // pub async fn resume(&self, task_id: &str) -> Result<(), DownloadError> {
    //     if let Some(mut task) = self.tasks.lock().await.get_mut(task_id) {
    //         if task.status == TaskStatus::Paused {
    //             task.status = TaskStatus::Queued;
    //             Self::run(Arc::clone(&self.tasks), task_id.to_string()).await;
    //         }
    //     }
    //     Ok(())
    // }

    // // 保存当前状态到磁盘
    // pub async fn save_state(&self) -> Result<(), DownloadError> {
    //     let tasks: Vec<DownloadTask> = self.tasks.lock().await.iter().map(|r| r.value().clone()).collect();
    //     let data = bincode::encode_to_vec(&tasks, bincode::config::standard())
    //         .map_err(|e| DownloadError::InvalidState(e.to_string()))?;

    //     tokio::fs::write(&self.state_file, data).await?;
    //     Ok(())
    // }

    // 获取任务状态
    pub async fn get_task_status(&self, task_id: &str) -> Option<TaskStatus> {
        let task = {
            let tasks = self.tasks.lock().await;
            let task = tasks.get(task_id).expect("任务不存在");
            Arc::clone(&task)
        };

        Some(task.lock()
            .await
            .status
            .clone())
    }

    async fn run(tasks: Arc<Mutex<DashMap<String, Arc<Mutex<DownloadTask>>>>>, task_id: String) {
        let task_lock = {
            let tasks = tasks.lock().await;
            let task = tasks.get(&task_id)
                .expect("任务不存在");
            Arc::clone(&task)
        };

        // 一次性获取所需数据
        let (task_id, output_path, total_size, url) = {
            let task_guard = task_lock.lock().await;
            (
                task_guard.task_id.clone(),
                task_guard.output_path.clone(),
                task_guard.total_size,
                task_guard.url.clone()
            )
        };

        println!("开始下载任务: {}", task_id);

        // 创建进度条
        let pb = ProgressBar::new(total_size);
        pb.set_style(
            indicatif::ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap()
                .progress_chars("#>-")
        );

        // 更新任务状态为下载中
        {
            let mut task_guard = task_lock.lock().await;
            task_guard.status = TaskStatus::Downloading;
        }
        pb.set_position(0);
        pb.set_length(total_size);
        pb.set_message("正在下载...");

        // 直接下载文件
        let client = reqwest::Client::new();
        let response = client.get(url)
            .send()
            .await
            .unwrap();

        let mut file = tokio::fs::File::create(&output_path).await.unwrap();
        let mut stream = response.bytes_stream();

        // 下载并显示进度
        let mut downloaded = 0u64;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.unwrap();
            tokio::io::AsyncWriteExt::write_all(&mut file, &chunk).await.unwrap();
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
    println!("Response Headers: {:?}", resp.headers());
    
    resp.headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|ct_len| ct_len.to_str().ok())
        .and_then(|ct_len| ct_len.parse().ok())
        .ok_or(DownloadError::InvalidUrl("无法获取文件大小".to_string()))
}