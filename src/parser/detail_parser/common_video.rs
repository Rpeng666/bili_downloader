use crate::common::client::client::BiliClient;
use crate::common::client::models::common::CommonResponse;
use crate::common::download_type::dash::DashVideoInfo;
use crate::common::download_type::mp4::Mp4VideoInfo;
use crate::common::models::DownloadType;
use crate::parser::detail_parser::models::PlayUrlData;
use crate::parser::detail_parser::{Parser, ParserOptions, VideoQuality};
use crate::parser::errors::ParseError;
use crate::parser::models::{ParsedMeta, StreamType, UrlType, VideoId};

use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashMap;
use tracing::debug;

pub struct CommonVideoParser<'a> {
    client: &'a BiliClient,
}

#[async_trait]
impl<'a> Parser for CommonVideoParser<'a> {
    async fn parse(&mut self, url_type: &UrlType) -> Result<ParsedMeta, ParseError> {
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
        let video_info = self.__get_video_info(bvid).await?;

        // 获取播放地址信息
        let play_url_info = self.__get_play_url(&video_info).await?;

        // 返回视频元数据
        Ok(ParsedMeta {
            title: video_info.title.clone(),
            stream_type: StreamType::Dash,
            meta: play_url_info,
        })
    }

    async fn parse_with_options(
        &mut self,
        url_type: &UrlType,
        options: ParserOptions,
    ) -> Result<ParsedMeta, ParseError> {
        // 确保传入的是普通视频选项
        let quality = match options {
            ParserOptions::CommonVideo { quality } => quality,
            _ => return Err(ParseError::ParseError("无效的普通视频解析选项".to_string())),
        };

        match url_type {
            UrlType::CommonVideo(video_id) => {
                let video_info = self.get_video_info(video_id).await?;
                debug!("视频信息: {:?}", video_info);
                self.create_video_meta(&video_info, quality).await
            }
            _ => Err(ParseError::InvalidUrl),
        }
    }
}

impl<'a> CommonVideoParser<'a> {
    pub fn new(client: &'a BiliClient) -> Self {
        Self { client }
    }

    async fn get_video_info(&self, video_id: &VideoId) -> Result<CommonVideoInfo, ParseError> {
        let mut params = HashMap::new();

        if let Some(bvid) = &video_id.bvid {
            params.insert("bvid".to_string(), bvid.clone());
        } else if let Some(aid) = &video_id.aid {
            params.insert("aid".to_string(), aid.to_string());
        } else {
            return Err(ParseError::ParseError("需要提供 bvid 或 aid".to_string()));
        }

        let resp = self
            .client
            .get_auto::<CommonResponse<CommonVideoInfo>>(
                "https://api.bilibili.com/x/web-interface/view",
                params,
            )
            .await
            .map_err(|e| ParseError::NetworkError(e.to_string()))?;

        resp.data
            .ok_or_else(|| ParseError::ParseError("未找到视频信息".to_string()))
    }

    async fn get_play_url(
        &self,
        video_info: &CommonVideoInfo,
        quality: VideoQuality,
    ) -> Result<PlayUrlData, ParseError> {
        let params = HashMap::from([
            ("bvid".to_string(), video_info.bvid.clone()),
            ("cid".to_string(), video_info.cid.to_string()),
            ("qn".to_string(), (quality as i32).to_string()),
            ("fnval".to_string(), "976".to_string()),
            ("fnver".to_string(), "0".to_string()),
            ("fourk".to_string(), "1".to_string()),
        ]);

        let resp = self
            .client
            .get_auto::<CommonResponse<PlayUrlData>>(
                "https://api.bilibili.com/x/player/playurl",
                params,
            )
            .await
            .map_err(|e| ParseError::NetworkError(e.to_string()))?;

        resp.data
            .ok_or_else(|| ParseError::ParseError("未找到播放地址信息".to_string()))
    }

    async fn create_video_meta(
        &self,
        video_info: &CommonVideoInfo,
        quality: VideoQuality,
    ) -> Result<ParsedMeta, ParseError> {
        let play_info = self.get_play_url(video_info, quality).await?;

        if let Some(dash_info) = play_info.dash {
            // 根据要求的质量选择视频流
            let video_stream = dash_info
                .video
                .iter()
                .filter(|v| v.quality <= quality as i32)
                .max_by_key(|v| v.quality)
                .ok_or_else(|| ParseError::ParseError("未找到所选质量的视频流".to_string()))?;

            let audio_stream = dash_info
                .audio
                .iter()
                .max_by_key(|a| a.quality)
                .ok_or_else(|| ParseError::ParseError("未找到可用的音频流".to_string()))?;

            let video_info = DashVideoInfo {
                url: format!("https://www.bilibili.com/video/{}", video_info.bvid),
                aid: video_info.aid,
                bvid: video_info.bvid.clone(),
                cid: video_info.cid,
                title: video_info.title.clone(),
                cover: video_info.pic.clone(),
                desc: video_info.desc.clone(),
                views: video_info.stat.view.to_string(),
                danmakus: video_info.stat.danmaku.to_string(),
                up_name: video_info.owner.name.clone(),
                up_mid: video_info.owner.mid,
                video_quality_id_list: vec![video_stream.quality as i32],
                video_url: video_stream.base_url.clone(),
                audio_url: audio_stream.base_url.clone(),
            };

            Ok(ParsedMeta {
                title: video_info.title.clone(),
                stream_type: StreamType::Dash,
                meta: DownloadType::CommonDash(video_info),
            })
        } else if let Some(mp4_info) = play_info.durl {
            let mp4_video_stream = mp4_info
                .iter()
                .find(|s| s.order == 1)
                .ok_or_else(|| ParseError::ParseError("未找到可用的视频流".to_string()))?;

            let video_info = Mp4VideoInfo {
                url: format!("https://www.bilibili.com/video/{}", video_info.bvid),
                aid: video_info.aid,
                bvid: video_info.bvid.clone(),
                cid: video_info.cid,
                title: video_info.title.clone(),
                cover: video_info.pic.clone(),
                desc: video_info.desc.clone(),
                views: video_info.stat.view.to_string(),
                danmakus: video_info.stat.danmaku.to_string(),
                up_name: video_info.owner.name.clone(),
                up_mid: video_info.owner.mid,
                video_url: mp4_video_stream.url.clone(),
            };

            Ok(ParsedMeta {
                title: video_info.title.clone(),
                stream_type: StreamType::MP4,
                meta: DownloadType::CommonMp4(video_info),
            })
        } else {
            Err(ParseError::ParseError("未解析出下载源地址".to_string()))
        }
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
