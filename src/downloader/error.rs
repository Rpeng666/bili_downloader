use thiserror::Error;
use std::fmt;

#[derive(Debug)]
pub enum DownloadError {
    HttpError(reqwest::Error),
    IoError(std::io::Error),
    InvalidUrl(String),
    TaskNotFound(String),
    TaskAlreadyExists(String),
    InvalidState(String),
    MergeError(String),
    SemaphoreError,
}

impl fmt::Display for DownloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DownloadError::HttpError(e) => write!(f, "HTTP错误: {}", e),
            DownloadError::IoError(e) => write!(f, "IO错误: {}", e),
            DownloadError::InvalidUrl(url) => write!(f, "无效的URL: {}", url),
            DownloadError::TaskNotFound(id) => write!(f, "任务未找到: {}", id),
            DownloadError::TaskAlreadyExists(id) => write!(f, "任务已存在: {}", id),
            DownloadError::InvalidState(msg) => write!(f, "无效的状态: {}", msg),
            DownloadError::SemaphoreError => write!(f, "信号量错误"),
            DownloadError::MergeError(msg) => write!(f, "合并错误: {}", msg),
        }
    }
}

impl std::error::Error for DownloadError {}

impl From<reqwest::Error> for DownloadError {
    fn from(error: reqwest::Error) -> Self {
        DownloadError::HttpError(error)
    }
}

impl From<std::io::Error> for DownloadError {
    fn from(error: std::io::Error) -> Self {
        DownloadError::IoError(error)
    }
}