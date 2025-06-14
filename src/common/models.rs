// 由于不同类型（直播间，普通视频，番剧，弹幕，课程）的解析出来的struct千差万别，如果再考虑上Dash流和MP4流，情况则更多
// downloader模块内部需要分别对这些情况处理，会充斥大量if else match，造成模型失血，逻辑不清
// 故不在考虑面向URL类型建模，而是细粒度分解，面向下载类型建模（Video，Audio.....)

use async_trait::async_trait;
use serde::Deserialize;

use crate::{
    common::download_type::{dash::DashVideoInfo, mp4::Mp4VideoInfo},
    downloader::models::DownloadTask,
    parser::errors::ParseError,
};

#[async_trait]
pub trait DownloadTaskTrait {
    // 将当前类型转换为下载任务
    async fn to_download_task(&self) -> Result<DownloadTask, ParseError>;
    // 处理下载任务完成的后处理任务（比如音频和视频合并）
    async fn post_handle_download_task(&self, task: &DownloadTask) -> Result<(), ParseError>;
}

// 枚举不同的要下载的类型
#[derive(Debug, Deserialize, Clone)]
pub enum DownloadType {
    CommonVideo(DashVideoInfo),       // 普通视频(dash流)
    BangumiEpisode(DashVideoInfo),    // 番剧EP(dash流)
    BangumiSeason(DashVideoInfo),     // 番剧季(dash流)
    CourseChapterDash(DashVideoInfo), // 课程章节(dash流)
    CourseChapterMp4(Mp4VideoInfo),   // 课程章节(Mp4流)
                                      // LiveRoom(LiveRoomInfo),      // 直播间
                                      // Collection(CollectionInfo),  // 合集
                                      // Favorite(FavoriteInfo),      // 收藏夹
                                      // UgcSeason(UgcSeasonInfo),    // UP主合集
                                      // Article(ArticleInfo),        // 专栏
}

impl DownloadType {
    pub async fn to_download_task(&self) -> Result<DownloadTask, ParseError> {
        match self {
            Self::CommonVideo(v) => return v.to_download_task().await,
            Self::BangumiEpisode(v) => return v.to_download_task().await,
            Self::BangumiSeason(v) => return v.to_download_task().await,
            Self::CourseChapterDash(v) => return v.to_download_task().await,
            Self::CourseChapterMp4(v) => return v.to_download_task().await,
        };
        Err(ParseError::ParseError(format!(
            "find type {:?} not implemented",
            self
        )))
    }

    pub async fn post_handle_download_task(&self, task: &DownloadTask) -> Result<(), ParseError> {
        match self {
            Self::CommonVideo(v) => return v.post_handle_download_task(task).await,
            Self::BangumiEpisode(v) => return v.post_handle_download_task(task).await,
            Self::BangumiSeason(v) => return v.post_handle_download_task(task).await,
            Self::CourseChapterDash(v) => return v.post_handle_download_task(task).await,
            Self::CourseChapterMp4(v) => return v.post_handle_download_task(task).await,
        }

        Err(ParseError::ParseError(format!(
            "post handle type {:?} not implemented",
            self
        )))
    }
}
