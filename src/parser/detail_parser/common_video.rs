use crate::common::client::client::BiliClient;
use crate::common::client::models::common::CommonResponse;
use crate::common::models::{DownloadType, ParsedMeta};
use crate::downloader::models::{DownloadTask, FileType};
use crate::parser::detail_parser::Parser;
use crate::parser::detail_parser::danmaku_handler::DanmakuHandler;
use crate::parser::detail_parser::error_utils::handle_api_error;
use crate::parser::detail_parser::models::{DashItem, DownloadConfig, PlayUrlData};
use crate::parser::detail_parser::parser_trait::ParserOptions;
use crate::parser::detail_parser::stream_utils::{select_audio_stream, select_video_stream};
use crate::parser::detail_parser::task_utils::{create_audio_task, create_danmaku_task, create_video_task};
use crate::parser::errors::ParseError;
use crate::parser::models::{UrlType, VideoQuality};

use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashMap;
use tracing::{debug, warn};

pub struct CommonVideoParser<'a> {
    client: &'a BiliClient,
}

#[async_trait]
impl<'a> Parser for CommonVideoParser<'a> {
    async fn parse_with_options(
        &mut self,
        url_type: &UrlType,
        options: ParserOptions,
    ) -> Result<ParsedMeta, ParseError> {
        let config = match &options {
            ParserOptions::CommonVideo { config } => config,
            _ => return Err(ParseError::ParseError("无效的普通视频解析选项".to_string())),
        };

        let url_info = match url_type {
            UrlType::CommonVideo(url_info) => url_info,
            _ => return Err(ParseError::InvalidUrl),
        };

        // 提取 bvid
        let bvid = url_info
            .bvid
            .as_ref()
            .ok_or_else(|| ParseError::ParseError("未找到bvid".to_string()))?
            .clone();

        // 获取视频信息
        let video_info = self.get_video_info(Some(bvid), None).await?;

        // 获取播放地址信息
        let download_items = self.create_video_meta(&video_info, config).await?;

        // 返回视频元数据
        Ok(ParsedMeta {
            title: video_info.title.clone(),
            download_type: DownloadType::CommonVideo,
            download_items: download_items,
        })
    }
}

impl<'a> CommonVideoParser<'a> {
    pub fn new(client: &'a BiliClient) -> Self {
        Self { client }
    }

    async fn get_video_info(
        &self,
        bvid: Option<String>,
        avid: Option<i64>,
    ) -> Result<CommonVideoInfo, ParseError> {
        let params = match (bvid, avid) {
            (Some(bvid), None) => HashMap::from([("bvid".to_string(), bvid)]),
            (None, Some(avid)) => HashMap::from([("aid".to_string(), avid.to_string())]),
            _ => return Err(ParseError::ParseError("必须提供bvid或avid".to_string())),
        };

        let resp = self
            .client
            .get_auto::<CommonResponse<CommonVideoInfo>>(
                "https://api.bilibili.com/x/web-interface/view",
                params,
            )
            .await
            .map_err(|e| ParseError::NetworkError(e.to_string()))?;

        // 检查API返回的错误码
        if resp.code != 0 {
            return Err(handle_api_error(resp.code, &resp.message, "视频"));
        }

        resp.data
            .ok_or_else(|| ParseError::ParseError("API响应中未找到视频信息".to_string()))
    }

    async fn get_play_url(
        &self,
        video_info: &CommonVideoInfo,
        config: &DownloadConfig,
    ) -> Result<PlayUrlData, ParseError> {
        let params = HashMap::from([
            ("bvid".to_string(), video_info.bvid.clone()),
            ("cid".to_string(), video_info.cid.to_string()),
            ("qn".to_string(), (config.resolution as i32).to_string()), // 设置清晰度
            ("fnval".to_string(), "16".to_string()),                    // 16表示需要音视频分离
            ("fourk".to_string(), "1".to_string()),                     // 1表示需要4K视频
            ("fnver".to_string(), "0".to_string()),                     // 0表示使用最新版本
        ]);

        debug!("请求播放地址参数: {:?}", params);
        debug!(
            "目标清晰度: {:?} ({})",
            config.resolution, config.resolution as i32
        );

        let resp = self
            .client
            .get_auto::<CommonResponse<PlayUrlData>>(
                "https://api.bilibili.com/x/player/playurl",
                params,
            )
            .await
            .map_err(|e| ParseError::NetworkError(e.to_string()))?;

        // 检查API返回的错误码
        if resp.code != 0 {
            return Err(handle_api_error(resp.code, &resp.message, "播放地址"));
        }

        resp.data
            .ok_or_else(|| ParseError::ParseError("未找到播放地址信息".to_string()))
            .and_then(|data| {
                if data.dash.is_none() && data.durl.is_none() {
                    Err(ParseError::ParseError("未解析出播放地址".to_string()))
                } else {
                    Ok(data)
                }
            })
    }

    async fn create_video_meta(
        &self,
        video_info: &CommonVideoInfo,
        config: &DownloadConfig,
    ) -> Result<Vec<DownloadTask>, ParseError> {
        let play_info = self.get_play_url(video_info, config).await?;
        debug!("获取到播放地址信息: {:?}", play_info);

        let mut download_task_vec: Vec<DownloadTask> = Vec::new();

        // --------------------------------------------------------------------
        let danmaku_download_task = if config.need_danmaku {
            let danmaku_download_url = DanmakuHandler::get_url(video_info.cid)
                .map_err(|e| ParseError::ParseError(e.to_string()))?;
            Some(create_danmaku_task(
                danmaku_download_url,
                &video_info.title,
                &config.output_dir,
                video_info.cid,
                HashMap::from([("desc".to_string(), video_info.desc.clone())]),
            ))
        } else {
            None
        };
        if danmaku_download_task.is_some() {
            download_task_vec.push(danmaku_download_task.unwrap());
        }

        // --------------------------------------------------------------------
        let video_stream_task = if config.need_video && play_info.dash.is_some() {
            select_video_stream(&play_info.dash.as_ref().unwrap().video, config.resolution)?
                .map(|video_url| {
                    create_video_task(
                        video_url,
                        &video_info.title,
                        None,
                        &config.output_dir,
                        HashMap::from([("desc".to_string(), video_info.desc.clone())]),
                    )
                })
        } else {
            None
        };

        if let Some(task) = video_stream_task {
            download_task_vec.push(task);
        }

        // --------------------------------------------------------------------
        let audio_stream_task = if config.need_audio && play_info.dash.is_some() {
            select_audio_stream(&play_info.dash.as_ref().unwrap().audio)?
                .map(|audio_url| {
                    create_audio_task(
                        audio_url,
                        &video_info.title,
                        None,
                        &config.output_dir,
                        HashMap::from([("desc".to_string(), video_info.desc.clone())]),
                    )
                })
        } else {
            None
        };

        if let Some(task) = audio_stream_task {
            download_task_vec.push(task);
        }

        // --------------------------------------------------------------------
        let mp4_stream_task: Option<DownloadTask> = if config.need_video && play_info.durl.is_some()
        {
            let play_info = play_info
                .durl
                .as_ref()
                .unwrap()
                .first()
                .ok_or_else(|| ParseError::ParseError("未找到 MP4 流信息".to_string()))?;
            debug!("MP4流信息: {:?}", play_info);

            Some(DownloadTask::new(
                play_info.url.clone(),
                FileType::Video,
                video_info.title.clone() + ".mp4",
                format!("./tmp/{}-durl-video.mp4", video_info.title),
                format!("./tmp/{}-durl-video.mp4", video_info.title),
                HashMap::from([("desc".to_string(), video_info.desc.clone())]),
            ))
        } else {
            None
        };

        for task in mp4_stream_task {
            download_task_vec.push(task);
        }

        // --------------------------------------------------------------------
        Ok(download_task_vec)
    }
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct VideoInfo {
    // 基础标识
    pub url: String,
    pub aid: i64,
    pub bvid: String,
    pub cid: i64,

    // 视频元数据
    pub title: String,
    pub cover: String,
    pub desc: String,
    pub views: String,
    pub danmakus: String,

    // UP主信息
    pub up_name: String,
    pub up_mid: i64,

    pub video_quality_id_list: Vec<i32>,

    // 流信息
    pub video_url: String,
    pub audio_url: String,
}

#[derive(Debug, Deserialize)]
pub struct CommonVideoInfo {
    redirect_url: Option<String>,
    title: String,
    pic: String,
    desc: String,
    owner: OwnerInfo,
    cid: i64,
    bvid: String,
}

#[derive(Debug, Deserialize)]
pub struct OwnerInfo {
    name: String,
    mid: i64,
}
