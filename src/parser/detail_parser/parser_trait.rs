use async_trait::async_trait;

use crate::{
    parser::{errors::ParseError, models::{ParsedMeta, UrlType}},
};

// 定义一个trait，用于解析视频信息，然后返回元数据
// 其他解析器可以实现这个trait
#[async_trait]
pub trait Parser {
    async fn parse(&mut self, url_type: &UrlType) -> Result<ParsedMeta, ParseError>;
}


