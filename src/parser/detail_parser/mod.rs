pub mod bangumi;
pub mod common_video;
pub mod course;
pub mod models;
pub mod parser_trait;
pub mod danmaku_handler;
pub mod stream_utils;
pub mod error_utils;
pub mod task_utils;

pub use bangumi::BangumiParser;
pub use common_video::CommonVideoParser;
pub use course::CourseParser;
pub use parser_trait::Parser;
pub use tracing::debug;

use crate::{
    common::client::client::BiliClient,
    parser::{errors::ParseError, models::UrlType},
};

pub fn get_detail_parser<'a>(
    url_type: &UrlType,
    client: &'a BiliClient,
) -> Result<Box<dyn Parser + 'a>, ParseError> {
    debug!("获取解析器: {:?}", url_type);
    // 根据 URL 类型选择对应的具体的解析器
    match url_type {
        UrlType::CommonVideo(_) => Ok(Box::new(CommonVideoParser::new(client))),
        UrlType::BangumiEpisode(_) | UrlType::BangumiSeason(_) => {
            Ok(Box::new(BangumiParser::new(client)))
        }
        UrlType::CourseEpisode(_) | UrlType::CourseSeason(_) => {
            Ok(Box::new(CourseParser::new(client)))
        }
        _ => Err(ParseError::UnsupportedType),
    }
}
