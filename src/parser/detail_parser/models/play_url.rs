use serde_derive::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct PlayUrlData {
    pub format: String,
    pub timelength: i64,
    pub accept_description: Vec<String>,
    pub accept_quality: Vec<i32>,
    pub quality: Option<i32>,
    pub dash: Option<DashInfo>,
    pub durl: Option<Vec<Mp4Info>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DashInfo {
    pub duration: i32,
    pub video: Vec<DashItem>,
    pub audio: Vec<DashItem>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DashItem {
    pub id: i32,
    pub base_url: String,
    pub backup_url: Option<Vec<String>>,
    pub bandwidth: i32,
    pub mime_type: String,
    pub codecs: String,
    pub width: i32,
    pub height: i32,
    pub frame_rate: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Mp4Info {
    pub order: i32,
    pub length: i32,
    pub size: i64,
    pub url: String,
}
