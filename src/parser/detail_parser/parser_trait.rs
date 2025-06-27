use async_trait::async_trait;
use serde_derive::{Deserialize, Serialize};

use crate::{
    common::models::{DownloadType, ParsedMeta},
    parser::{
        detail_parser::models::DownloadConfig,
        errors::ParseError,
        models::{UrlType, VideoQuality},
    },
};

// 不同类型内容的特定选项
#[derive(Debug, Clone)]
pub enum ParserOptions {
    // 普通视频的选项
    CommonVideo {
        config: DownloadConfig, // 下载配置
    },

    // 番剧的选项
    Bangumi {
        config: DownloadConfig, // 下载配置
    },

    // 课程的选项
    Course {
        quality: VideoQuality,
        episode_range: Option<String>, // 仅用于整季课程
    },
}

impl Default for ParserOptions {
    fn default() -> Self {
        Self::CommonVideo {
            config: DownloadConfig::default(),
        }
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
pub enum StreamType {
    #[default]
    Dash, // DASH流
    MP4, // MP4流
}

// 定义一个trait，用于解析视频信息，然后返回元数据
#[async_trait]
pub trait Parser {
    // 解析可能返回多个视频的元数据
    async fn parse_with_options(
        &mut self,
        url_type: &UrlType,
        options: ParserOptions,
    ) -> Result<ParsedMeta, ParseError>;
}

// 解析集数范围字符串，返回需要下载的集数列表
// 例如: "1-3,5,7-9" => [1,2,3,5,7,8,9]
pub fn parse_episode_range(range_str: &str) -> Result<Vec<i64>, ParseError> {
    let mut episodes = Vec::new();

    for part in range_str.split(',') {
        if part.contains('-') {
            let range: Vec<&str> = part.split('-').collect();
            if range.len() != 2 {
                return Err(ParseError::ParseError("无效的集数范围格式".to_string()));
            }

            let start: i64 = range[0]
                .parse()
                .map_err(|_| ParseError::ParseError("无效的起始集数".to_string()))?;
            let end: i64 = range[1]
                .parse()
                .map_err(|_| ParseError::ParseError("无效的结束集数".to_string()))?;

            if start > end {
                return Err(ParseError::ParseError(
                    "起始集数不能大于结束集数".to_string(),
                ));
            }

            episodes.extend(start..=end);
        } else {
            let ep: i64 = part
                .parse()
                .map_err(|_| ParseError::ParseError("无效的集数".to_string()))?;
            episodes.push(ep);
        }
    }

    episodes.sort_unstable();
    episodes.dedup();

    if episodes.is_empty() {
        return Err(ParseError::ParseError("没有有效的集数".to_string()));
    }

    Ok(episodes)
}
