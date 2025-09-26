use std::path::PathBuf;

use core::DownloadCore;
use tokio::time::Duration;
use tracing::debug;

use crate::Result;
use crate::common::client::client::BiliClient;
use crate::downloader::models::{DownloadTask, FileType, TaskStatus};

pub mod core;
pub mod error;
pub mod models;

pub struct VideoDownloader {
    download_manager: DownloadCore,
}

impl VideoDownloader {
    pub fn new(concurrent_tasks: usize, state_file: PathBuf, download_client: BiliClient) -> Self {
        Self {
            download_manager: DownloadCore::new(concurrent_tasks, state_file, &download_client),
        }
    }

    pub async fn download(&self, task: &mut Vec<DownloadTask>) -> Result<()> {
        debug!("task: {:?}", task);

        for t in task {
            self.download_file(
                t.url.clone(),
                t.name.clone(),
                t.output_path.clone(),
                t.file_type.clone(),
            )
            .await?;
        }
        Ok(())
    }

    async fn download_file(
        &self,
        url: String,
        name: String,
        output_path: String,
        file_type: FileType,
    ) -> Result<()> {
        crate::common::logger::PrettyLogger::separator();
        crate::common::logger::PrettyLogger::info(format!("开始下载: {}", name));

        let download_file_path = PathBuf::from(output_path);
        // 确保输出目录存在
        if let Some(parent) = download_file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        self.start_download(&url, &download_file_path, &file_type)
            .await?;

        crate::common::logger::PrettyLogger::success(format!("下载完成: {}", download_file_path.display()));
        Ok(())
    }

    async fn start_download(&self, url: &str, path: &PathBuf, file_type: &FileType) -> Result<()> {
        let task_id = self.download_manager.add_task(url, path, file_type).await?;

        loop {
            let status = self.download_manager.get_task_status(&task_id).await;
            if status == Some(TaskStatus::Completed) {
                break;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        match self.download_manager.get_task_status(&task_id).await {
            Some(TaskStatus::Completed) => Ok(()),
            _ => Err("下载失败".into()),
        }
    }
}
