use std::collections::HashMap;

use crate::common::client::client::BiliClient;
use crate::common::client::models::common::CommonResponse;
use crate::common::models::DownloadType;
use crate::downloader::models::{DownloadTask, FileType};
use crate::parser::detail_parser::Parser;
use crate::parser::detail_parser::models::{DownloadConfig, PlayUrlData};
use crate::parser::detail_parser::parser_trait::{ParserOptions, StreamType, parse_episode_range};
use crate::parser::models::UrlType;
use crate::parser::{ParsedMeta, errors::ParseError};
use async_trait::async_trait;
use serde::de;
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
    async fn get_season_info(
        &self,
        season_id: Option<&str>,
        ep_id: Option<&str>,
    ) -> Result<BangumiInfo, ParseError> {
        let params = match (season_id, ep_id) {
            (Some(sid), None) => HashMap::from([("season_id".to_string(), sid.to_string())]),
            (None, Some(eid)) => HashMap::from([("ep_id".to_string(), eid.to_string())]),
            _ => {
                return Err(ParseError::ParseError(
                    "必须提供 season_id 或 ep_id".to_string(),
                ));
            }
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

        // 检查API返回的错误码
        if resp.code != 0 {
            return match resp.code {
                -403 => Err(ParseError::ParseError(format!(
                    "番剧访问被拒绝（-403）: {}。可能原因：1. 番剧需要大会员权限 2. 地区限制 3. 需要登录",
                    resp.message
                ))),
                -404 => Err(ParseError::ParseError(format!(
                    "番剧不存在（-404）: {}。番剧可能已下架或URL错误",
                    resp.message
                ))),
                -10403 => Err(ParseError::ParseError(format!(
                    "大会员专享番剧（-10403）: {}。此番剧需要大会员权限，请登录大会员账号",
                    resp.message
                ))),
                6001 => Err(ParseError::ParseError(format!(
                    "地区限制（6001）: {}。此番剧在当前地区不可观看",
                    resp.message
                ))),
                _ => Err(ParseError::ParseError(format!(
                    "番剧API返回错误（{}）: {}",
                    resp.code, resp.message
                ))),
            };
        }

        resp.result
            .ok_or_else(|| ParseError::ParseError("API响应中未找到番剧信息".to_string()))
    }

    // 获取播放地址
    async fn get_play_url(&self, ep_id: &str, cid: i64) -> Result<PlayUrlData, ParseError> {
        let params = HashMap::from([
            ("ep_id".to_string(), ep_id.to_string()),
            ("cid".to_string(), cid.to_string()),
            // ("qn".to_string(), (quality as i32).to_string()),
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

        // 检查API返回的错误码
        if resp.code != 0 {
            return match resp.code {
                -403 => Err(ParseError::ParseError(format!(
                    "番剧播放地址获取被拒绝（-403）: {}。可能原因：1. 需要大会员权限 2. Cookie已过期 3. 需要登录",
                    resp.message
                ))),
                -404 => Err(ParseError::ParseError(format!(
                    "番剧播放地址不存在（-404）: {}。番剧可能已下架",
                    resp.message
                ))),
                -10403 => Err(ParseError::ParseError(format!(
                    "大会员专享番剧（-10403）: {}。此番剧需要大会员权限，请登录大会员账号",
                    resp.message
                ))),
                6001 => Err(ParseError::ParseError(format!(
                    "地区限制（6001）: {}。此番剧在当前地区不可观看",
                    resp.message
                ))),
                _ => Err(ParseError::ParseError(format!(
                    "番剧播放地址API返回错误（{}）: {}",
                    resp.code, resp.message
                ))),
            };
        }

        resp.result
            .ok_or_else(|| ParseError::ParseError("API响应中未找到播放地址信息".to_string()))
    }

    // 创建单集视频的元数据
    async fn create_episode_meta(
        &self,
        title: &str,
        episode: &Episode,
        config: &DownloadConfig,
    ) -> Result<Vec<DownloadTask>, ParseError> {
        let play_info = self
            .get_play_url(&episode.id.to_string(), episode.cid)
            .await
            .map_err(|e| ParseError::ParseError(e.to_string()))?;

        let mut download_task_vec: Vec<DownloadTask> = Vec::new();

        let video_stream_task = if config.need_video {
            if let Some(video_url) = play_info.dash.as_ref().and_then(|d| d.video.first()) {
                Some(DownloadTask::new(
                    video_url.base_url.clone(),
                    FileType::Video,
                    format!("{} - {} - {}", title, episode.title, video_url.id),
                    format!("./tmp/{} - {} - {}.mp4", title, episode.title, video_url.id),
                    config.output_dir.clone(),
                    HashMap::new(),
                ))
            } else if let Some(durl_info) = play_info.durls.as_ref().and_then(|d| d.first()) {
                let durl_item = durl_info
                    .durl
                    .first()
                    .ok_or_else(|| ParseError::ParseError("未找到 MP4 流信息".to_string()))?;

                Some(DownloadTask::new(
                    durl_item.url.clone(),
                    FileType::Video,
                    format!("{} - {} - {}.mp4", title, episode.title, durl_item.order),
                    format!(
                        "./tmp/{} - {} - {}.mp4",
                        title, episode.title, durl_item.order
                    ),
                    config.output_dir.clone(),
                    HashMap::from([("desc".to_string(), episode.title.clone())]),
                ))
            } else {
                return Err(ParseError::ParseError(
                    "未找到 DASH 或 MP4 格式的视频流".to_string(),
                ));
            }
        } else {
            None
        };

        let audio_stream_task = if config.need_audio {
            if let Some(audio_url) = play_info.dash.as_ref().and_then(|d| d.audio.first()) {
                Some(DownloadTask::new(
                    audio_url.base_url.clone(),
                    FileType::Audio,
                    format!("{} - {} - {}.m4s", title, episode.title, audio_url.id),
                    format!("./tmp/{} - {} - {}.m4s", title, episode.title, audio_url.id),
                    config.output_dir.clone(),
                    HashMap::from([("desc".to_string(), episode.title.clone())]),
                ))
            } else {
                None
            }
        } else {
            None
        };

        if let Some(video_task) = video_stream_task {
            download_task_vec.push(video_task);
        }
        if let Some(audio_task) = audio_stream_task {
            download_task_vec.push(audio_task);
        }

        Ok(download_task_vec)
    }
}

#[async_trait]
impl<'a> Parser for BangumiParser<'a> {
    async fn parse_with_options(
        &mut self,
        url_type: &UrlType,
        options: ParserOptions,
    ) -> Result<ParsedMeta, ParseError> {
        // 确保传入的是番剧选项
        let config = match options {
            ParserOptions::Bangumi { config } => config,
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

                let download_items = self
                    .create_episode_meta(&bangumi_info.title, &episode, &config)
                    .await
                    .map_err(|e| ParseError::ParseError(e.to_string()))?;

                Ok(ParsedMeta {
                    title: bangumi_info.title.clone(),
                    download_type: DownloadType::Bangumi,
                    download_items: download_items,
                })
            }
            UrlType::BangumiSeason(ss_id) => {
                // 获取番剧信息
                let bangumi_info = self.get_season_info(Some(ss_id), None).await?;
                let episode_range = config.episode_range.as_ref();
                debug!("番剧 {} 共有 {} 集", bangumi_info.title, bangumi_info.total);
                debug!("指定的集数范围: {:?}", episode_range);
                debug!("番剧集数列表: {:?}", bangumi_info.episodes);

                let episodes_to_download = match episode_range {
                    Some(range) => {
                        // 解析要下载的集数范围
                        let episodes = parse_episode_range(&range)?;
                        debug!("解析后的集数范围: {:?}", episodes);
                        // 验证集数是否有效
                        let valid_episodes: Vec<_> = episodes
                            .into_iter()
                            .filter_map(|id| bangumi_info.episodes.get(id as usize - 1))
                            .collect();

                        if valid_episodes.is_empty() {
                            return Err(ParseError::ParseError(
                                "指定的集数范围不在番剧集数列表中".to_string(),
                            ));
                        }

                        valid_episodes
                    }
                    None => {
                        // 如果没有指定范围，则下载所有集数
                        // bangumi_info.episodes.iter().collect()
                        return Err(ParseError::ParseError("未指定集数范围".to_string()));
                    }
                };

                // 获取所有选定集数的元数据
                let mut season_download_tasks: Vec<DownloadTask> = Vec::new();
                for episode in episodes_to_download {
                    debug!("处理集数: {} - {}", episode.id, episode.title);
                    let episode_tasks = self
                        .create_episode_meta(&bangumi_info.title, episode, &config)
                        .await?;
                    debug!("获取到集数 {} 成功", episode.id);
                    season_download_tasks.extend(episode_tasks);
                }

                Ok(ParsedMeta {
                    title: bangumi_info.title.clone(),
                    download_type: DownloadType::Bangumi,
                    download_items: season_download_tasks,
                })
            }
            _ => Err(ParseError::InvalidUrl),
        }
    }
}
