use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct PlayUrlResponse {
    pub data: Option<PlayUrlData>,
    pub result: Option<PlayUrlData>
}

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
    pub quality: u16,

    #[serde(rename = "codecid")]
    pub codec_id: u8,

    #[serde(rename = "baseUrl")]
    pub base_url: String,

    #[serde(rename = "mimeType")]
    pub mime_type: String,


    pub bandwidth: i64,
}

#[derive(Debug, Deserialize)]
pub struct AudioStream {
    #[serde(rename = "id")]
    pub quality: u16,

    #[serde(rename = "baseUrl")]
    pub base_url: String,

    #[serde(rename = "mimeType")]
    pub mime_type: String,

    pub bandwidth: i64,
}
