use std::path::PathBuf;

use core::DownloadCore;
use tokio::time::Duration;
use tracing::{debug, info};

use crate::Result;
use crate::common::client::client::BiliClient;
use crate::downloader::models::{DownloadItem, DownloadTask, TaskStatus};

pub mod core;
pub mod error;
pub mod merger;
pub mod models;

// 定义一个特征来获取视频信息
pub trait HasVideoInfo {
    fn get_bvid(&self) -> &str;
    fn get_title(&self) -> &str;
    fn get_video_url(&self) -> &str;
    fn get_audio_url(&self) -> &str;
}

pub struct VideoDownloader {
    download_manager: DownloadCore,
    output_dir: PathBuf,
}

impl VideoDownloader {
    pub fn new(
        concurrent_tasks: usize,
        state_file: PathBuf,
        output_dir: PathBuf,
        download_client: BiliClient,
    ) -> Self {
        Self {
            download_manager: DownloadCore::new(concurrent_tasks, state_file, &download_client),
            output_dir,
        }
    }

    pub async fn download(&self, task: &mut DownloadTask) -> Result<()> {
        debug!("task: {:?}", task);

        for task_item in &mut task.items {
            match task_item {
                DownloadItem::Video {
                    url,
                    name,
                    desc,
                    status,
                    output_path,
                } => {
                    self.download_file(
                        url.clone(),
                        name.clone(),
                        desc.clone(),
                        output_path.clone(),
                    )
                    .await?;
                    *status = TaskStatus::Completed;
                }
                DownloadItem::Audio {
                    url,
                    name,
                    desc,
                    status,
                    output_path,
                } => {
                    self.download_file(
                        url.clone(),
                        name.clone(),
                        desc.clone(),
                        output_path.clone(),
                    )
                    .await?;
                    *status = TaskStatus::Completed;
                }
                _ => {
                    return Err("不支持的下载项类型".into());
                }
            }
        }
        Ok(())
    }

    async fn download_file(
        &self,
        url: String,
        name: String,
        desc: String,
        output_path: String,
    ) -> Result<()> {
        info!("------------------------------------------------------");
        info!("开始下载 {} {} ... ", name, desc);

        let download_file_path = self.output_dir.join(output_path);
        // 确保输出目录存在
        if let Some(parent) = download_file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        self.start_download(&url, &download_file_path).await?;

        info!("下载完成！ {}", name);
        Ok(())
    }

    async fn start_download(&self, url: &str, path: &PathBuf) -> Result<()> {
        let task_id = self.download_manager.add_task(url, path).await?;

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
