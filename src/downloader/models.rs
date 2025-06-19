use std::path::{Path, PathBuf};

use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct DownloadTask {
    pub title: String,
    pub items: Vec<DownloadItem>,
}

#[derive(Clone, Debug)]
pub enum DownloadItem {
    Video {
        url: String,
        name: String,
        desc: String,
        status: TaskStatus,
        output_path: String, // 输出路径
    },
    Audio {
        url: String,
        name: String,
        desc: String,
        status: TaskStatus,
        output_path: String, // 输出路径
    },
    MP4 {
        url: String,
        name: String,
        desc: String,
        status: TaskStatus,
        output_path: String, // 输出路径
    },
    Danmaku {
        cid: u64,
        name: String,
        desc: String,
        status: TaskStatus,
    },
    Subtitle {
        url: String,
        lang: String,
        desc: String,
        status: TaskStatus,
    },
    CoverImage {
        url: String,
        name: String,
        desc: String,
        status: TaskStatus,
    },
    // ...
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode, PartialEq)]
pub enum TaskStatus {
    Queued,

    Downloading,

    Paused,

    Completed,

    Error(String),
}

pub struct DownloadProgress {
    pub task_id: String,
    pub status: TaskStatus,
    pub url: String,
    pub output_path: PathBuf,
    pub downloaded: u64,
    pub total_size: u64,
}
