use std::collections::HashMap;

use crate::common::client::client::BiliClient;
use crate::common::client::models::common::CommonResponse;
use crate::common::download_type::video::{VideoInfo, VideoInfoVec};
use crate::common::models::DownloadType;
use crate::parser::detail_parser::Parser;
use crate::parser::detail_parser::models::PlayUrlData;
use crate::parser::detail_parser::parser_trait::{ParserOptions, StreamType, parse_episode_range};
use crate::parser::models::{UrlType, VideoQuality};
use crate::parser::{ParsedMeta, errors::ParseError};
use async_trait::async_trait;
use serde_derive::Deserialize;
use tracing::debug;

// 番剧单集信息响应
#[derive(Debug, Deserialize)]
struct BangumiEpResponse {
    #[serde(rename = "epInfo")]
    ep_info: Option<EpInfo>,
    #[serde(rename = "seasonInfo")]
    season_info: Option<SeasonInfo>,
}

#[derive(Debug, Deserialize)]
struct EpInfo {
    id: u64,       // ep_id
    aid: i64,      // av号
    cid: i64,      // 视频cid
    title: String, // 标题
    #[serde(rename = "seasonId")]
    season_id: u64, // 所属番剧的 season_id
}

#[derive(Debug, Deserialize)]
struct SeasonInfo {
    #[serde(rename = "seasonId")]
    season_id: u64, // season_id
    title: String, // 番剧标题
}

#[derive(Debug, Deserialize)]
struct BangumiSeasonInfo {
    title: String, // 番剧标题
    total: u32,    // 总集数

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
    id: u64,               // ep_id
    aid: i64,              // av号
    cid: i64,              // 视频cid
    title: String,         // 标题
    long_title: String,    // 长标题
    duration: u32,         // 时长（秒）
    badge: Option<String>, // 标记（会员专享等）
    cover: String,         // 封面图
}

#[derive(Debug, Deserialize)]
pub struct BangumiInfo {
    title: String, // 番剧标题
    #[serde(rename = "mediaInfo")]
    media_info: Option<MediaInfo>, // 媒体信息
    episodes: Vec<Episode>, // 集数列表
    total: u32,    // 总集数
}

pub struct BangumiParser<'a> {
    client: &'a BiliClient,
}

impl<'a> BangumiParser<'a> {
    pub fn new(client: &'a BiliClient) -> Self {
        Self { client }
    }

    // 根据 season_id 获取番剧信息
    async fn get_season_info(&self, season_id: Option<&str>, ep_id: Option<&str>) -> Result<BangumiInfo, ParseError> {
        let params =  match (season_id, ep_id) {
            (Some(sid), None) => HashMap::from([("season_id".to_string(), sid.to_string())]),
            (None, Some(eid)) => HashMap::from([("ep_id".to_string(), eid.to_string())]),
            _ => return Err(ParseError::ParseError("必须提供 season_id 或 ep_id".to_string())),
        };

        let resp = self
            .client
            .get_auto::<CommonResponse<BangumiInfo>>(
                "https://api.bilibili.com/pgc/view/web/season",
                params,
            )
            .await
            .map_err(|e| ParseError::NetworkError(e.to_string()))?;

        debug!("获取到的番剧信息: {:?}", resp);

        resp.result
            .ok_or_else(|| ParseError::ParseError("未找到番剧信息".to_string()))
    }

    // 获取播放地址
    async fn get_play_url(
        &self,
        ep_id: &str,
        cid: i64,
        quality: VideoQuality,
    ) -> Result<PlayUrlData, ParseError> {
        let params = HashMap::from([
            ("ep_id".to_string(), ep_id.to_string()),
            ("cid".to_string(), cid.to_string()),
            ("qn".to_string(), (quality as i32).to_string()),
            ("fnval".to_string(), "976".to_string()),
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

        resp.result
            .ok_or_else(|| ParseError::ParseError("未找到播放地址信息".to_string()))
    }

    // 创建单集视频的元数据
    async fn create_episode_meta(
        &self,
        episode: &Episode,
        quality: VideoQuality,
    ) -> Result<ParsedMeta, ParseError> {
        let play_info = self
            .get_play_url(&episode.id.to_string(), episode.cid, quality)
            .await?;

        if let Some(dash_info) = play_info.dash {
            let video_stream = dash_info
                .video
                .iter()
                .max_by_key(|v| v.bandwidth)
                .cloned()
                .ok_or_else(|| ParseError::ParseError("未找到所选质量的视频流".to_string()))?;

            let audio_stream = dash_info
                .audio
                .iter()
                .max_by_key(|a| a.bandwidth)
                .cloned()
                .ok_or_else(|| ParseError::ParseError("未找到可用的音频流".to_string()))?;

            let video_info = VideoInfo {
                url: format!("https://www.bilibili.com/bangumi/play/ep{}", episode.id),
                aid: episode.aid,
                bvid: format!("ep{}", episode.id),
                cid: episode.cid,
                title: if episode.long_title.is_empty() {
                    episode.title.clone()
                } else {
                    format!("{} - {}", episode.title, episode.long_title)
                },
                cover: episode.cover.clone(),
                desc: String::new(),
                views: String::new(),
                danmakus: String::new(),
                up_name: String::new(),
                up_mid: 0,
                stream_type: StreamType::Dash,
                video_quality_id_list: play_info.accept_quality,
                video_url: Some(video_stream.base_url.clone()),
                audio_url: Some(audio_stream.base_url.clone()),
                mp4_url: None, // DASH 不需要 MP4 流
            };

            let video_info_vec = VideoInfoVec(vec![video_info.clone()]);

            Ok(ParsedMeta {
                title: video_info.title.clone(),
                stream_type: StreamType::Dash,
                meta: DownloadType::Bangumi(video_info_vec),
            })
        } else if let Some(durl) = play_info.durl {
            // 如果没有 DASH 信息，使用 MP4 流
            let video_stream = durl
                .iter()
                .max_by_key(|d| d.order)
                .cloned()
                .ok_or_else(|| ParseError::ParseError("未找到 MP4 流".to_string()))?;

            let video_info = VideoInfo {
                url: format!("https://www.bilibili.com/bangumi/play/ep{}", episode.id),
                aid: episode.aid,
                bvid: format!("ep{}", episode.id),
                cid: episode.cid,
                title: if episode.long_title.is_empty() {
                    episode.title.clone()
                } else {
                    format!("{} - {}", episode.title, episode.long_title)
                },
                cover: episode.cover.clone(),
                desc: String::new(),
                views: String::new(),
                danmakus: String::new(),
                up_name: String::new(),
                up_mid: 0,
                stream_type: StreamType::Dash, // You may want to use a different StreamType for MP4
                video_quality_id_list: vec![],
                video_url: None,
                audio_url: None,
                mp4_url: Some(video_stream.url.clone()),
            };

            let video_info_vec = VideoInfoVec(vec![video_info.clone()]);

            Ok(ParsedMeta {
                title: video_info.title.clone(),
                stream_type: StreamType::Dash, // You may want to use a different StreamType for MP4
                meta: DownloadType::Bangumi(video_info_vec),
            })
        } else {
            Err(ParseError::ParseError("未找到播放地址信息".to_string()))
        }
    }

    // // 创建季度视频的元数据
    // async fn create_season_meta(
    //     &self,
    //     episode: &Episode,
    //     bangumi_info: &BangumiInfo,
    //     quality: VideoQuality,
    // ) -> Result<ParsedMeta, ParseError> {
    //     let play_info = self
    //         .get_play_url(&episode.id.to_string(), episode.cid, quality)
    //         .await?;

    //     if let Some(dash_info) = play_info.dash {
    //         let video_stream = dash_info
    //             .video
    //             .iter()
    //             .filter(|v| v.quality <= quality as i32)
    //             .max_by_key(|v| v.quality)
    //             .ok_or_else(|| ParseError::ParseError("未找到所选质量的视频流".to_string()))?;

    //         let audio_stream = dash_info
    //             .audio
    //             .iter()
    //             .max_by_key(|a| a.quality)
    //             .ok_or_else(|| ParseError::ParseError("未找到可用的音频流".to_string()))?;

    //         let video_info = VideoInfo {
    //             url: format!("https://www.bilibili.com/bangumi/play/ep{}", episode.id),
    //             aid: episode.aid,
    //             bvid: format!("ep{}", episode.id),
    //             cid: episode.cid,
    //             title: format!("{} - {}", bangumi_info.title, episode.title),
    //             cover: episode.cover.clone(),
    //             desc: episode.long_title.clone(),
    //             views: String::new(),
    //             danmakus: String::new(),
    //             up_name: String::new(),
    //             up_mid: 0,
    //             video_quality_id_list: vec![video_stream.quality as i32],
    //             video_url: video_stream.base_url.clone(),
    //             audio_url: audio_stream.base_url.clone(),
    //         };

    //         Ok(ParsedMeta {
    //             title: video_info.title.clone(),
    //             stream_type: StreamType::Dash,
    //             meta: DownloadType::BangumiEpisodeDash(video_info),
    //         })
    //     } else {
    //         Err(ParseError::ParseError(
    //             "未找到 DASH 格式的视频流".to_string(),
    //         ))
    //     }
    // }
}

#[async_trait]
impl<'a> Parser for BangumiParser<'a> {
    async fn parse_with_options(
        &mut self,
        url_type: &UrlType,
        options: ParserOptions,
    ) -> Result<ParsedMeta, ParseError> {
        // 确保传入的是番剧选项
        let (quality, episode_range) = match options {
            ParserOptions::Bangumi {
                quality,
                episode_range,
            } => (quality, episode_range),
            _ => return Err(ParseError::ParseError("无效的番剧解析选项".to_string())),
        };

        match url_type {
            UrlType::BangumiEpisode(ep_id) => {
                // 单集下载，直接获取单集信息
                let bangumi_info = self.get_season_info(None, Some(ep_id)).await?;
                debug!("番剧 {} 共有 {} 集", bangumi_info.title, bangumi_info.total);
                let episode = bangumi_info
                    .episodes
                    .iter()
                    .find(|ep| ep.id == (*ep_id).parse::<u64>().unwrap_or(0))
                    .ok_or_else(|| ParseError::ParseError("未找到指定的番剧集数".to_string()))?;
                let meta = self.create_episode_meta(&episode, quality).await?;
                Ok(meta)
            }
            // UrlType::BangumiSeason(ss_id) => {
            //     // 获取番剧信息
            //     let bangumi_info = self.get_season_info(ss_id).await?;
            //     debug!("番剧 {} 共有 {} 集", bangumi_info.title, bangumi_info.total);

            //     let episodes_to_download = match episode_range {
            //         Some(range) => {
            //             // 解析要下载的集数范围
            //             let episodes = parse_episode_range(&range)?;

            //             // 验证集数是否有效
            //             let valid_episodes: Vec<_> = episodes
            //                 .into_iter()
            //                 .filter_map(|ep_id| {
            //                     bangumi_info
            //                         .episodes
            //                         .iter()
            //                         .find(|ep| ep.id as i64 == ep_id)
            //                 })
            //                 .collect();

            //             if valid_episodes.is_empty() {
            //                 return Err(ParseError::ParseError(
            //                     "指定的集数范围不在番剧集数列表中".to_string(),
            //                 ));
            //             }

            //             valid_episodes
            //         }
            //         None => {
            //             // 如果没有指定范围，则下载所有集数
            //             bangumi_info.episodes.iter().collect()
            //         }
            //     };

            //     // 获取所有选定集数的元数据
            //     let mut download_type = Vec::new();
            //     for episode in episodes_to_download {
            //         match self
            //             .create_season_meta(episode, &bangumi_info, quality)
            //             .await
            //         {
            //             Ok(meta) => download_type.push(meta),
            //             Err(e) => {
            //                 debug!("获取第 {} 集信息失败: {:?}", episode.id, e);
            //                 continue;
            //             }
            //         }
            //     }

            //     let metas = ParsedMeta {
            //         title: bangumi_info.title.clone(),
            //         stream_type: StreamType::Dash,
            //         meta: download_type,
            //     };
            //     Ok(metas)
            // }
            _ => Err(ParseError::InvalidUrl),
        }
    }
}
