use std::collections::HashMap;

use crate::common::models::{DownloadType, ParsedMeta};
use crate::common::{client::client::BiliClient, client::models::common::CommonResponse};
use crate::downloader::models::{DownloadTask, FileType};
use crate::parser::detail_parser::models::{CourseEpisode, CourseInfo, DashItem, DownloadConfig};
use crate::parser::detail_parser::parser_trait::{ParserOptions, parse_episode_range};
use crate::parser::models::{UrlType, VideoQuality};
use crate::parser::{
    detail_parser::{Parser, models::PlayUrlData},
    errors::ParseError,
};
use async_trait::async_trait;
use tracing::{debug, info, warn};

pub struct CourseParser<'a> {
    client: &'a BiliClient,
}

impl<'a> CourseParser<'a> {
    pub fn new(client: &'a BiliClient) -> Self {
        Self { client }
    }

    // 获取课程信息
    async fn get_course_info(
        &self,
        season_id: Option<&str>,
        ep_id: Option<&str>,
    ) -> Result<CourseInfo, ParseError> {
        let params = match (season_id, ep_id) {
            (Some(sid), _) => HashMap::from([(String::from("season_id"), sid.to_string())]),
            (_, Some(eid)) => HashMap::from([(String::from("ep_id"), eid.to_string())]),
            _ => {
                return Err(ParseError::ParseError(
                    "需要提供 season_id 或 ep_id".to_string(),
                ));
            }
        };

        let resp = self
            .client
            .get_auto::<CommonResponse<CourseInfo>>(
                "https://api.bilibili.com/pugv/view/web/season",
                params,
            )
            .await
            .map_err(|e| ParseError::NetworkError(e.to_string()))?;

        debug!("get_course_info resp: {:?}", resp);

        // 检查API返回的错误码
        if resp.code != 0 {
            return match resp.code {
                -403 => Err(ParseError::ParseError(format!(
                    "课程访问被拒绝（-403）: {}。可能原因：1. 课程需要购买 2. 需要登录 3. 权限不足",
                    resp.message
                ))),
                -404 => Err(ParseError::ParseError(format!(
                    "课程不存在（-404）: {}。课程可能已下架或URL错误",
                    resp.message
                ))),
                -500 => Err(ParseError::ParseError(format!(
                    "课程访问限制（-500）: {}。课程可能需要购买或特定权限",
                    resp.message
                ))),
                _ => Err(ParseError::ParseError(format!(
                    "课程API返回错误（{}）: {}",
                    resp.code, resp.message
                ))),
            };
        }

        resp.data
            .ok_or_else(|| ParseError::ParseError("API响应中未找到课程信息".to_string()))
    }

    // 获取播放地址
    async fn get_play_url(
        &self,
        ep_id: i64,
        aid: i64,
        cid: i64,
    ) -> Result<PlayUrlData, ParseError> {
        let params = HashMap::from([
            (String::from("avid"), aid.to_string()),
            (String::from("cid"), cid.to_string()),
            (String::from("ep_id"), ep_id.to_string()),
            (String::from("qn"), String::from("116")), // 画质参数
            (String::from("fnver"), String::from("0")), // 固定值
            (String::from("fnval"), String::from("976")), // 固定值
            (String::from("fourk"), String::from("1")),
        ]);

        let resp = self
            .client
            .get_auto::<CommonResponse<PlayUrlData>>(
                "https://api.bilibili.com/pugv/player/web/playurl",
                params,
            )
            .await
            .map_err(|e| ParseError::NetworkError(e.to_string()))?;

        // 检查API返回的错误码
        if resp.code != 0 {
            return match resp.code {
                -403 => Err(ParseError::ParseError(format!(
                    "课程播放地址获取被拒绝（-403）: {}。可能原因：1. 课程需要购买 2. Cookie已过期 3. 需要登录",
                    resp.message
                ))),
                -404 => Err(ParseError::ParseError(format!(
                    "课程播放地址不存在（-404）: {}。课程可能已下架",
                    resp.message
                ))),
                -500 => Err(ParseError::ParseError(format!(
                    "课程播放限制（-500）: {}。课程可能需要购买或特定权限",
                    resp.message
                ))),
                _ => Err(ParseError::ParseError(format!(
                    "课程播放地址API返回错误（{}）: {}",
                    resp.code, resp.message
                ))),
            };
        }

        resp.data
            .ok_or_else(|| ParseError::ParseError("API响应中未找到播放地址信息".to_string()))
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
            debug!(
                "流 {}: 清晰度ID={}, width={:?}, height={:?}",
                i, stream.id, stream.width, stream.height
            );
        }

        let target_quality_id = resolution as i32;
        debug!("目标清晰度ID: {}", target_quality_id);

        // 首先尝试精确匹配清晰度ID
        if let Some(stream) = streams.iter().find(|s| s.id == target_quality_id) {
            debug!("找到精确匹配的清晰度: ID={}", stream.id);
            return Ok(Some(stream.base_url.clone()));
        }

        // 如果没有精确匹配，选择最接近且不超过目标清晰度的流
        let mut suitable_streams: Vec<_> = streams
            .iter()
            .filter(|s| s.id <= target_quality_id)
            .collect();

        if !suitable_streams.is_empty() {
            // 按清晰度ID降序排序，选择最高的
            suitable_streams.sort_by(|a, b| b.id.cmp(&a.id));
            let selected = suitable_streams[0];
            debug!(
                "选择最接近的清晰度: ID={} (目标: {})",
                selected.id, target_quality_id
            );
            return Ok(Some(selected.base_url.clone()));
        }

        // 如果所有流的清晰度都高于目标，选择最低的
        let mut all_streams = streams.to_vec();
        all_streams.sort_by(|a, b| a.id.cmp(&b.id));
        let fallback = &all_streams[0];

        // 检查是否是高质量视频权限问题
        let highest_available_quality = all_streams.last().map(|s| s.id).unwrap_or(0);
        if target_quality_id >= 112 && highest_available_quality < target_quality_id {
            // 112是1080P+
            warn!(
                "目标清晰度 {} 可能需要大会员权限，最高可用清晰度: {}",
                target_quality_id, highest_available_quality
            );
            warn!("💡 提示：1080P+、4K等高清晰度通常需要大会员权限，请确保已登录大会员账号");
        }

        debug!("目标清晰度过低，降级到最低可用清晰度: ID={}", fallback.id);

        Ok(Some(fallback.base_url.clone()))
    }

    // 根据单集课程信息创建视频元数据
    async fn create_video_meta(
        &self,
        title: &str,
        episode: &CourseEpisode,
        config: &DownloadConfig,
    ) -> Result<Vec<DownloadTask>, ParseError> {
        let play_info = self
            .get_play_url(episode.id, episode.aid, episode.cid)
            .await?;

        let mut download_task_vec: Vec<DownloadTask> = Vec::new();

        // --------------------------------------------------------------------
        let video_stream_task = if config.need_video && play_info.dash.is_some() {
            self.select_video_stream(&play_info.dash.as_ref().unwrap().video, config.resolution)?
                .map(|video_url| {
                    DownloadTask::new(
                        video_url,
                        FileType::Video,
                        format!("{} - {}.mp4", title, episode.title),
                        format!("./tmp/{}-{}.mp4", title, episode.title),
                        config.output_dir.clone(),
                        HashMap::new(),
                    )
                })
        } else {
            None
        };

        // --------------------------------------------------------------------
        let audio_stream_task = if config.need_audio && play_info.dash.is_some() {
            // 如果需要音频且有 DASH 流
            let audio_url = play_info
                .dash
                .as_ref()
                .and_then(|d| d.audio.first())
                .ok_or_else(|| ParseError::ParseError("未找到音频流".to_string()))?;
            // 构建下载任务
            Some(DownloadTask {
                url: audio_url.base_url.clone(),
                file_type: FileType::Audio,
                name: format!("{} - {}.m4s", title, episode.title),
                output_path: format!("./tmp/{}-{}.m4s", title, episode.title),
                temp_path: config.output_dir.clone(),
                metadata: HashMap::new(),
            })
        } else {
            None
        };

        if let Some(video_task) = &video_stream_task {
            download_task_vec.push(video_task.clone());
        }

        if let Some(audio_task) = &audio_stream_task {
            download_task_vec.push(audio_task.clone());
        }

        Ok(download_task_vec)
    }
}

#[async_trait]
impl<'a> Parser for CourseParser<'a> {
    async fn parse_with_options(
        &mut self,
        url_type: &UrlType,
        options: ParserOptions,
    ) -> Result<ParsedMeta, ParseError> {
        debug!("开始解析----------------------");
        debug!("开始解析课程信息: {:?}", url_type);
        debug!("解析选项: {:?}", options);
        // 确保传入的是课程选项
        let config = match options {
            ParserOptions::Course { config } => config,
            _ => return Err(ParseError::ParseError("无效的课程解析选项".to_string())),
        };

        match url_type {
            UrlType::CourseEpisode(ep_id) => {
                // 处理单集课程
                let course_info = self.get_course_info(None, Some(ep_id)).await?;
                debug!(
                    "课程 {} 共有 {} 集",
                    course_info.title,
                    course_info.episodes.len()
                );
                let episode = course_info
                    .episodes
                    .iter()
                    .find(|ep| ep.id.to_string() == *ep_id)
                    .ok_or_else(|| ParseError::ParseError("未找到章节信息".to_string()))?;

                let download_itmes = self
                    .create_video_meta(&course_info.title, &episode, &config)
                    .await?;

                Ok(ParsedMeta {
                    title: course_info.title.clone(),
                    download_items: download_itmes,
                    download_type: DownloadType::Course,
                })
            }
            UrlType::CourseSeason(ss_id) => {
                // 获取课程信息
                let course_info = self.get_course_info(Some(ss_id), None).await?;
                debug!(
                    "课程 {} 共有 {} 集",
                    course_info.title,
                    course_info.episodes.len()
                );
                debug!("指定的集数范围: {:?}", config.episode_range);
                debug!("课程提供的集数范围: {:?}", course_info.episodes);

                let episodes_to_download = match &config.episode_range {
                    Some(range) => {
                        // 解析要下载的集数范围
                        let episodes = parse_episode_range(range)?;

                        // 验证集数是否有效
                        let valid_episodes: Vec<_> = episodes
                            .into_iter()
                            .filter_map(|id| course_info.episodes.get(id as usize - 1))
                            .collect();

                        if valid_episodes.is_empty() {
                            return Err(ParseError::ParseError(
                                "指定的集数范围不在课程集数列表中".to_string(),
                            ));
                        }

                        valid_episodes
                    }
                    None => {
                        return Err(ParseError::ParseError("未指定集数范围".to_string()));
                    }
                };

                let mut download_items: Vec<DownloadTask> = Vec::new();
                for episode in episodes_to_download {
                    let tasks = self
                        .create_video_meta(&course_info.title, &episode, &config)
                        .await?;
                    download_items.extend(tasks);
                }

                Ok(ParsedMeta {
                    title: course_info.title.clone(),
                    download_items,
                    download_type: DownloadType::Course,
                })
            }
            _ => Err(ParseError::InvalidUrl),
        }
    }
}
