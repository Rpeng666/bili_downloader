use regex::Regex;

use super::{errors::ParseError, models::VideoMeta, parser_trait::Parser};
pub struct CheeseParser;


impl Parser for CheeseParser {
    async fn parse(&mut self, url: &str) -> Result<VideoMeta, ParseError> {
        // 解析Cheese视频链接
        let re = Regex::new(r"^https?://cheese\.com/video/(\d+)$").unwrap();
        if let Some(caps) = re.captures(url) {
            let video_id = caps[1].to_string();
            // 这里可以添加实际的解析逻辑
            Ok(VideoMeta {
                title: format!("Cheese Video {}", video_id),
                duration: 0,
                segments: vec![],
                quality_options: vec![],
            })
        } else {
            Err(ParseError::InvalidUrl)
        }
    }
}