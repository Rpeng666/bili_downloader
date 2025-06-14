use std::collections::HashMap;

use crate::common::client::client::BiliClient;
use crate::common::client::models::common::CommonResponse;
use crate::common::download_type::dash::DashVideoInfo;
use crate::common::download_type::mp4::Mp4VideoInfo;
use crate::common::models::DownloadType;
use crate::parser::detail_parser::models::PlayUrlData;
use crate::parser::detail_parser::Parser;
use crate::parser::models::UrlType;
use crate::parser::{
    errors::ParseError,
    models::{ParsedMeta, StreamType},
};
use async_trait::async_trait;
use serde_derive::Deserialize;
use tracing::debug;

#[derive(Debug, Deserialize)]
struct BangumiInfo {
    title: String, // 番剧标题

    total: u32, // 总集数

    #[serde(rename = "mediaInfo")]
    media_info: Option<MediaInfo>,

    episodes: Vec<Episode>,
}

#[derive(Debug, Deserialize)]
struct MediaInfo {
    #[serde(rename = "typeName")]
    type_name: String, // 类型名称
                       // 其他可能需要的字段
}

#[derive(Debug, Deserialize)]
struct Episode {
    id: u64, // ep_id

    aid: i64, // av号

    cid: i64, // 视频cid

    title: String, // 标题

    long_title: String, // 长标题

    duration: u32, // 时长（秒）

    badge: Option<String>, // 标记（会员专享等）

    cover: String, // 封面图
}

pub struct BangumiParser<'a> {
    client: &'a BiliClient,
}

impl<'a> BangumiParser<'a> {
    pub fn new(client: &'a BiliClient) -> Self {
        Self { client }
    }

    // 获取番剧信息
    async fn get_bangumi_info(&self, ep_id: &str) -> Result<BangumiInfo, ParseError> {
        let params = HashMap::from([("ep_id".to_string(), ep_id.to_string())]);
        let resp = self
            .client
            .get_auto::<CommonResponse<BangumiInfo>>(
                "https://api.bilibili.com/pgc/view/web/season",
                params,
            )
            .await
            .map_err(|e| ParseError::NetworkError(e.to_string()))?;

        debug!("获取到的番剧信息000");

        let bangumi_info = resp
            .result
            .ok_or_else(|| ParseError::ParseError("未找到番剧信息".to_string()))?;

        debug!("获取到的番剧信息111");
        Ok(bangumi_info)
    }

    // 获取播放地址
    async fn get_play_url(&self, ep_id: &str, cid: i64) -> Result<PlayUrlData, ParseError> {
        let params = HashMap::from([
            ("ep_id".to_string(), ep_id.to_string()),
            ("cid".to_string(), cid.to_string()),
            ("qn".to_string(), "112".to_string()), // 选择合适的清晰度
            ("fnval".to_string(), "16".to_string()), // 启用 DASH
            ("fnver".to_string(), "0".to_string()),
            ("fourk".to_string(), "1".to_string()),
        ]);

        let resp = self
            .client
            .get_auto::<CommonResponse<PlayUrlData>>(
                "https://api.bilibili.com/pgc/player/web/playurl",
                params,
            )
            .await
            .map_err(|e| ParseError::NetworkError(e.to_string()))?;

        // debug!("get_play_url: {:?}", resp);

        // 解析播放地址信息
        resp.result
            .ok_or_else(|| ParseError::ParseError("未找到播放地址信息".to_string()))
    }
}

#[async_trait]
impl<'a> Parser for BangumiParser<'a> {
    async fn parse(&mut self, url_type: &UrlType) -> Result<ParsedMeta, ParseError> {
        // 1. 从URL中提取ep_id
        let url_info = match url_type {
            UrlType::BangumiEpisode(url_info) => url_info,
            _ => return Err(ParseError::InvalidUrl),
        };
        let id = url_info;
        debug!("提取到的ep_id: {}", id);

        // 2. 获取番剧信息
        let info = self.get_bangumi_info(id).await?;
        debug!("获取到的番剧信息: {:?}", info);

        // Store the duration before moving episodes
        let duration = info.episodes.first().map(|ep| ep.duration).unwrap_or(0);

        // 4. 获取当前分集的播放地址
        let ep_id_u64 = id
            .parse::<u64>()
            .map_err(|_| ParseError::ParseError("ep_id 解析失败".to_string()))?;
        let current_ep = info
            .episodes
            .iter()
            .find(|seg| seg.id == ep_id_u64)
            .ok_or_else(|| ParseError::ParseError("未找到指定分集".to_string()))?;

        debug!("当前分集信息: {:?}", current_ep);

        let play_info = self.get_play_url(&id, current_ep.cid).await?;

        debug!("play_info: {:?}", play_info);

        // 选择最高质量的视频和音频流
        if let Some(dash_info) = play_info.dash {
            let video_stream = dash_info
                .video
                .iter()
                .max_by_key(|v| v.quality)
                .ok_or_else(|| ParseError::ParseError("未找到可用的视频流".to_string()))?;

            let audio_stream = dash_info
                .audio
                .iter()
                .max_by_key(|a| a.quality)
                .ok_or_else(|| ParseError::ParseError("未找到可用的音频流".to_string()))?;

            // 构建视频信息
            let video_info = DashVideoInfo {
                url: "https://www.bilibili.com/bangumi/play/".to_string() + &id,
                aid: current_ep.aid,
                bvid: format!("ep{}", ep_id_u64),
                cid: current_ep.cid,
                title: current_ep.title.clone(),
                cover: current_ep.cover.clone(),
                desc: info
                    .media_info
                    .as_ref()
                    .map_or(String::new(), |m| m.type_name.clone()),
                views: String::new(),
                danmakus: String::new(),
                up_name: String::new(),
                up_mid: 0,
                video_quality_id_list: vec![video_stream.quality as i32],
                video_url: video_stream.base_url.clone(),
                audio_url: audio_stream.base_url.clone(),
            };
            return Ok(ParsedMeta {
                title: current_ep.title.clone(),
                stream_type: StreamType::Dash,
                meta: DownloadType::BangumiEpisode(video_info),
            });
        } else if let Some(mp4_info) = play_info.durl {
            let mp4_video_stream = mp4_info[0].clone();

            let video_info = Mp4VideoInfo {
                url: "https://www.bilibili.com/bangumi/play/".to_string() + &id, // 使用课程的URL作为基础
                aid: current_ep.aid.clone(),                             // 使用真实的 aid
                bvid: format!("cheese_{}", ep_id_u64), // 使用课程 ep_id 作为标识
                cid: current_ep.cid,
                title: current_ep.title.clone(),
                cover: current_ep.cover.clone(), // 使用课程封面
                desc: "".to_string(),
                views: String::new(),
                danmakus: String::new(),
                up_name: String::new(),
                up_mid: 0,
                video_url: mp4_video_stream.url,
            };
            Ok(ParsedMeta {
                title: current_ep.title.clone(),
                stream_type: StreamType::MP4,
                meta: DownloadType::CourseChapterMp4(video_info),
            })
        } else {
            Err(ParseError::ParseError("未解析出下载源地址".to_string()))
        }
    }
}

