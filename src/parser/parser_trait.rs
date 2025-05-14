use crate::common::api::client::BiliClient;

use super::{errors::ParseError, models::VideoMeta};



// 定义一个trait，用于解析视频信息，然后返回元数据
// 其他解析器可以实现这个trait
pub trait Parser {
    async fn parse(&mut self, url: &str) -> Result<VideoMeta, ParseError>;
}
