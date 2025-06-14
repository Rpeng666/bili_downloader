use crate::common::client::client::BiliClient;
use crate::common::client::models::common::CommonResponse;
use crate::common::download_type::dash::DashVideoInfo;
use crate::common::models::DownloadType;
use crate::parser::detail_parser::Parser;
use crate::parser::detail_parser::models::PlayUrlData;
use crate::parser::errors::ParseError;
use crate::parser::models::{ParsedMeta, StreamType, UrlType};

use async_trait::async_trait;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use tracing::{debug, info};

pub struct CommonVideoParser<'a> {
    client: &'a BiliClient,
    part: Option<String>,
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
}

impl<'a> CommonVideoParser<'a> {
    pub fn new(client: &'a BiliClient) -> Self {
        Self { client, part: None }
    }

    // 解析分P信息
    fn get_part(&mut self, url: &str) {
        let re = Regex::new(r"p=([0-9]+)").unwrap();
        self.part = re
            .captures(url)
            .and_then(|cap| cap.get(1))
            .and_then(|m| m.as_str().parse().ok());
    }

    // 从av号获取bvid
    fn get_aid(&mut self, url: &str) -> Result<(), ParseError> {
        let re = Regex::new(r"av([0-9]+)").unwrap();
        let aid = re
            .captures(url)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str())
            .ok_or(ParseError::InvalidUrl)?;

        let bvid = self.aid_to_bvid(aid.parse()?);
        self.set_bvid(&bvid)?;
        Ok(())
    }

    // 获取BV号
    fn get_bvid(&mut self, url: &str) -> Result<(), ParseError> {
        let re = Regex::new(r"BV\w+").unwrap();
        let bvid = re
            .find(url)
            .map(|m| m.as_str())
            .ok_or(ParseError::InvalidUrl)?;

        self.set_bvid(bvid)?;
        Ok(())
    }

    // 设置bvid和url
    fn set_bvid(&mut self, bvid: &str) -> Result<(String, String), ParseError> {
        let url = format!("https://www.bilibili.com/video/{}", bvid);
        Ok((bvid.to_string(), url.clone()))
    }

    // 获取视频信息
    pub async fn __get_video_info(&mut self, bvid: String) -> Result<VideoInfo, ParseError> {
        let params = HashMap::from([("bvid".to_string(), bvid.clone())]);

        let resp = self
            .client
            .get_auto::<CommonResponse<CommonVideoInfo>>(
                "https://api.bilibili.com/x/web-interface/wbi/view",
                params,
            )
            .await?;

        let mut video_info = VideoInfo::default();
        let data = resp
            .data
            .ok_or_else(|| ParseError::ParseError("未找到数据".to_string()))?;

        if let Some(ref redirect_url) = data.redirect_url {
            // 处理重定向

            return Err(ParseError::Redirect(redirect_url.clone()));
        } else {
            video_info.title = data.title.clone();
            video_info.cover = data.pic.clone();
            video_info.desc = data.desc.clone();
            video_info.up_name = data.owner.name.clone();
            video_info.up_mid = data.owner.mid.clone();
            video_info.cid = data.cid.clone();
            video_info.bvid = data.bvid.clone();
        }

        Ok(video_info)
    }

    // 获取媒体信息
    async fn __get_play_url(&mut self, video_info: &VideoInfo) -> Result<DownloadType, ParseError> {
        let params = HashMap::from([
            ("bvid".to_string(), video_info.bvid.clone()),
            ("cid".to_string(), video_info.cid.to_string()),
            ("qn".to_string(), "0".to_string()), // 0表示自动选择清晰度
            ("fnver".to_string(), "0".to_string()), // 版本号
            ("fnval".to_string(), "16".to_string()), // 流格式
            ("otype".to_string(), "json".to_string()), // 输出格式
            ("platform".to_string(), "web".to_string()), // 平台
            ("type".to_string(), "mp4".to_string()), // 视频类型
            ("otype".to_string(), "json".to_string()), // 输出格式
        ]);

        let resp = self
            .client
            .get_auto::<CommonResponse<PlayUrlData>>(
                "https://api.bilibili.com/x/player/wbi/playurl",
                params,
            )
            .await?;

        let download_info = resp
            .data
            .ok_or_else(|| ParseError::ParseError("未找到数据".to_string()))?;

        if let Some(dash) = download_info.dash {
            let stream_type = StreamType::Dash;
            info!("检测到Dash流地址");

            let mut selected_video_url = String::from("");
            // 解析视频流
            // 选择最高质量的视频流
            // 测试的时候，选择最低质量的视频流
            if let Some(best_video) = dash.video.iter().min_by_key(|v| v.bandwidth) {
                selected_video_url = best_video.base_url.clone();
            }

            let mut selected_audio_url = String::from("");
            // 解析音频流
            // 选择最高质量的音频流
            // 测试的时候，选择最低质量的音频流
            if let Some(best_audio) = dash.audio.iter().min_by_key(|a| a.bandwidth) {
                selected_audio_url = best_audio.base_url.clone();
            }

            return Ok(DownloadType::CommonVideo(DashVideoInfo {
                url: video_info.url.clone(),
                aid: video_info.aid.clone(),
                bvid: video_info.bvid.clone(),
                cid: video_info.cid.clone(),
                title: video_info.title.clone(),
                cover: video_info.cover.clone(),
                desc: video_info.desc.clone(),
                views: video_info.views.clone(),
                danmakus: video_info.danmakus.clone(),
                up_name: video_info.up_name.clone(),
                up_mid: video_info.up_mid.clone(),
                video_quality_id_list: video_info.video_quality_id_list.clone(),
                video_url: selected_video_url,
                audio_url: selected_audio_url,
            }));
        } else if let Some(durl) = download_info.durl {
            let stream_type = StreamType::MP4;

            let mut selected_video_url = String::from("");
            // 对于 MP4 流，直接使用 durl 中的 URL
            if let Some(first_url) = durl.first() {
                selected_video_url = first_url.url.clone();
            }

            return Err(ParseError::ApiError("暂不支持的类型, 后续更新".to_string()));
        } else {
            return Err(ParseError::ParseError("未找到数据源".to_string()));
        }
    }

    pub fn aid_to_bvid(&self, aid: i64) -> String {
        const XOR_CODE: i64 = 23442827791579;
        const MAX_AID: i64 = 1 << 51;
        const ALPHABET: &str = "FcwAPNKTMug3GV5Lj7EJnHpWsx4tb8haYeviqBz6rkCy12mUSDQX9RdoZf";
        const ENCODE_MAP: [usize; 9] = [8, 7, 0, 5, 1, 3, 2, 4, 6];

        let mut bvid = vec!['0'; 9];
        let mut tmp = (MAX_AID | aid) ^ XOR_CODE;

        for (i, &pos) in ENCODE_MAP.iter().enumerate() {
            let index = (tmp % (ALPHABET.len() as i64)) as usize;
            bvid[pos] = ALPHABET.chars().nth(index).unwrap();
            tmp /= ALPHABET.len() as i64;
        }

        format!("BV1{}", bvid.into_iter().collect::<String>())
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
