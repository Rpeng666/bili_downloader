use async_trait::async_trait;
use std::collections::HashMap;
use tracing::debug;

use crate::common::models::{DownloadType, ParsedMeta};
use crate::common::{client::client::BiliClient, client::models::common::CommonResponse};
use crate::downloader::models::DownloadTask;
use crate::parser::detail_parser::error_utils::handle_api_error;
use crate::parser::detail_parser::models::{CourseEpisode, CourseInfo, DownloadConfig};
use crate::parser::detail_parser::parser_trait::{ParserOptions, parse_episode_range};
use crate::parser::detail_parser::stream_utils::{select_audio_stream, select_video_stream};
use crate::parser::detail_parser::task_utils::{create_audio_task, create_video_task};
use crate::parser::models::UrlType;
use crate::parser::{
    detail_parser::{Parser, models::PlayUrlData},
    errors::ParseError,
};

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
            return Err(handle_api_error(resp.code, &resp.message, "课程"));
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
            return Err(handle_api_error(resp.code, &resp.message, "课程播放地址"));
        }

        resp.data
            .ok_or_else(|| ParseError::ParseError("API响应中未找到播放地址信息".to_string()))
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
            select_video_stream(&play_info.dash.as_ref().unwrap().video, config.resolution)?
                .map(|video_url| {
                    create_video_task(
                        video_url,
                        title,
                        Some(&episode.title),
                        &config.output_dir,
                        HashMap::new(),
                    )
                })
        } else {
            None
        };

        // --------------------------------------------------------------------
        let audio_stream_task = if config.need_audio && play_info.dash.is_some() {
            select_audio_stream(&play_info.dash.as_ref().unwrap().audio)?
                .map(|audio_url| {
                    create_audio_task(
                        audio_url,
                        title,
                        Some(&episode.title),
                        &config.output_dir,
                        HashMap::new(),
                    )
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
