

pub struct StreamSelector;

impl StreamSelector {
    /// 根据流类型选择合适的流
    pub fn select_stream(
        &self,
        dash_info: &DashInfo,
        config: &DownloadConfig,
    ) -> Option<ParsedMeta> {
        match stream_type {
            "DASH" => streams.iter().find(|s| s.stream_type == "DASH").cloned(),
            "MP4" => streams.iter().find(|s| s.stream_type == "MP4").cloned(),
            _ => None,
        }
    }
}