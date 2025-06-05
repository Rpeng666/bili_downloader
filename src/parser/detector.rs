use super::{errors::ParseError, models::VideoType};
use regex::Regex;

// 支持正则表达式
const PATTERNS: &str = r"cheese|av|BV|ep|ss|md|live|b23.tv|bili2233.cn|blackboard|festival";

pub fn detect_video_type(input: &str) -> Result<VideoType, ParseError> {
    // 优先处理URL解析
    if let Some(url_type) = parse_url(input) {
        return Ok(url_type);
    }
    detect_raw_id(input)
}

fn parse_url(url: &str) -> Option<VideoType> {
    let url = url.trim();
    let caps = Regex::new(PATTERNS).unwrap().captures(&url);
    // println!("url: {:?}", url);
    // println!("caps111: {:?}", caps);

    if let Some(caps) = caps {
        // println!("caps222: {:?}", caps);
        match &caps[0] {
            "cheese" => Some(VideoType::CourseChapter(url.to_string())),
            "av" => Some(VideoType::CommonVideo(url.to_string())),
            "BV" => Some(VideoType::CommonVideo(url.to_string())),
            "ep" => Some(VideoType::BangumiEpisode(url.to_string())),
            "ss" => Some(VideoType::BangumiSeason(url.to_string())),
            _ => None,
        }
    } else {
        None
    }
}

fn detect_raw_id(id: &str) -> Result<VideoType, ParseError> {
    let id = id.trim().to_uppercase();

    if id.starts_with("AV") {
        Ok(VideoType::CommonVideo(id))
    } else if id.starts_with("BV") {
        Ok(VideoType::CommonVideo(id))
    } else if id.starts_with("EP") {
        Ok(VideoType::BangumiEpisode(id[2..].to_string()))
    } else if id.starts_with("SS") {
        Ok(VideoType::BangumiSeason(id[2..].to_string()))
    } else {
        Err(ParseError::UnsupportedFormat)
    }
}
