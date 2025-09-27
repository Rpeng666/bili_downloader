use crate::downloader::models::{DownloadTask, FileType};
use std::collections::HashMap;

/// 创建视频下载任务的辅助函数
pub fn create_video_task(
    url: String,
    title: &str,
    episode_title: Option<&str>,
    output_dir: &str,
    metadata: HashMap<String, String>,
) -> DownloadTask {
    let filename = if let Some(ep_title) = episode_title {
        format!("{} - {}.mp4", title, ep_title)
    } else {
        format!("{}.mp4", title)
    };

    let output_path = format!("{}/{}", output_dir, filename);

    DownloadTask::new(
        url,
        FileType::Video,
        filename,
        output_path,
        output_dir.to_string(),
        metadata,
    )
}

/// 创建音频下载任务的辅助函数
pub fn create_audio_task(
    url: String,
    title: &str,
    episode_title: Option<&str>,
    output_dir: &str,
    metadata: HashMap<String, String>,
) -> DownloadTask {
    let filename = if let Some(ep_title) = episode_title {
        format!("{} - {}.m4s", title, ep_title)
    } else {
        format!("{}.m4s", title)
    };

    let output_path = format!("{}/{}", output_dir, filename);

    DownloadTask::new(
        url,
        FileType::Audio,
        filename,
        output_path,
        output_dir.to_string(),
        metadata,
    )
}

/// 创建弹幕下载任务的辅助函数
pub fn create_danmaku_task(
    url: String,
    title: &str,
    output_dir: &str,
    cid: i64,
    metadata: HashMap<String, String>,
) -> DownloadTask {
    let filename = format!("{}.xml", title);
    let output_path = format!("{}/{}", output_dir, filename);

    DownloadTask::new(
        url,
        FileType::Danmaku,
        filename,
        output_path,
        cid.to_string(),
        metadata,
    )
}