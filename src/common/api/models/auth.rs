use serde_derive::Deserialize;

/// 登录状态响应结构
#[derive(Debug, Deserialize)]
pub struct LoginStatusResponse { 
    pub code: i32,
     
    pub message: String, 
    
    pub data: Option<LoginStatusData>,
}

#[derive(Debug, Deserialize)]
pub struct LoginStatusData {
    pub url: String,
    pub code: i32,
    pub refresh_token: String,
    pub timestamp: u64,
    pub message: String,
}

impl LoginStatusResponse {
    // /// 提取用户UID
    // pub fn uid(&self) -> u64 {
    //     self.uid
    // }
    
    // /// 获取格式化后的cookies
    // pub fn formatted_cookies(&self) -> String {
    //     self.cookies.replace(';', "; ")
    // }
}

/// 暴露给业务层的二维码数据
#[derive(Debug, Deserialize)]
pub struct QrLoginData {
    pub url: String,
    pub qrcode_key: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginInfo{
    pub url: String,
    pub qrcode_key: String,
    pub token:String,
    pub challenge: String,
    pub gt: String,
    pub validate: String,
    pub seccode: String,
    captcha_key: String,
}


#[derive(Debug, Deserialize)]
pub struct QrStatus {
    pub status: String,
    pub message: String,
    pub data: Option<LoginInfo>,
}

#[derive(Debug, Deserialize)]
pub struct UserProfile {
    pub mid: u64,
    pub name: String,
    pub face: String,
    pub sign: String,
}