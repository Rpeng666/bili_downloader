use errors::ParseError;
use models::{QualityOption, VideoMeta, VideoSegment, VideoType, VideoInfo};
use parser_trait::Parser;

use crate::common::api::client::BiliClient;

pub mod detector;
pub mod errors;
pub mod models;
pub mod parser_trait;
pub mod cheese_parser;
use cheese_parser::CheeseParser;
pub mod bangumi_parser;
use bangumi_parser::BangumiParser;
pub mod video_parser;
use video_parser::CommonVideoParser;
pub mod wbi_utils;
pub mod utils;

enum AnyParser<'a> {
    Cheese(CheeseParser),
    Common(CommonVideoParser<'a>),  
    Bangumi(BangumiParser),
}

impl<'a> Parser for AnyParser<'a> {
    async fn parse(&mut self, url: &str) -> Result<VideoMeta, ParseError> {
        match self {
            AnyParser::Cheese(p) => p.parse(url).await,
            AnyParser::Common(p) => p.parse(url).await,
            AnyParser::Bangumi(p) => p.parse(url).await,
        }
    }
}

pub struct VideoParser {
    api_client: BiliClient,
    authenticated: bool,
    video_info: Option<VideoInfo>,
}

impl VideoParser {

    pub fn new(api_client: BiliClient, authenticated: bool) -> Self {
        Self {
            api_client,
            authenticated,
            video_info: None,
        }
    }
    
    // 解析入口
    pub async fn parse(&mut self, url: &str) -> Result<VideoMeta, ParseError> {
        // 检测视频类型
        let video_type = detector::detect_video_type(url)?;
        println!("检测到视频类型：{}", video_type);

        let mut parser = match video_type {
            VideoType::CourseChapter(_) => AnyParser::Cheese(CheeseParser),
            VideoType::CommonVideo(_) => AnyParser::Common(CommonVideoParser::new(&self.api_client)),
            VideoType::BangumiEpisode(_) | VideoType::BangumiSeason(_) => AnyParser::Bangumi(BangumiParser),
            _ => return Err(ParseError::UnsupportedType),
        };

        // 解析视频
        let meta = parser.parse(url).await?;
        
        // 保存视频信息
        if let AnyParser::Common(p) = parser {
            self.video_info = Some(p.get_video_info().await?);
        }

        Ok(meta)
    }

    pub fn get_video_info(&self) -> Option<&VideoInfo> {
        self.video_info.as_ref()
    }
}