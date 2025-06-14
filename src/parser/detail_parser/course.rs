use std::collections::HashMap;

use crate::common::client::client::BiliClient;
use crate::common::client::models::common::CommonResponse;
use crate::common::download_type::dash::DashVideoInfo;
use crate::common::download_type::mp4::Mp4VideoInfo;
use crate::common::models::DownloadType;
use crate::parser::detail_parser::Parser;
use crate::parser::detail_parser::models::PlayUrlResponse;
use crate::parser::models::{StreamType, UrlType};
use crate::parser::{errors::ParseError, models::ParsedMeta};
use async_trait::async_trait;
use serde_derive::Deserialize;
use tracing::debug;

#[derive(Debug, Deserialize)]
struct CourseInfo {
    title: String,

    season_id: i64,

    cover: String,

    paid_view: bool, // 购买状态

    episodes: Vec<Episode>,
}

#[derive(Debug, Deserialize)]
struct Episode {
    id: String,
    aid: i64,
    cid: i64,
    duration: i32,
    title: String,
    release_date: i64,
}

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
        seanson_id: &str,
        ep_id: &str,
    ) -> Result<CourseInfo, ParseError> {
        let params = if !seanson_id.is_empty() {
            HashMap::from([("season_id".to_string(), seanson_id.to_string())])
        } else if !ep_id.is_empty() {
            HashMap::from([("ep_id".to_string(), ep_id.to_string())])
        } else {
            HashMap::new()
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

        let data = resp
            .data
            .ok_or_else(|| ParseError::ParseError("未找到课程信息".to_string()))
            .map_err(|e| ParseError::ParseError(e.to_string()))?;

        // // 检查购买状态
        // if data.paid_view {
        //     return Err(ParseError::PaymentRequired);
        // }

        Ok(data)
    }

    // 获取播放地址
    async fn get_play_url(
        &self,
        epid: String,
        aid: i64,
        cid: i64,
    ) -> Result<PlayUrlResponse, ParseError> {
        let params = HashMap::from([
            ("avid".to_string(), aid.to_string()),
            ("cid".to_string(), cid.to_string()),
            ("ep_id".to_string(), epid),
            ("qn".to_string(), "116".to_string()),  // 画质参数
            ("fnver".to_string(), "0".to_string()), // 固定值
            ("fnval".to_string(), "976".to_string()), // 固定值
            ("fourk".to_string(), "1".to_string()),
        ]);

        let resp = self
            .client
            .get_auto::<CommonResponse<PlayUrlResponse>>(
                "https://api.bilibili.com/pugv/player/web/playurl",
                params,
            )
            .await
            .map_err(|e| ParseError::NetworkError(e.to_string()))?;

        resp.data
            .ok_or_else(|| ParseError::ParseError("未找到播放地址信息".to_string()))
    }
}

#[async_trait]
impl<'a> Parser for CourseParser<'a> {
    async fn parse(&mut self, url_type: &UrlType) -> Result<ParsedMeta, ParseError> {
        let url_info = match url_type {
            UrlType::CourseChapterDash(url_info) => url_info,
            _ => return Err(ParseError::InvalidUrl),
        };
        // 1. 从URL中提取ep_id
        let id = url_info;

        debug!("获取到的id: {}", id);

        let season_id = id.split("ss").nth(1).unwrap_or("");

        let ep_id = id.split("ep").nth(1).unwrap_or("");

        debug!("获取到的season_id: {:?}, ep_id: {:?}", season_id, ep_id);

        // 2. 获取课程信息
        let course_info = self.get_course_info(season_id, ep_id).await?;

        debug!("course_info: {:?}", course_info);

        // 3. 获取课程章节列表
        let episodes = course_info.episodes;

        // 5. 获取当前章节的播放地址
        let episode_info = episodes
            .iter()
            .find(|c| c.id.to_string() == ep_id)
            .ok_or_else(|| ParseError::ParseError("未找到章节信息".to_string()))?;

        let play_info = self
            .get_play_url(
                episode_info.id.clone(),
                episode_info.aid.clone(),
                episode_info.cid.clone(),
            )
            .await?;

        let play_info_data = play_info
            .data
            .ok_or_else(|| ParseError::ParseError("xx".to_string()))?;

        if let Some(dash_info) = play_info_data.dash {
            // 选择最高质量的视频流和音频流
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
                url: "https://www.bilibili.com/cheese/".to_string() + &id, // 使用课程的URL作为基础
                aid: episode_info.aid,                                     // 使用真实的 aid
                bvid: format!("cheese_{}", ep_id), // 使用课程 ep_id 作为标识
                cid: episode_info.cid.clone(),
                title: format!("{} - {}", course_info.title, episode_info.title),
                cover: course_info.cover, // 使用课程封面
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
                title: course_info.title,
                stream_type: StreamType::Dash,
                meta: DownloadType::CourseChapterDash(video_info),
            })
        } else if let Some(mp4_info) = play_info_data.durl {
            let mp4_video_stream = mp4_info[0].clone();

            let video_info = Mp4VideoInfo {
                url: "https://www.bilibili.com/cheese/".to_string() + &id, // 使用课程的URL作为基础
                aid: episode_info.aid.clone(),                             // 使用真实的 aid
                bvid: format!("cheese_{}", ep_id), // 使用课程 ep_id 作为标识
                cid: episode_info.cid.clone(),
                title: format!("{} - {}", course_info.title, episode_info.title),
                cover: course_info.cover, // 使用课程封面
                desc: "".to_string(),
                views: String::new(),
                danmakus: String::new(),
                up_name: String::new(),
                up_mid: 0,
                video_url: mp4_video_stream.url,
            };
            Ok(ParsedMeta {
                title: course_info.title,
                stream_type: StreamType::MP4,
                meta: DownloadType::CourseChapterMp4(video_info),
            })
        } else {
            Err(ParseError::ParseError("未解析出下载源地址".to_string()))
        }
    }
}

#[derive(Debug, Deserialize)]
struct ChapterListResponse {
    data: ChapterListData,
}

#[derive(Debug, Deserialize)]
struct ChapterListData {
    chapters: Vec<Episode>,
}
