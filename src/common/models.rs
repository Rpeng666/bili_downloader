use async_trait::async_trait;
use serde::Deserialize;

use crate::{
    downloader::models::DownloadTask,
    parser::{detail_parser::parser_trait::ParserOptions, errors::ParseError},
    post_process::post_process,
};

// -----------------------------------------------------------------------------------------------
#[async_trait]
pub trait DownloadTaskTrait {
    // 将当前类型转换为下载任务
    async fn to_download_task(&self) -> Result<DownloadTask, ParseError>;
    // 处理下载任务完成的后处理任务（比如音频和视频合并）
    async fn post_process(
        &self,
        task: &DownloadTask,
        parser_options: &ParserOptions,
    ) -> Result<(), ParseError>;
}

// -----------------------------------------------------------------------------------------------

//需要下载数据的元数据
#[derive(Debug, Clone)]
pub struct ParsedMeta {
    pub title: String,                     // 标题
    pub download_type: DownloadType,       // 下载类型
    pub download_items: Vec<DownloadTask>, // 下载任务列表
}

impl ParsedMeta {
    pub async fn post_process(
        &self,
        task: &Vec<DownloadTask>,
        parser_options: &ParserOptions,
    ) -> Result<(), ParseError> {
        match &self.download_type {
            DownloadType::CommonVideo => post_process(task, parser_options).await,
            DownloadType::Bangumi => post_process(task, parser_options).await,
            DownloadType::Course => post_process(task, parser_options).await,
            _ => Err(ParseError::ParseError("不支持的下载类型".to_string())),
        }
    }
}

// -----------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub enum DownloadType {
    CommonVideo, // 普通视频下载
    Bangumi,     // 番剧下载
    Course,
    // Course(CourseInfoVec), // 课程下载
}
