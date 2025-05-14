use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use bincode::{Encode, Decode};

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct DownloadTask {
    pub task_id: String,
    pub url: String,
    pub output_path: PathBuf,
    pub total_size: u64,
    pub downloaded: u64,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode, PartialEq)]
pub enum TaskStatus {
    Queued,
    Downloading,
    Paused,
    Completed,
    Error(String),
}