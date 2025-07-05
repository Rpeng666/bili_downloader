use crate::common::client::client::BiliClient;
use crate::common::client::models::common::CommonResponse;
use crate::common::models::{DownloadType, ParsedMeta};
use crate::downloader::models::{DownloadTask, FileType};
use crate::parser::detail_parser::Parser;
use crate::parser::detail_parser::danmaku_handler::DanmakuHandler;
use crate::parser::detail_parser::models::{DashItem, DownloadConfig, PlayUrlData};
use crate::parser::detail_parser::parser_trait::ParserOptions;
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
            return match resp.code {
                -403 => Err(ParseError::ParseError(format!(
                    "访问被拒绝（-403）: {}。可能原因：1. 视频需要登录或大会员权限 2. 视频被删除或私密 3. 地区限制", 
                    resp.message
                ))),
                -404 => Err(ParseError::ParseError(format!(
                    "视频不存在（-404）: {}。视频可能已被删除或URL错误", 
                    resp.message
                ))),
                62002 => Err(ParseError::ParseError(format!(
                    "视频不可见（62002）: {}。视频可能是私密视频或需要特定权限", 
                    resp.message
                ))),
                62012 => Err(ParseError::ParseError(format!(
                    "视频审核中（62012）: {}。视频正在审核，暂时无法访问", 
                    resp.message
                ))),
                _ => Err(ParseError::ParseError(format!(
                    "API返回错误（{}）: {}", 
                    resp.code, resp.message
                ))),
            };
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
            ("fnval".to_string(), "16".to_string()), // 16表示需要音视频分离
            ("fourk".to_string(), "1".to_string()),  // 1表示需要4K视频
            ("fnver".to_string(), "0".to_string()),  // 0表示使用最新版本
        ]);

        debug!("请求播放地址参数: {:?}", params);
        debug!("目标清晰度: {:?} ({})", config.resolution, config.resolution as i32);

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
            return match resp.code {
                -403 => Err(ParseError::ParseError(format!(
                    "播放地址获取被拒绝（-403）: {}。可能原因：1. 清晰度需要大会员权限 2. Cookie已过期 3. 需要登录", 
                    resp.message
                ))),
                -404 => Err(ParseError::ParseError(format!(
                    "播放地址不存在（-404）: {}。视频可能已被删除", 
                    resp.message
                ))),
                -10403 => Err(ParseError::ParseError(format!(
                    "大会员专享（-10403）: {}。当前清晰度需要大会员权限，请登录大会员账号或选择较低清晰度", 
                    resp.message
                ))),
                _ => Err(ParseError::ParseError(format!(
                    "播放地址API返回错误（{}）: {}", 
                    resp.code, resp.message
                ))),
            };
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

    fn select_video_stream(
        &self,
        streams: &[DashItem],
        resolution: VideoQuality,
    ) -> Result<Option<String>, ParseError> {
        if streams.is_empty() {
            return Err(ParseError::ParseError(
                "没有可用的视频流。可能原因：1. 视频需要大会员权限 2. 当前清晰度不可用 3. Cookie已过期，请重新登录".to_string()
            ));
        }

        debug!("可用的视频流数量: {}", streams.len());
        for (i, stream) in streams.iter().enumerate() {
            debug!("流 {}: 清晰度ID={}, width={:?}, height={:?}", 
                i, stream.id, stream.width, stream.height);
        }

        let target_quality_id = resolution as i32;
        debug!("目标清晰度ID: {}", target_quality_id);

        // 首先尝试精确匹配清晰度ID
        if let Some(stream) = streams.iter().find(|s| s.id == target_quality_id) {
            debug!("找到精确匹配的清晰度: ID={}", stream.id);
            return Ok(Some(stream.base_url.clone()));
        }

        // 如果没有精确匹配，选择最接近且不超过目标清晰度的流
        let mut suitable_streams: Vec<_> = streams.iter()
            .filter(|s| s.id <= target_quality_id)
            .collect();
        
        if !suitable_streams.is_empty() {
            // 按清晰度ID降序排序，选择最高的
            suitable_streams.sort_by(|a, b| b.id.cmp(&a.id));
            let selected = suitable_streams[0];
            debug!("选择最接近的清晰度: ID={} (目标: {})", selected.id, target_quality_id);
            return Ok(Some(selected.base_url.clone()));
        }

        // 如果所有流的清晰度都高于目标，选择最低的
        let mut all_streams = streams.to_vec();
        all_streams.sort_by(|a, b| a.id.cmp(&b.id));
        let fallback = &all_streams[0];
        
        // 检查是否是高质量视频权限问题
        let highest_available_quality = all_streams.last().map(|s| s.id).unwrap_or(0);
        if target_quality_id >= 112 && highest_available_quality < target_quality_id { // 112是1080P+
            warn!("目标清晰度 {} 可能需要大会员权限，最高可用清晰度: {}", 
                target_quality_id, highest_available_quality);
            warn!("💡 提示：1080P+、4K等高清晰度通常需要大会员权限，请确保已登录大会员账号");
        }
        
        debug!("目标清晰度过低，降级到最低可用清晰度: ID={}", fallback.id);
        
        Ok(Some(fallback.base_url.clone()))
    }

    fn select_audio_stream(
        &self,
        streams: &[DashItem],
    ) -> Result<Option<String>, ParseError> {
        if streams.is_empty() {
            return Err(ParseError::ParseError(
                "没有可用的音频流。可能原因：1. 视频源异常 2. 网络问题 3. Cookie已过期".to_string()
            ));
        }

        debug!("可用的音频流数量: {}", streams.len());
        for (i, stream) in streams.iter().enumerate() {
            debug!("音频流 {}: 清晰度ID={}, 编码={}, 带宽={}", 
                i, stream.id, stream.codecs, stream.bandwidth);
        }

        // 按音频质量（带宽）降序排序，选择最高质量的音频
        let mut sorted_streams = streams.to_vec();
        sorted_streams.sort_by(|a, b| b.bandwidth.cmp(&a.bandwidth));
        
        let selected = &sorted_streams[0];
        debug!("选择最高质量音频流: ID={}, 带宽={}", selected.id, selected.bandwidth);
        
        Ok(Some(selected.base_url.clone()))
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
            Some(DownloadTask::new(
                danmaku_download_url,
                FileType::Danmaku,
                video_info.title.clone() + ".xml",
                format!("./tmp/{}-danmaku.xml", video_info.title),
                video_info.cid.to_string(),
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
            self.select_video_stream(&play_info.dash.as_ref().unwrap().video, config.resolution)?
                .map(|video_url| {
                    DownloadTask::new(
                        video_url,
                        FileType::Video,
                        video_info.title.clone() + ".mp4",
                        format!("./tmp/{}-video.mp4", video_info.title),
                        video_info.cid.to_string(),
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
            self.select_audio_stream(&play_info.dash.as_ref().unwrap().audio)?
                .map(|audio_url| {
                    DownloadTask::new(
                        audio_url,
                        FileType::Audio,
                        video_info.title.clone() + ".mp3",
                        format!("./tmp/{}-audio.mp3", video_info.title),
                        video_info.cid.to_string(),
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
            play_info
                .durl
                .as_ref()
                .and_then(|d| d.first())
                .map(|mp4_info| mp4_info.url.clone())
                .map(|mp4_info| {
                    DownloadTask::new(
                        mp4_info,
                        FileType::Video,
                        video_info.title.clone() + ".mp4",
                        format!("./tmp/{}-durl-video.mp4", video_info.title),
                        video_info.cid.to_string(),
                        HashMap::from([("desc".to_string(), video_info.desc.clone())]),
                    )
                })
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
