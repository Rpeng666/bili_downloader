use async_trait::async_trait;
use serde::Deserialize;

use crate::{
    common::models::DownloadTaskTrait, downloader::models::DownloadTask, parser::errors::ParseError
};

#[derive(Debug, Default, Clone, Deserialize)]
pub struct Mp4VideoInfo {
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

    // 视频流信息
    pub video_url: String,
}

#[async_trait]
impl DownloadTaskTrait for Mp4VideoInfo {
    async fn to_download_task(&self) -> Result<DownloadTask, ParseError> {
        Err(ParseError::ApiError("xxx".to_string()))
    }

    async fn post_handle_download_task(&self, task: &DownloadTask) -> Result<(), ParseError> {
        // 处理下载任务完成的后处理任务（比如音频和视频合并）
        Err(ParseError::ApiError(
            "Post handle not implemented".to_string(),
        ))
    }
}
