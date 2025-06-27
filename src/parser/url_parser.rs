use super::errors::ParseError;
use super::models::{UrlType, VideoId};
use lazy_static::lazy_static;
use regex::Regex;
use reqwest::Client;
use url::Url;

pub struct UrlParser {
    client: Client,
}

impl UrlParser {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn parse(&self, input: &str) -> Result<UrlType, ParseError> {
        // 标准化URL
        let url = self.normalize_url(input).await?;

        // 解析视频类型
        self.extract_video_type(&url).await
    }

    async fn normalize_url(&self, input: &str) -> Result<String, ParseError> {
        // 处理短链接
        if input.contains("b23.tv") {
            return self.expand_short_url(input).await;
        }

        // 处理移动端链接
        if input.contains("m.bilibili.com") {
            return Ok(input.replace("m.bilibili.com", "www.bilibili.com"));
        }

        // 尝试解析URL
        if let Ok(url) = Url::parse(input) {
            Ok(url.into())
        } else {
            // 如果不是URL，可能是裸ID
            self.handle_raw_id(input)
        }
    }

    async fn expand_short_url(&self, url: &str) -> Result<String, ParseError> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| ParseError::NetworkError(e.to_string()))?;

        if let Some(location) = response.headers().get("location") {
            Ok(location.to_str().unwrap_or_default().to_string())
        } else {
            Err(ParseError::InvalidShortUrl)
        }
    }

    fn handle_raw_id(&self, id: &str) -> Result<String, ParseError> {
        lazy_static! {
            static ref BV_PATTERN: Regex = Regex::new(r"^BV[0-9A-Za-z]{10}$").unwrap();
            static ref AV_PATTERN: Regex = Regex::new(r"^av(\d+)$").unwrap();
            static ref EP_PATTERN: Regex = Regex::new(r"^ep(\d+)$").unwrap();
            static ref SS_PATTERN: Regex = Regex::new(r"^ss(\d+)$").unwrap();
            static ref COURSE_EP_PATTERN: Regex = Regex::new(r"^cp(\d+)$").unwrap();
            static ref COURSE_SS_PATTERN: Regex = Regex::new(r"^cs(\d+)$").unwrap();
        }

        let id = id.trim();

        if BV_PATTERN.is_match(id) {
            Ok(format!("https://www.bilibili.com/video/{}", id))
        } else if AV_PATTERN.is_match(id) {
            Ok(format!("https://www.bilibili.com/video/{}", id))
        } else if EP_PATTERN.is_match(id) {
            Ok(format!("https://www.bilibili.com/bangumi/play/{}", id))
        } else if SS_PATTERN.is_match(id) {
            Ok(format!("https://www.bilibili.com/bangumi/play/{}", id))
        } else if COURSE_EP_PATTERN.is_match(id) {
            Ok(format!(
                "https://www.bilibili.com/cheese/play/ep{}",
                &id[2..]
            ))
        } else if COURSE_SS_PATTERN.is_match(id) {
            Ok(format!(
                "https://www.bilibili.com/cheese/play/ss{}",
                &id[2..]
            ))
        } else {
            Err(ParseError::InvalidUrl)
        }
    }

    async fn extract_video_type(&self, url: &str) -> Result<UrlType, ParseError> {
        lazy_static! {
            static ref VIDEO_PATTERNS: Vec<(Regex, fn(&str) -> UrlType)> = vec![
                // BV号
                (Regex::new(r"BV([0-9A-Za-z]{10})").unwrap(),
                |id| UrlType::CommonVideo(VideoId { bvid: Some(id.to_string()), aid: None })),

                // av号
                (Regex::new(r"av(\d+)").unwrap(),
                |id| UrlType::CommonVideo(VideoId { bvid: None, aid: Some(id.parse().unwrap_or_default()) })),

                // 课程单集
                (Regex::new(r"cheese/play/ep(\d+)").unwrap(),
                |id| UrlType::CourseEpisode(id.to_string())),

                // 课程整季
                (Regex::new(r"cheese/play/ss(\d+)").unwrap(),
                |id| UrlType::CourseSeason(id.to_string())),

                // 番剧ep
                (Regex::new(r"ep(\d+)").unwrap(),
                |id| UrlType::BangumiEpisode(id.to_string())),

                // 番剧ss
                (Regex::new(r"ss(\d+)").unwrap(),
                |id| UrlType::BangumiSeason(id.to_string())),

                // 直播间
                (Regex::new(r"live.bilibili.com/(\d+)").unwrap(),
                |id| UrlType::LiveRoom(id.to_string())),

                // 合集
                (Regex::new(r"medialist/detail/ml(\d+)").unwrap(),
                |id| UrlType::Collection(id.to_string())),

                // 收藏夹
                (Regex::new(r"favlist/(\d+)").unwrap(),
                |id| UrlType::Favorite(id.to_string())),

                // UP主合集
                (Regex::new(r"medialist/play/ml(\d+)").unwrap(),
                |id| UrlType::UgcSeason(id.to_string())),

                // 专栏
                (Regex::new(r"read/cv(\d+)").unwrap(),
                |id| UrlType::Article(id.to_string())),
            ];
        }

        for (pattern, constructor) in VIDEO_PATTERNS.iter() {
            if let Some(caps) = pattern.captures(url) {
                return Ok(constructor(&caps[1]));
            }
        }

        Err(ParseError::UnsupportedFormat)
    }
}
