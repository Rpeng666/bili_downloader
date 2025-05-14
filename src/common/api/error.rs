use clap::error;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("网络请求失败: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("响应解析失败: {0}")]
    InvalidResponse(String),

    #[error("需要登录认证")]
    AuthRequired,

    #[error("服务暂时不可用，请稍后重试")]
    RetryLater,

    #[error("API限制访问: {0}")]
    AccessDenied(String),

    #[error("操作超时")]
    OperationTimeout,

    #[error("加锁失败")]
    LockError,

    #[error("无效的会话")]
    InvalidSession,

    #[error("未知错误: {0}")]
    Unknown(String),

    #[error("B站 API 错误: {1}")]
    ApiError(i64, String),  // 添加这个类型来处理 B站 API 的错误

    #[error("显示错误: {0}")]
    DisplayError(String),

    #[error("二维码过期")]
    QrCodeExpired
}

impl From<serde_json::Error> for ApiError {
    fn from(e: serde_json::Error) -> Self {
        Self::InvalidResponse(e.to_string())
    }
}