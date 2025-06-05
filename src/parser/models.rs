use serde_derive::Deserialize;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum VideoType {
    CommonVideo(String),    // bvid
    BangumiEpisode(String), // ep_id
    BangumiSeason(String),  // season_id
    CourseChapter(String),  // chapter_id
    LiveRoom(String),       // room_id
}

impl fmt::Display for VideoType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CommonVideo(id) => write!(f, "普通视频BV {}", id),
            Self::BangumiEpisode(id) => write!(f, "番剧EP {}", id),
            Self::BangumiSeason(id) => write!(f, "番剧季 {}", id),
            Self::CourseChapter(id) => write!(f, "课程章节 {}", id),
            Self::LiveRoom(id) => write!(f, "直播间 {}", id),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub enum StreamType {
    Dash, // DASH流
    Flv,  // FLV流
}

impl Default for StreamType {
    fn default() -> Self {
        StreamType::Dash
    }
}

// 视频元数据
#[derive(Debug, Deserialize)]
pub struct VideoMeta {
    pub title: String,
    pub duration: u32,
    pub segments: Vec<VideoSegment>,
    pub quality_options: Vec<QualityOption>,
}

// 分集信息
#[derive(Debug, Deserialize)]
pub struct VideoSegment {
    pub id: String,
    pub title: String,
    pub cid: u64,
}

// 画质选项
#[derive(Debug, Deserialize)]
pub struct QualityOption {
    pub codecid: u8,
    pub quality: u16,
    pub format: String,
    pub description: String,
}

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

    // pub pages_list: Vec<VideoPage>,

    // 分P信息
    // pub pages_list: Vec<VideoPage>,
    pub video_quality_id_list: Vec<i32>,
    pub video_quality_desc_list: Vec<String>,

    // 流信息
    pub stream_type: StreamType,
    pub video_url: String,
    pub audio_url: String,
}

pub enum ParseType {
    Video,
    Bangumi,
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
