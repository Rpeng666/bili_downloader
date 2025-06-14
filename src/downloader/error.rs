use std::{fmt, path::PathBuf};

#[derive(Debug)]
pub enum DownloadError {
    /// HTTP错误
    HttpError(reqwest::Error),
    /// IO错误
    IoError(std::io::Error),
    /// 无效的URL错误
    InvalidUrl(String),
    /// 任务未找到错误
    TaskNotFound(String),
    /// 任务已存在错误
    TaskAlreadyExists(String),
    /// 无效状态错误
    InvalidState(String),
    /// 合并错误
    MergeError(String),
    /// 文件未找到错误
    FileNotFound(PathBuf),
    /// 流错误
    StreamError(String),
    /// ffmpeg执行错误
    FfmpegError(String),
    /// ffmpeg未找到
    FfmpegNotFound,
    /// 信号量错误
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
            DownloadError::FileNotFound(path) => write!(f, "文件未找到: {}", path.display()),
            DownloadError::FfmpegError(msg) => write!(f, "ffmpeg错误: {}", msg),
            DownloadError::FfmpegNotFound => write!(f, "ffmpeg未找到，请安装ffmpeg"),
            DownloadError::MergeError(msg) => write!(f, "合并错误: {}", msg),
            DownloadError::StreamError(msg) => write!(f, "流错误: {}", msg),
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
