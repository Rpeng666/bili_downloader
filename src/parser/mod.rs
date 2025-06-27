use crate::{
    common::{client::client::BiliClient, models::ParsedMeta},
    parser::detail_parser::{
        get_detail_parser,
        parser_trait::ParserOptions,
    },
};
use errors::ParseError;
use models::UrlType;
use tracing::debug;

pub mod detail_parser;
pub mod errors;
pub mod models;
pub mod url_parser;

/// 主视频解析器，负责协调整个解析过程
pub struct VideoParser {
    api_client: BiliClient,
    authenticated: bool,
    parsed_meta: Option<ParsedMeta>,
}

impl VideoParser {
    pub fn new(api_client: BiliClient, authenticated: bool) -> Self {
        Self {
            api_client,
            authenticated,
            parsed_meta: None,
        }
    }

    /// 解析视频URL，返回视频元数据
    ///
    /// # 参数
    /// - `url`: 视频URL或ID
    ///
    /// # 返回值
    /// - `Result<ParsedMeta, ParseError>`: 解析成功返回视频元数据，失败返回错误
    pub async fn parse(
        &mut self,
        url: &str,
        options: &ParserOptions,
    ) -> Result<ParsedMeta, ParseError> {
        // 1. 解析URL，获取视频类型
        let url_type = url_parser::UrlParser::new().parse(url).await?;
        debug!("解析到视频类型: {:?}", url_type);

        // 2. 根据视频类型选择对应的解析器
        let mut parser = get_detail_parser(&url_type, &self.api_client)?;
        debug!("获取到解析器");

        // 3. 解析视频
        let parsed_meta = parser
            .parse_with_options(&url_type, options.clone())
            .await?;
        self.parsed_meta = Some(parsed_meta.clone());

        Ok(parsed_meta)
    }

    /// 检查视频是否需要登录
    pub fn need_login(&self, url_type: &UrlType) -> bool {
        url_type.need_login()
    }
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn test_parse_common_video() {
        // 测试普通视频解析
    }

    #[tokio::test]
    async fn test_parse_bangumi() {
        // 测试番剧解析
    }

    #[tokio::test]
    async fn test_parse_course() {
        // 测试课程解析
    }
}
