use serde_derive::Deserialize;
use std::fmt;

use crate::{
    common::models::DownloadType
};

#[derive(Debug, Clone, PartialEq)]
pub struct VideoId {
    pub bvid: Option<String>,
    pub aid: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UrlType {
    CommonVideo(VideoId), // 普通视频

    BangumiEpisode(String), // ep_id

    BangumiSeason(String), // season_id

    CourseChapterDash(String), // chapter_id

    LiveRoom(String), // room_id

    Collection(String), // 合集id

    Favorite(String), // 收藏夹id

    UgcSeason(String), // UP主合集

    Article(String), // 专栏
}

impl UrlType {
    pub fn base_url(&self) -> String {
        match self {
            Self::CommonVideo(_) => "https://www.bilibili.com/video/".to_string(),
            Self::BangumiEpisode(_) => "https://www.bilibili.com/bangumi/play/ep".to_string(),
            Self::BangumiSeason(_) => "https://www.bilibili.com/bangumi/play/ss".to_string(),
            Self::CourseChapterDash(_) => "https://www.bilibili.com/cheese/play/".to_string(),
            Self::LiveRoom(_) => "https://live.bilibili.com/".to_string(),
            Self::Collection(_) => "https://www.bilibili.com/medialist/detail/ml".to_string(),
            Self::Favorite(_) => "https://www.bilibili.com/favlist/".to_string(),
            Self::UgcSeason(_) => "https://www.bilibili.com/medialist/play/ml".to_string(),
            Self::Article(_) => "https://www.bilibili.com/read/cv".to_string(),
        }
    }

    pub fn need_login(&self) -> bool {
        matches!(
            self,
            Self::BangumiEpisode(_) | Self::CourseChapterDash(_) | Self::Favorite(_)
        )
    }
}

impl fmt::Display for UrlType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CommonVideo(id) => {
                if let Some(bvid) = &id.bvid {
                    write!(f, "普通视频 BV{}", bvid)
                } else if let Some(aid) = &id.aid {
                    write!(f, "普通视频 av{}", aid)
                } else {
                    write!(f, "普通视频(未知ID)")
                }
            }
            Self::BangumiEpisode(id) => write!(f, "番剧 EP{}", id),
            Self::BangumiSeason(id) => write!(f, "番剧季 {}", id),
            Self::CourseChapterDash(id) => write!(f, "课程章节 {}", id),
            Self::LiveRoom(id) => write!(f, "直播间 {}", id),
            Self::Collection(id) => write!(f, "合集 {}", id),
            Self::Favorite(id) => write!(f, "收藏夹 {}", id),
            Self::UgcSeason(id) => write!(f, "UP主合集 {}", id),
            Self::Article(id) => write!(f, "专栏 {}", id),
        }
    }
}

// 需要下载数据的元数据
#[derive(Debug, Clone)]
pub struct ParsedMeta {
    // 解析出来的一些通用的信息
    pub title: String, // 视频标题
    pub stream_type: StreamType,
    // 枚举不同的要下载的类型
    pub meta: DownloadType,
}



#[derive(Debug, Deserialize, Clone)]
pub enum StreamType {
    Dash, // DASH流
    MP4,  // MP4流
}

impl Default for StreamType {
    fn default() -> Self {
        StreamType::Dash
    }
}

// 画质选项
#[derive(Debug, Deserialize)]
pub struct QualityOption {
    pub codecid: u8,
    pub quality: u64,
    pub format: String,
    pub description: String,
}



pub enum VideoQuality {
    Quality360p = 16,
    Quality480p = 32,
    Quality720p = 64,
    Quality1080p = 80,
    Quality1080pPLUS = 112,
    Quality1080p60 = 116,
    Quality4k = 120,
    QualityHdr = 125,
    QualityDolby = 126,
    Quality8k = 127,
}

pub enum AudioQuality {
    Quality64k = 30216,
    Quality132k = 30232,
    QualityDolby = 30250,
    QualityHiRES = 30251,
    Quality192k = 30280,
}

// --------------------------------------------------------

// 分集信息结构体
#[derive(Debug, Clone)]
struct EpisodeItem {
    title: String,
    cid: i64,
    badge: String,
    duration: String,
}

// 视频类型枚举
#[derive(Debug, Clone, PartialEq)]
enum VideoType {
    Single,
    Part,
    Collection,
}

// 显示模式枚举
#[derive(Debug, Clone, PartialEq)]
enum EpisodeDisplayType {
    Single,
    InSection,
    All,
}

impl From<i32> for EpisodeDisplayType {
    fn from(value: i32) -> Self {
        match value {
            0 => EpisodeDisplayType::Single,
            1 => EpisodeDisplayType::InSection,
            2 => EpisodeDisplayType::All,
            _ => EpisodeDisplayType::Single,
        }
    }
}
