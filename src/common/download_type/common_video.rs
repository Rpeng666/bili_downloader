use crate::parser::detail_parser::{models::DownloadConfig, parser_trait::StreamType};

// ---------------------------------------------------------------------------
#[derive(Debug, Default, Clone)]
pub struct VideoInfo {
    pub danmaku_url: Option<String>,
    pub subtitle_uris: Option<Vec<String>>,
    // 基础标识
    pub url: String,
    pub aid: i64,
    pub bvid: String,
    pub cid: i64,
    //视频元数据
    pub title: String,
    pub cover: String,
    pub desc: String,
    pub views: String,
    pub danmakus: String,
    // UP主信息
    pub up_name: String,
    pub up_mid: i64,
    pub video_quality_id_list: Vec<i32>,
    pub strean_type: StreamType,
    //流信息
    pub video_url: Option<String>,
    //可选的视频流地址
    pub audio_url: Option<String>, //可选的音频流地址
    pub mp4_url: Option<String>,
}

// ---------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct VideoInfoVec {
    pub config: DownloadConfig,
    pub videos: Vec<VideoInfo>,
}

impl VideoInfoVec {
    pub fn new(config: DownloadConfig) -> Self {
        Self {
            config: config,
            videos: Vec::new(),
        }
    }

    pub fn push(&mut self, video: VideoInfo) {
        self.videos.push(video);
    }
}
