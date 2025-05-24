use std::collections::HashMap;
use regex::Regex;
use serde_json::Value;
use crate::common::api::client::BiliClient;
use crate::common::api::models::common::CommonResponse;
use crate::parser::errors::ParseError;
use crate::parser::models::{VideoInfo, VideoMeta, StreamType};
use crate::parser::parser_trait::Parser;
use crate::parser::utils::FormatTool;

pub struct CommonVideoParser<'a> {
    client: &'a BiliClient,
    video_info: VideoInfo,
    part: Option<String>,
}

impl<'a> Parser for CommonVideoParser<'a> {
    async fn parse(&mut self, url: &str) -> Result<VideoMeta, ParseError> {
        // 提取 bvid
        let bvid = Regex::new(r"(BV\w+)")
            .map_err(|_| ParseError::ParseError("Invalid regex pattern".to_string()))?
            .captures(url)
            .and_then(|c| c.get(0))
            .map(|m| m.as_str())
            .ok_or(ParseError::InvalidUrl)?;

        self.video_info.bvid = bvid.to_string();
        self.video_info.url = url.to_string();

        // 获取视频信息
        self.__get_video_info().await?;

        // 获取媒体信息
        self.__get_video_available_media_info().await?;
        
        // 返回视频元数据
        Ok(VideoMeta {
            title: self.video_info.title.clone(),
            duration: 0,
            segments: vec![],
            quality_options: vec![],
        })
    }
}

impl<'a> CommonVideoParser<'a> {

    pub fn new(client: &'a BiliClient) -> Self {
        Self {
            client,
            video_info: VideoInfo {
                url: String::new(),
                aid: 0,
                bvid: String::new(),
                cid: 0,
                title: String::new(),
                cover: String::new(),
                desc: String::new(),
                views: String::new(),
                danmakus: String::new(),
                up_name: String::new(),
                up_mid: 0,
                video_quality_id_list: vec![],
                video_quality_desc_list: vec![],
                stream_type: StreamType::Dash,
                video_url: String::new(),
                audio_url: String::new(),
            },
            part: None,
        }
    }

    // 解析分P信息
    fn get_part(&mut self, url: &str) {
        let re = Regex::new(r"p=([0-9]+)").unwrap();
        self.part = re.captures(url)
            .and_then(|cap| cap.get(1))
            .and_then(|m| m.as_str().parse().ok());
    }

    // 从av号获取bvid
    fn get_aid(&mut self, url: &str) -> Result<(), ParseError> {
        let re = Regex::new(r"av([0-9]+)").unwrap();
        let aid = re.captures(url)
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
        let bvid = re.find(url)
            .map(|m| m.as_str())
            .ok_or(ParseError::InvalidUrl)?;
            
        self.set_bvid(bvid)?;
        Ok(())
    }

    // 设置bvid和url
    fn set_bvid(&mut self, bvid: &str) -> Result<(), ParseError> {
        self.video_info.bvid = bvid.to_string();
        self.video_info.url = format!("https://www.bilibili.com/video/{}", bvid);
        Ok(())
    }

    // 获取视频信息
    pub async fn __get_video_info(&mut self) -> Result<VideoInfo, ParseError> {
        let params = HashMap::from([
            ("bvid".to_string(), self.video_info.bvid.clone())
        ]);

        let resp = self.client.get_wbi::<CommonResponse<Value>>(
            "https://api.bilibili.com/x/web-interface/wbi/view",
            params
        ).await?;

        // println!("resp get video info: {:?}", resp);
        if resp.code != 0 {
            // debug
            println!("resp get video info: {:?}", resp);
            return Err(ParseError::ApiError(resp.message));
        }
        // println!("resp get video info: {:?}", resp);
        // 处理重定向
        if let Some(data) = &resp.data {
            if let Some(redirect_url) = data.get("redirect_url") {
                if let Some(url) = redirect_url.as_str() {
                    return Err(ParseError::Redirect(url.to_string()));
                }
            } else {
                self.video_info.title = data.get("title").unwrap().as_str().unwrap().to_string();
                self.video_info.cover = data.get("pic").unwrap().as_str().unwrap().to_string();
                self.video_info.desc = data.get("desc").unwrap().as_str().unwrap().to_string();
                self.video_info.up_name = data.get("owner").unwrap().get("name").unwrap().as_str().unwrap().to_string();
                self.video_info.up_mid = data.get("owner").unwrap().get("mid").unwrap().as_i64().unwrap();
                self.video_info.cid = data.get("cid").unwrap().as_i64().unwrap();
                self.video_info.bvid = data.get("bvid").unwrap().as_str().unwrap().to_string();
            }
        }

        Ok(self.video_info.clone())
    }

    // 获取媒体信息
    async fn __get_video_available_media_info(&mut self) -> Result<(), ParseError> {
        let params = HashMap::from([
            ("bvid".to_string(), self.video_info.bvid.clone()),
            ("cid".to_string(), self.video_info.cid.to_string()),
            ("qn".to_string(), "0".to_string()),
            ("fnval".to_string(), "16".to_string()),
            ("fourk".to_string(), "1".to_string()),
        ]);

        let resp = self.client.get_wbi::<CommonResponse<Value>>(
            "https://api.bilibili.com/x/player/wbi/playurl",
            params
        ).await?;

        if resp.code != 0 {
            return Err(ParseError::ApiError(resp.message));
        }

        let download_info = resp.data.as_ref().unwrap();

        if let Some(dash) = download_info.get("dash") {
            self.video_info.stream_type = StreamType::Dash;
            println!("DASH stream detected");
            // println!("DASH stream: {:?}", dash);

            // 将dash转换为JSON字符串
            let dash_str = dash.to_string();
            println!("DASH stream JSON: {}", dash_str);
            // 写入到文件
            std::fs::write("dash.json", dash_str).unwrap();
            
            // 等待输入，暂停程序
            // std::io::stdin().read_line(&mut String::new()).unwrap();

            // 解析视频流
            if let Some(video) = dash.get("video") {
                if let Some(video_list) = video.as_array() {
                    // 选择最高质量的视频流
                    // 测试的时候，选择最低质量的视频流
                    if let Some(best_video) = video_list.iter().min_by_key(|v| {
                        v.get("bandwidth").and_then(|b| b.as_u64()).unwrap_or(0)
                    }) {
                        self.video_info.video_url = best_video.get("baseUrl")
                            .and_then(|u| u.as_str())
                            .unwrap_or("")
                            .to_string();
                    }
                }
            }

            // 解析音频流
            if let Some(audio) = dash.get("audio") {
                if let Some(audio_list) = audio.as_array() {
                    // 选择最高质量的音频流
                    // 测试的时候，选择最低质量的音频流
                    if let Some(best_audio) = audio_list.iter().min_by_key(|a| {
                        a.get("bandwidth").and_then(|b| b.as_u64()).unwrap_or(0)
                    }) {
                        self.video_info.audio_url = best_audio.get("baseUrl")
                            .and_then(|u| u.as_str())
                            .unwrap_or("")
                            .to_string();
                    }
                }
            }
        } else {
            self.video_info.stream_type = StreamType::Flv;
            // 对于 FLV 流，直接使用 durl 中的 URL
            if let Some(durl) = download_info.get("durl") {
                if let Some(durl_list) = durl.as_array() {
                    if let Some(first_url) = durl_list.first() {
                        self.video_info.video_url = first_url.get("url")
                            .and_then(|u| u.as_str())
                            .unwrap_or("")
                            .to_string();
                    }
                }
            }
        }

        Ok(())
    }

    // 获取视频信息
    pub async fn get_video_info(&self) -> Result<VideoInfo, ParseError> {
        Ok(self.video_info.clone())
    }
    

    // 检查 JSON 响应
    fn check_json(&self, resp: &CommonResponse<Value>) -> Result<(), ParseError> {
        if resp.code != 0 {
            return Err(ParseError::ApiError(resp.message.clone()));
        }
        Ok(())
    }

    // 解析视频信息
    fn parse_video_info(&mut self, info: Value) -> Result<(), ParseError> {
        // 检查是否需要重定向
        if let Some(redirect_url) = info.get("redirect_url") {
            return Err(ParseError::Redirect(redirect_url.as_str()
                .ok_or(ParseError::ParseError("Invalid redirect URL".into()))?
                .to_string()));
        }

        // 更新 VideoInfo 结构体
        self.video_info.title = info["title"].as_str()
            .ok_or(ParseError::ParseError("Missing title".into()))?
            .to_string();

        self.video_info.cover = info["pic"].as_str()
            .ok_or(ParseError::ParseError("Missing cover".into()))?
            .to_string();

        self.video_info.desc = info["desc"].as_str()
            .ok_or(ParseError::ParseError("Missing description".into()))?
            .to_string();

        self.video_info.views = FormatTool::format_data_count(
            info["stat"]["view"].as_u64()
                .ok_or(ParseError::ParseError("Missing view count".into()))?
        );

        self.video_info.danmakus = FormatTool::format_data_count(
            info["stat"]["danmaku"].as_u64()
                .ok_or(ParseError::ParseError("Missing danmaku count".into()))?
        );

        self.video_info.up_name = info["owner"]["name"].as_str()
            .ok_or(ParseError::ParseError("Missing uploader name".into()))?
            .to_string();

        self.video_info.up_mid = info["owner"]["mid"].as_i64()
            .ok_or(ParseError::ParseError("Missing uploader ID".into()))?;

        self.video_info.cid = info["cid"].as_i64()
            .ok_or(ParseError::ParseError("Missing cid".into()))?;

        Ok(())
    }

    // 解析 DASH 流信息
    fn parse_dash_stream(&mut self, dash: &Value) -> Result<(), ParseError> {
        // TODO: 实现 DASH 流解析
        Ok(())
    }

    pub fn find_video_id(&self, url: &str) -> Option<&'static str> {
        let re = Regex::new(r"av|BV").ok()?;
        re.find(url)
            .map(|m| m.as_str())
            .and_then(|s| match s {
                "av" => Some("av"),
                "BV" => Some("BV"),
                _ => None
            })
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


// 分集信息结构体
#[derive(Debug, Clone)]
struct EpisodeItem {
    title: String,
    cid: i64,
    badge: String,
    duration: String,
}

// 视频类型枚举
#[derive(Debug, Clone, PartialEq)]
enum VideoType {
    Single,
    Part,
    Collection,
}

// 显示模式枚举
#[derive(Debug, Clone, PartialEq)]
enum EpisodeDisplayType {
    Single,
    InSection,
    All,
}

impl From<i32> for EpisodeDisplayType {
    fn from(value: i32) -> Self {
        match value {
            0 => EpisodeDisplayType::Single,
            1 => EpisodeDisplayType::InSection,
            2 => EpisodeDisplayType::All,
            _ => EpisodeDisplayType::Single,
        }
    }
}