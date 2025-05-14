use std::fmt;
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
    #[error("解析错误: {0}")]
    ParseError(String),
    #[error("API错误: {0}")]
    ApiError(String),
    #[error("重定向: {0}")]
    Redirect(String),
}

impl From<crate::common::api::error::ApiError> for ParseError {
    fn from(err: crate::common::api::error::ApiError) -> Self {
        match err {
            crate::common::api::error::ApiError::InvalidResponse(msg) => ParseError::ApiError(msg),
            crate::common::api::error::ApiError::Reqwest(e) => ParseError::ApiError(e.to_string()),
            crate::common::api::error::ApiError::ApiError(_, msg) => ParseError::ApiError(msg),
            crate::common::api::error::ApiError::Unknown(msg) => ParseError::ApiError(msg),
            _ => ParseError::ApiError(err.to_string()),
        }
    }
}

impl From<ParseIntError> for ParseError {
    fn from(err: ParseIntError) -> Self {
        ParseError::ParseError(err.to_string())
    }
}