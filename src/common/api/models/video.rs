use serde_derive::Deserialize;

#[derive(Debug, Deserialize)]
pub struct VideoInfo {
    pub title: String,
    pub duration: u64,
    pub pages: Vec<VideoPage>,
}

#[derive(Debug, Deserialize)]
pub struct VideoPage {
    pub cid: u64,
    pub page: u32,
    pub part: String,
}