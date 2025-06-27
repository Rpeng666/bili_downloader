use serde::Serialize;
use serde_derive::Deserialize;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct VideoId {
    pub bvid: Option<String>,
    pub aid: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CourseId {
    pub ep_id: Option<String>, // 单集ID
    pub ss_id: Option<String>, // 整季ID
}

#[derive(Debug, Clone, PartialEq)]
pub enum UrlType {
    CommonVideo(VideoId), // 普通视频

    BangumiEpisode(String), // ep_id

    BangumiSeason(String), // season_id

    CourseEpisode(String), // 课程单集 ep_id

    CourseSeason(String), // 课程整季 ss_id

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
            Self::CourseEpisode(_) => "https://www.bilibili.com/cheese/play/ep".to_string(),
            Self::CourseSeason(_) => "https://www.bilibili.com/cheese/play/ss".to_string(),
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
            Self::BangumiEpisode(_) | Self::CourseEpisode(_) | Self::Favorite(_)
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
            Self::CourseEpisode(id) => write!(f, "课程单集 {}", id),
            Self::CourseSeason(id) => write!(f, "课程整季 {}", id),
            Self::LiveRoom(id) => write!(f, "直播间 {}", id),
            Self::Collection(id) => write!(f, "合集 {}", id),
            Self::Favorite(id) => write!(f, "收藏夹 {}", id),
            Self::UgcSeason(id) => write!(f, "UP主合集 {}", id),
            Self::Article(id) => write!(f, "专栏 {}", id),
        }
    }
}

// 视频清晰度选项
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum VideoQuality {
    Q360P = 16,     // 流畅 360P
    Q480P = 32,     // 清晰 480P
    Q720P = 64,     // 高清 720P
    Q720P60 = 74,   // 高清 720P60
    Q1080P = 80,    // 高清 1080P
    Q1080PP = 112,  // 高清 1080P+
    Q1080P60 = 116, // 高清 1080P60
    Q4K = 120,      // 超清 4K
    QHdr = 125,     // HDR 真彩色
    Q8K = 127,      // 超高清 8K
}

impl Default for VideoQuality {
    fn default() -> Self {
        Self::Q1080P // 默认选择 1080P
    }
}

pub enum AudioQuality {
    Quality64k = 30216,
    Quality132k = 30232,
    QualityDolby = 30250,
    QualityHiRES = 30251,
    Quality192k = 30280,
}

// --------------------------------------------------------
