use serde_derive::Deserialize;

use crate::parser::models::VideoQuality;

#[derive(Debug, Deserialize, Clone)]
pub struct DownloadConfig {
    pub resolution: VideoQuality,      // 分辨率
    pub need_video: bool,              // 是否需要视频
    pub need_audio: bool,              // 是否需要音频
    pub need_danmaku: bool,            // 是否需要弹幕
    pub need_subtitle: bool,           // 是否需要字幕
    pub merge: bool,                   // 是否需要合并音视频
    pub output_format: String,         // 输出格式
    pub output_dir: String,            // 输出目录
    pub concurrency: usize,            // 并发数
    pub episode_range: Option<String>, // 集数范围
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            resolution: VideoQuality::default(),
            need_video: true,
            need_audio: true,
            need_danmaku: true,
            need_subtitle: true,
            merge: true,
            output_format: "mp4".to_string(),
            output_dir: "./downloads".to_string(),
            concurrency: 4,
            episode_range: None,
        }
    }
}
