use serde_derive::Deserialize;

/// 登录状态响应结构
#[derive(Debug, Deserialize)]
pub struct CommonResponse<T> {
    pub code: i32,

    pub message: String,

    pub data: Option<T>,

    pub result: Option<T>,
}
