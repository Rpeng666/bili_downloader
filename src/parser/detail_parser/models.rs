use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct PlayUrlData {
    pub durl: Option<Vec<MP4Segment>>, // MP4
    pub dash: Option<DashInfo>,        //dash
}

#[derive(Debug, Deserialize)]
pub struct DashInfo {
    pub video: Vec<VideoStream>,
    pub audio: Vec<AudioStream>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MP4Segment {
    pub size: i64,
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct VideoStream {
    #[serde(rename = "id")]
    pub quality: u64,

    #[serde(rename = "codecid")]
    pub codec_id: u8,

    pub base_url: String,

    pub mime_type: String,


    pub bandwidth: i64,
}

#[derive(Debug, Deserialize)]
pub struct AudioStream {
    #[serde(rename = "id")]
    pub quality: u64,

    pub base_url: String,

    pub mime_type: String,

    pub bandwidth: i64,
}
