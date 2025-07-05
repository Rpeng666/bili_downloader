use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default)]
pub struct DownloadTask {
    pub url: String,
    pub file_type: FileType,
    pub name: String,
    pub output_path: String,
    pub temp_path: String,
    pub metadata: HashMap<String, String>,
}

impl DownloadTask {
    pub fn new(
        url: String,
        file_type: FileType,
        name: String,
        output_path: String,
        temp_path: String,
        metadata: HashMap<String, String>,
    ) -> Self {
        Self {
            url,
            file_type,
            name,
            output_path,
            temp_path,
            metadata,
        }
    }

    pub fn get_output_path(&self) -> PathBuf {
        PathBuf::from(&self.output_path)
    }
    pub fn get_temp_path(&self) -> PathBuf {
        PathBuf::from(&self.temp_path)
    }
}

// --------------------------------------------------------------------
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode, PartialEq, Eq)]
pub enum TaskStatus {
    Queued,
    Downloading,
    Completed,
    Failed,
    Error(String),
    Skipped(String), // 跳过任务，包含跳过原因
}

pub struct DownloadProgress {
    pub task_id: String,
    pub url: String,
    pub output_path: PathBuf,
    pub total_size: u64,
    pub downloaded: u64,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode, PartialEq, Eq, Default)]
pub enum FileType {
    #[default]
    Video,
    Audio,
    Danmaku,
    Subtitle,
    Image,
    Other(String), // 其他类型
}
