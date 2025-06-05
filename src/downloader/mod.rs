use std::path::PathBuf;

use core::DownloadCore;
use merger::MediaMerger;
use task::TaskStatus;
use tokio::time::Duration;

use crate::Result;
use crate::common::api::client::BiliClient;
use crate::parser::models::StreamType;

pub mod core;
pub mod error;
pub mod merger;
pub mod task;

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

    pub async fn download(&self, video_info: &crate::parser::models::VideoInfo) -> Result<()> {
        match video_info.stream_type {
            StreamType::Dash => self.download_dash(video_info).await,
            StreamType::Flv => self.download_flv(video_info).await,
        }
    }

    async fn download_dash(&self, video_info: &crate::parser::models::VideoInfo) -> Result<()> {
        println!("开始下载 ... ");

        // 创建临时目录
        let tmp_dir = self.output_dir.join("tmp");
        tokio::fs::create_dir_all(&tmp_dir).await?;
        let output_dir = self.output_dir.join("output");
        tokio::fs::create_dir_all(&output_dir).await?;

        // 下载视频流
        println!("------------------------------------------------------");
        // println!("开始下载视频流: {}", video_info.video_url);
        println!("开始下载视频流");
        let video_path = self
            .output_dir
            .join(format!("tmp/{}_video.mp4", video_info.bvid));
        self.download_file(&video_info.video_url, &video_path)
            .await?;

        // 下载音频流
        println!("------------------------------------------------------");
        // println!("开始下载音频流: {}", video_info.audio_url);
        println!("开始下载音频流");
        let audio_path = self
            .output_dir
            .join(format!("tmp/{}_audio.m4a", video_info.bvid));
        self.download_file(&video_info.audio_url, &audio_path)
            .await?;

        // 合并视频和音频
        println!("------------------------------------------------------");
        println!("开始合并视频和音频...");
        let output_path = self
            .output_dir
            .join(format!("output/{}.mp4", video_info.title));
        let merger = MediaMerger;
        merger
            .merge_av(&video_path, &audio_path, &output_path)
            .await?;

        println!("下载完成！");
        Ok(())
    }

    async fn download_flv(&self, video_info: &crate::parser::models::VideoInfo) -> Result<()> {
        println!("开始下载 FLV 视频...");
        let output_path = self.output_dir.join(format!("{}.flv", video_info.title));
        self.download_file(&video_info.video_url, &output_path)
            .await?;
        println!("下载完成！");
        Ok(())
    }

    async fn download_file(&self, url: &str, path: &PathBuf) -> Result<()> {
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
