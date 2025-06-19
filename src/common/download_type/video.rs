use async_trait::async_trait;
use serde::Deserialize;

use crate::{
    common::models::DownloadTaskTrait,
    downloader::{
        merger,
        models::{DownloadItem, DownloadTask, TaskStatus},
    },
    parser::{detail_parser::parser_trait::StreamType, errors::ParseError},
};

#[derive(Debug, Default, Clone, Deserialize)]
pub struct VideoInfo {
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

    pub stream_type: StreamType, // 流类型，DASH或MP4

    // 流信息
    pub video_url: Option<String>, // 可选的视频流地址
    pub audio_url: Option<String>, // 可选的音频流地址
    pub mp4_url: Option<String>,   // 可选的MP4流地址
}

#[async_trait]
impl DownloadTaskTrait for VideoInfo {
    async fn to_download_task(&self) -> Result<DownloadTask, ParseError> {
        // 将VideoInfo转换为下载任务
        let mut items = vec![];

        if let Some(video_url) = &self.video_url {
            items.push(DownloadItem::Video {
                url: video_url.clone(),
                name: self.title.clone(),
                desc: self.desc.clone(),
                status: TaskStatus::Queued,
                output_path: format!("./tmp/{}/{}.mp4", self.title, self.title),
            });
        }

        if let Some(audio_url) = &self.audio_url {
            items.push(DownloadItem::Audio {
                url: audio_url.clone(),
                name: self.title.clone(),
                desc: self.desc.clone(),
                status: TaskStatus::Queued,
                output_path: format!("./tmp/{}/{}.mp4", self.title, self.title),
            });
        }

        if let Some(mp4_url) = &self.mp4_url {
            items.push(DownloadItem::MP4 {
                url: mp4_url.clone(),
                name: self.title.clone(),
                desc: self.desc.clone(),
                status: TaskStatus::Queued,
                output_path: format!("./tmp/{}/{}.mp4", self.title, self.title),
            });
        }

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

#[derive(Debug, Default, Clone, Deserialize)]
pub struct VideoInfoVec(pub Vec<VideoInfo>);
impl VideoInfoVec {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn push(&mut self, video_info: VideoInfo) {
        self.0.push(video_info);
    }
}

#[async_trait]
impl DownloadTaskTrait for VideoInfoVec {
    async fn to_download_task(&self) -> Result<DownloadTask, ParseError> {
        // 将VideoInfoVec转换为下载任务
        let mut items = Vec::new();
        for episode in &self.0 {
            items.push(episode.to_download_task().await?);
        }

        Ok(DownloadTask {
            title: format!("Season {:?}", self.0),
            items: items.into_iter().flat_map(|task| task.items).collect(),
        })
    }

    async fn post_handle_download_task(&self, task: &DownloadTask) -> Result<(), ParseError> {
        // 对每个视频进行后处理
        let mut futures = Vec::new();
        for item in &task.items {
            if let DownloadItem::Video { .. } = item {
                // 处理视频下载完成后的合并
                let video_info = self.0.iter().find(|ep| {
                    if let DownloadItem::Video { name, .. } = item {
                        ep.title == *name
                    } else {
                        false
                    }
                });

                if let Some(video_info) = video_info {
                    futures.push(video_info.post_handle_download_task(task));
                }
            }
        }
        Ok(())
    }
}
