use async_trait::async_trait;
use serde::Deserialize;

use crate::{
    common::models::DownloadTaskTrait,
    downloader::{
        merger,
        models::{DownloadItem, DownloadTask, TaskStatus},
    },
    parser::errors::ParseError,
};

#[derive(Debug, Default, Clone, Deserialize)]
pub struct DashVideoInfo {
    // 基础标识
    pub url: String,
    pub aid: i64,
    pub bvid: String,
    pub cid: i64,

    // 视频元数据
    pub title: String,
    pub cover: String,
    pub desc: String,
    pub views: String,
    pub danmakus: String,

    // UP主信息
    pub up_name: String,
    pub up_mid: i64,

    pub video_quality_id_list: Vec<i32>,

    // 流信息
    pub video_url: String,
    pub audio_url: String,
}

#[async_trait]
impl DownloadTaskTrait for DashVideoInfo {
    async fn to_download_task(&self) -> Result<DownloadTask, ParseError> {
        // 将DashVideoInfo转换为下载任务
        let items = vec![
            DownloadItem::Video {
                url: self.video_url.clone(),
                name: format!("{}", self.title),
                desc: self.desc.clone(),
                output_path: format!("{}/{}.mp4", self.title, self.title),
                status: TaskStatus::Queued,
            },
            DownloadItem::Audio {
                url: self.audio_url.clone(),
                name: format!("{}", self.title),
                output_path: format!("{}/{}.mp3", self.title, self.title),
                desc: self.desc.clone(),
                status: TaskStatus::Queued,
            },
        ];

        Ok(DownloadTask {
            title: self.title.clone(),
            items,
        })
    }

    async fn post_handle_download_task(&self, task: &DownloadTask) -> Result<(), ParseError> {
        // 处理下载任务完成的后处理任务（比如音频和视频合并）

        // 先找出TaskStatus为Completed的任务
        let video_item = task
            .items
            .iter()
            .find(|item| {
                if let DownloadItem::Video { status, .. } = item {
                    *status == TaskStatus::Completed
                } else {
                    false
                }
            })
            .ok_or_else(|| ParseError::ApiError("Video item not found".to_string()))?;

        let video_output_path = if let DownloadItem::Video { output_path, .. } = video_item {
            output_path
        } else {
            return Err(ParseError::ApiError("Video item not found".to_string()));
        };

        let audio_item = task
            .items
            .iter()
            .find(|item| {
                if let DownloadItem::Audio { status, .. } = item {
                    *status == TaskStatus::Completed
                } else {
                    false
                }
            })
            .ok_or_else(|| ParseError::ApiError("Audio item not found".to_string()))?;

        let audio_output_path = if let DownloadItem::Audio { output_path, .. } = audio_item {
            output_path
        } else {
            return Err(ParseError::ApiError("Audio item not found".to_string()));
        };

        use std::path::Path;

        // 合并
        merger::MediaMerger
            .merge_av(
                Path::new(video_output_path),
                Path::new(audio_output_path),
                Path::new(&format!("output/{}.mp4", self.title)),
            )
            .await
            .map_err(|e| ParseError::ApiError(e.to_string()))?;

        Ok(())
    }
}
