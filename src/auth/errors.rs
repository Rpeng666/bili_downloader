use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("网络请求失败: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("二维码生成失败: {0}")]
    QrError(#[from] qrcode::types::QrError),

    #[error("IO操作失败: {0}")]
    IoError(#[from] std::io::Error),

    #[error("登录超时")]
    Timeout,

    #[error("API返回操作：{0}")]
    ApiError(String)
}

pub type Result<T> = std::result::Result<T, AuthError>;