use std::collections::HashMap;

use crate::common::{
    client::client::BiliClient, client::models::common::CommonResponse,
    download_type::dash::DashVideoInfo, download_type::mp4::Mp4VideoInfo, models::DownloadType,
};
use crate::parser::detail_parser::models::{CourseEpisode, CourseInfo};
use crate::parser::detail_parser::parser_trait::{ParserOptions, parse_episode_range};
use crate::parser::{
    detail_parser::{Parser, models::PlayUrlData},
    errors::ParseError,
    models::{ParsedMeta, StreamType, UrlType},
};
use async_trait::async_trait;
use tracing::{debug, info};

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

    // 处理单集课程
    async fn handle_episode(&self, ep_id: &str) -> Result<ParsedMeta, ParseError> {
        let course_info = self.get_course_info(None, Some(ep_id)).await?;

        // 从课程列表中找到当前集
        let episode = course_info
            .episodes
            .iter()
            .find(|ep| ep.id.to_string() == ep_id)
            .ok_or_else(|| ParseError::ParseError("未找到章节信息".to_string()))?;

        self.create_video_meta(&course_info, episode).await
    }

    // 处理整季课程
    pub async fn get_season_info(&self, season_id: &str) -> Result<CourseInfo, ParseError> {
        self.get_course_info(Some(season_id), None).await
    }

    // 处理选定的课程集数
    pub async fn handle_selected_episodes(
        &self,
        season_id: &str,
        ep_ids: &[i64],
    ) -> Result<Vec<ParsedMeta>, ParseError> {
        let course_info = self.get_course_info(Some(season_id), None).await?;

        let mut results = Vec::new();
        for ep_id in ep_ids {
            if let Some(episode) = course_info.episodes.iter().find(|ep| ep.id == *ep_id) {
                match self.create_video_meta(&course_info, episode).await {
                    Ok(meta) => results.push(meta),
                    Err(e) => debug!("处理章节 {} 失败: {:?}", ep_id, e),
                }
            }
        }

        if results.is_empty() {
            Err(ParseError::ParseError(
                "没有成功解析任何选中的章节".to_string(),
            ))
        } else {
            Ok(results)
        }
    }

    // 根据课程信息创建视频元数据
    async fn create_video_meta(
        &self,
        course_info: &CourseInfo,
        episode: &CourseEpisode,
        quality: VideoQuality,
    ) -> Result<ParsedMeta, ParseError> {
        let play_info = self
            .get_play_url(episode.id, episode.aid, episode.cid)
            .await?;

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

            // 构建视频信息
            let video_info = DashVideoInfo {
                url: format!("https://www.bilibili.com/cheese/play/ep{}", episode.id),
                aid: episode.aid,
                bvid: format!("cheese_{}", episode.id),
                cid: episode.cid,
                title: format!("{} - {}", course_info.title, episode.title),
                cover: course_info.cover.clone(),
                desc: "".to_string(),
                views: String::new(),
                danmakus: String::new(),
                up_name: String::new(),
                up_mid: 0,
                video_quality_id_list: vec![video_stream.quality as i32],
                video_url: video_stream.base_url.clone(),
                audio_url: audio_stream.base_url.clone(),
            };

            Ok(ParsedMeta {
                title: format!("{} - {}", course_info.title, episode.title),
                stream_type: StreamType::Dash,
                meta: DownloadType::CourseChapterDash(video_info),
            })
        } else if let Some(mp4_info) = play_info.durl {
            let mp4_video_stream = mp4_info
                .iter()
                .find(|s| s.order == 1)
                .ok_or_else(|| ParseError::ParseError("未找到可用的视频流".to_string()))?;

            // 构建视频信息
            let video_info = Mp4VideoInfo {
                url: format!("https://www.bilibili.com/cheese/play/ep{}", episode.id),
                aid: episode.aid,
                bvid: format!("cheese_{}", episode.id),
                cid: episode.cid,
                title: format!("{} - {}", course_info.title, episode.title),
                cover: course_info.cover.clone(),
                desc: "".to_string(),
                views: String::new(),
                danmakus: String::new(),
                up_name: String::new(),
                up_mid: 0,
                video_url: mp4_video_stream.url.clone(),
            };

            Ok(ParsedMeta {
                title: format!("{} - {}", course_info.title, episode.title),
                stream_type: StreamType::MP4,
                meta: DownloadType::CourseChapterMp4(video_info),
            })
        } else {
            Err(ParseError::ParseError("未解析出下载源地址".to_string()))
        }
    }
}

#[async_trait]
impl<'a> Parser for CourseParser<'a> {
    async fn parse(&mut self, url_type: &UrlType) -> Result<ParsedMeta, ParseError> {
        match url_type {
            UrlType::CourseEpisode(ep_id) => self.handle_episode(ep_id).await,
            UrlType::CourseSeason(ss_id) => {
                // 对于整季，需要先获取课程信息并展示
                let course_info = self.get_season_info(ss_id).await?;
                info!(
                    "课程 {} 共有 {} 集",
                    course_info.title,
                    course_info.episodes.len()
                );

                // 这里我们返回第一集作为示例
                // TODO: 在实际应用中，你需要：
                // 1. 展示所有集数信息给用户
                // 2. 让用户选择要下载哪些集
                // 3. 使用 handle_selected_episodes 方法下载选中的集数
                let first_episode = course_info
                    .episodes
                    .first()
                    .ok_or_else(|| ParseError::ParseError("课程列表为空".to_string()))?;

                self.create_video_meta(&course_info, first_episode).await
            }
            _ => Err(ParseError::InvalidUrl),
        }
    }

    async fn parse_with_options(
        &mut self,
        url_type: &UrlType,
        options: ParserOptions,
    ) -> Result<ParsedMeta, ParseError> {
        // 确保传入的是课程选项
        let (quality, episode_range) = match options {
            ParserOptions::Course {
                quality,
                episode_range,
            } => (quality, episode_range),
            _ => return Err(ParseError::ParseError("无效的课程解析选项".to_string())),
        };

        match url_type {
            UrlType::CourseEpisode(ep_id) => {
                let course_info = self.get_course_info(None, Some(ep_id)).await?;

                let episode = course_info
                    .episodes
                    .iter()
                    .find(|ep| ep.id.to_string() == *ep_id)
                    .ok_or_else(|| ParseError::ParseError("未找到章节信息".to_string()))?;

                self.create_video_meta(&course_info, episode, quality).await
            }
            UrlType::CourseSeason(ss_id) => {
                // 获取课程信息
                let course_info = self.get_course_info(Some(ss_id), None).await?;
                debug!(
                    "课程 {} 共有 {} 集",
                    course_info.title,
                    course_info.episodes.len()
                );

                match episode_range {
                    Some(range) => {
                        // 解析要下载的集数范围
                        let episodes = parse_episode_range(&range)?;

                        // 验证集数是否有效
                        let valid_episodes: Vec<_> = episodes
                            .into_iter()
                            .filter_map(|ep_id| {
                                course_info.episodes.iter().find(|ep| ep.id == ep_id)
                            })
                            .collect();

                        if valid_episodes.is_empty() {
                            return Err(ParseError::ParseError(
                                "指定的集数范围不在课程集数列表中".to_string(),
                            ));
                        }

                        // 获取第一个有效集数的信息来返回
                        let first_episode = &valid_episodes[0];
                        self.create_video_meta(&course_info, first_episode, quality)
                            .await
                    }
                    None => {
                        // 如果没有指定范围，则返回第一集的信息
                        let first_episode = course_info
                            .episodes
                            .first()
                            .ok_or_else(|| ParseError::ParseError("课程列表为空".to_string()))?;

                        self.create_video_meta(&course_info, first_episode, quality)
                            .await
                    }
                }
            }
            _ => Err(ParseError::InvalidUrl),
        }
    }
}
