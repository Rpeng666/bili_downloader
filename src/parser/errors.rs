use crate::common::client::error::ApiError;
use std::num::ParseIntError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("无效的URL")]
    InvalidUrl,

    #[error("不支持的视频类型")]
    UnsupportedType,

    #[error("不支持的格式")]
    UnsupportedFormat,

    #[error("无效的短链接")]
    InvalidShortUrl,

    #[error("网络错误: {0}")]
    NetworkError(String),

    #[error("需要登录")]
    LoginRequired,

    #[error("重定向失败: {0}")]
    RedirectFailed(String),

    #[error("解析错误: {0}")]
    ParseError(String),

    #[error("API错误: {0}")]
    ApiError(String),

    #[error("重定向错误: {0}")]
    Redirect(String),

    #[error("需要付费")]
    PaymentRequired,
}

impl From<ApiError> for ParseError {
    fn from(err: ApiError) -> Self {
        match err {
            ApiError::InvalidResponse(msg) => ParseError::ApiError(msg),
            ApiError::Reqwest(e) => ParseError::ApiError(e.to_string()),
            ApiError::ApiError(_, msg) => ParseError::ApiError(msg),
            ApiError::Unknown(msg) => ParseError::ApiError(msg),
            _ => ParseError::ApiError(err.to_string()),
        }
    }
}

impl From<ParseIntError> for ParseError {
    fn from(err: ParseIntError) -> Self {
        ParseError::ParseError(err.to_string())
    }
}
