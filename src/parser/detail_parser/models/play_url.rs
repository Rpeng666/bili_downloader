use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct PlayUrlData {
    pub format: String,                  // 流格式
    pub timelength: i64,                 // 时长，单位为秒
    pub quality: Option<i32>,            // 当前选择的分辨率ID
    pub dash: Option<DashInfo>,          // DASH流信息
    pub durl: Option<Vec<Mp4Info>>,           // MP4流信息
    pub durls: Option<Vec<DurlInfo>>,    // MP4流信息
}

// ------------------------------------------------------------------------------------------
#[derive(Debug, Clone, Deserialize)]
pub struct DashInfo {
    pub duration: i64,        // 时长，单位为秒
    pub audio: Vec<DashItem>, // 音频流信息
    pub video: Vec<DashItem>, // 视频流信息
}

#[derive(Debug, Clone, Deserialize)]
pub struct DashItem {
    pub id: i32,                         // 流ID
    pub base_url: String,                // 基础URL
    pub backup_url: Option<Vec<String>>, // 备用URL列表
    pub mime_type: String,               // MIME类型
    pub codecs: String,                  // 编解码器信息
    pub bandwidth: i64,                  // 带宽，单位为bps
    pub width: Option<i32>,              // 宽度，单位为像素
    pub height: Option<i32>,             // 高度，单位为像素
    pub frame_rate: Option<String>,      // 帧率
}

// ------------------------------------------------------------------------------------------
#[derive(Debug, Clone, Deserialize)]
pub struct DurlInfo {
    pub quality: i32,  // 分辨率ID
    pub durl: Vec<Mp4Info>, // MP4流信息
}

#[derive(Debug, Clone, Deserialize)]
pub struct Mp4Info {
    pub order: i32,                      // 流的顺序
    pub length: i64,                     // 时长，单位为秒
    pub size: i64,                       // 文件大小，单位为字节
    pub url: String,                     // 流的URL
    pub backup_url: Option<Vec<String>>, // 备用URL列表
    pub quality: Option<i32>,            // 分辨率ID
}
