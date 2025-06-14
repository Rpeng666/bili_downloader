use serde_derive::Deserialize;

/// 登录状态响应结构
#[derive(Debug, Deserialize)]
pub struct CommonResponse<T> {
    pub code: i32,

    pub message: String,

    pub data: Option<T>,
}

/// 用户信息响应结构
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct UserInfoResponse {
    #[serde(rename = "isLogin")]
    pub is_login: bool,

    pub face: String,

    pub mid: u64,

    pub uname: String,

    #[serde(rename = "vipType")]
    pub vip_type: u32,

    #[serde(rename = "vipStatus")]
    pub vip_status: u32,
}
