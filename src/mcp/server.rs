use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde_json::{json, Value};

use crate::auth::AuthManager;
use crate::downloader::VideoDownloader;
use crate::parser::{VideoParser, models::VideoQuality};
use crate::parser::detail_parser::models::DownloadConfig;
use crate::parser::detail_parser::parser_trait::ParserOptions;
use crate::common::client::client::BiliClient;

#[derive(serde::Deserialize, serde::Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<Value>,
    result: Option<Value>,
    error: Option<JsonRpcError>,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    data: Option<Value>,
}

pub struct BiliMcpServer {
    auth_manager: AuthManager,
    active_downloads: Arc<Mutex<HashMap<String, String>>>, // task_id -> status
    download_manager: Arc<Mutex<Option<VideoDownloader>>>,
}

impl BiliMcpServer {
    pub fn new() -> Self {
        Self {
            auth_manager: AuthManager::new(),
            active_downloads: Arc::new(Mutex::new(HashMap::new())),
            download_manager: Arc::new(Mutex::new(None)),
        }
    }

    // 初始化下载管理器
    async fn init_downloader(&self) -> anyhow::Result<()> {
        let mut manager = self.download_manager.lock().await;
        if manager.is_none() {
            *manager = Some(VideoDownloader::new(4, "state.json".into(), BiliClient::new()));
        }
        Ok(())
    }

    // 运行MCP服务器
    pub async fn run(&self) -> anyhow::Result<()> {
        eprintln!("🚀 BiliDownloader MCP Server 启动中...");
        eprintln!("📡 等待AI助手连接...");
        eprintln!("🔧 MCP功能已实现，支持以下工具:");
        eprintln!("   - bili_download: 下载B站视频");
        eprintln!("   - bili_parse_info: 解析视频信息");
        eprintln!("   - bili_list_downloads: 列出下载任务");
        eprintln!("   - bili_cancel_download: 取消下载任务");
        eprintln!("   - bili_login_status: 检查登录状态");
        eprintln!("   - bili_qr_login: 二维码登录");

        let mut stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut buffer = [0; 1024];

        loop {
            match stdin.read(&mut buffer).await {
                Ok(0) => break, // EOF
                Ok(n) => {
                    let line = String::from_utf8_lossy(&buffer[..n]);
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }

                    let response = self.handle_request(line).await;
                    if let Ok(response_json) = serde_json::to_string(&response) {
                        stdout.write_all(response_json.as_bytes()).await?;
                        stdout.write_all(b"\n").await?;
                        stdout.flush().await?;
        }
    }
                Err(e) => {
                    eprintln!("读取输入错误: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    pub async fn handle_request(&self, request_line: &str) -> JsonRpcResponse {
        let request: Result<JsonRpcRequest, _> = serde_json::from_str(request_line.trim());

        match request {
            Ok(req) => {
                let result = match req.method.as_str() {
                    "initialize" => self.handle_initialize(req.params).await,
                    "tools/list" => self.handle_tools_list().await,
                    "tools/call" => self.handle_tools_call(req.params).await,
                    "resources/list" => self.handle_resources_list().await,
                    "resources/read" => self.handle_resources_read(req.params).await,
                    _ => Err(anyhow::anyhow!("未知方法: {}", req.method)),
                };

                match result {
                    Ok(result) => JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: req.id,
                        result: Some(result),
                        error: None,
                    },
                    Err(e) => JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: req.id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32603,
                            message: e.to_string(),
                            data: None,
                        }),
                    },
                }
            }
            Err(e) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: None,
                result: None,
                error: Some(JsonRpcError {
                    code: -32700,
                    message: format!("解析JSON失败: {}", e),
                    data: None,
                }),
            },
        }
    }

    async fn handle_initialize(&self, _params: Option<Value>) -> anyhow::Result<Value> {
        Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {
                    "listChanged": true
                },
                "resources": {
                    "listChanged": true
                }
            },
            "serverInfo": {
                "name": "bili-downloader-mcp",
                "version": "0.1.0"
            }
        }))
    }

    async fn handle_tools_list(&self) -> anyhow::Result<Value> {
        Ok(json!({
            "tools": [
                {
                    "name": "bili_download",
                    "description": "下载B站视频、番剧或课程",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "url": {
                                "type": "string",
                                "description": "B站视频/番剧/课程链接"
                            },
                            "quality": {
                                "type": "string",
                                "description": "视频清晰度",
                                "enum": ["360p", "480p", "720p", "720p60", "1080p", "1080p+", "1080p60", "4k", "hdr", "8k"],
                                "default": "1080p"
                            },
                            "output_dir": {
                                "type": "string",
                                "description": "输出目录路径",
                                "default": "./downloads"
                            },
                            "parts": {
                                "type": "string",
                                "description": "要下载的集数范围 (如: 1-5,7,9-12)，仅适用于番剧和课程"
                            },
                            "login_required": {
                                "type": "boolean",
                                "description": "是否需要登录下载高清内容",
                                "default": false
                            }
                        },
                        "required": ["url"]
                    }
                },
                {
                    "name": "bili_parse_info",
                    "description": "解析B站视频信息但不下载",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "url": {
                                "type": "string",
                                "description": "B站视频/番剧/课程链接"
                            }
                        },
                        "required": ["url"]
                    }
                },
                {
                    "name": "bili_list_downloads",
                    "description": "列出当前下载任务状态",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                },
                {
                    "name": "bili_cancel_download",
                    "description": "取消指定的下载任务",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "task_id": {
                                "type": "string",
                                "description": "要取消的任务ID"
                            }
                        },
                        "required": ["task_id"]
                    }
                },
                {
                    "name": "bili_login_status",
                    "description": "检查当前B站登录状态",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                },
                {
                    "name": "bili_qr_login",
                    "description": "生成二维码进行B站登录",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                }
            ]
        }))
    }

    async fn handle_tools_call(&self, params: Option<Value>) -> anyhow::Result<Value> {
        let params = params.ok_or_else(|| anyhow::anyhow!("缺少参数"))?;
        let args = params.get("arguments").ok_or_else(|| anyhow::anyhow!("缺少arguments"))?;
        let name = params.get("name").and_then(|v| v.as_str()).ok_or_else(|| anyhow::anyhow!("缺少name"))?;

        let result = match name {
            "bili_download" => self.handle_download(args.clone()).await?,
            "bili_parse_info" => self.handle_parse_info(args.clone()).await?,
            "bili_list_downloads" => self.handle_list_downloads(args.clone()).await?,
            "bili_cancel_download" => self.handle_cancel_download(args.clone()).await?,
            "bili_login_status" => self.handle_login_status(args.clone()).await?,
            "bili_qr_login" => self.handle_qr_login(args.clone()).await?,
            _ => return Err(anyhow::anyhow!("未知工具: {}", name)),
        };

        Ok(result)
    }

    async fn handle_resources_list(&self) -> anyhow::Result<Value> {
        Ok(json!({
            "resources": [
                {
                    "uri": "downloads://active",
                    "name": "Active Downloads",
                    "description": "当前活跃的下载任务列表",
                    "mimeType": "application/json"
                },
                {
                    "uri": "downloads://history",
                    "name": "Download History",
                    "description": "下载历史记录",
                    "mimeType": "application/json"
                },
                {
                    "uri": "config://settings",
                    "name": "Downloader Settings",
                    "description": "下载器配置设置",
                    "mimeType": "application/json"
                }
            ]
        }))
    }

    async fn handle_resources_read(&self, params: Option<Value>) -> anyhow::Result<Value> {
        let params = params.ok_or_else(|| anyhow::anyhow!("缺少参数"))?;
        let uri = params.get("uri").and_then(|v| v.as_str()).ok_or_else(|| anyhow::anyhow!("缺少uri"))?;

        let content = match uri {
            "downloads://active" => self.get_active_downloads_content().await,
            "downloads://history" => self.get_download_history_content().await,
            "config://settings" => self.get_settings_content().await,
            _ => return Err(anyhow::anyhow!("未知资源: {}", uri)),
        };

        Ok(json!({
            "contents": [{
                "uri": uri,
                "mimeType": "application/json",
                "text": content
            }]
        }))
    }

    async fn handle_download(&self, args: Value) -> anyhow::Result<Value> {
        let url = args["url"].as_str()
            .ok_or_else(|| anyhow::anyhow!("缺少url参数"))?;

        let quality = args["quality"].as_str().unwrap_or("1080p");
        let output_dir = args["output_dir"].as_str().unwrap_or("./downloads");
        let parts = args["parts"].as_str();

        // 初始化下载管理器
        self.init_downloader().await?;

        // 解析质量参数
        let video_quality = match quality {
            "360p" => VideoQuality::Q360P,
            "480p" => VideoQuality::Q480P,
            "720p" => VideoQuality::Q720P,
            "720p60" => VideoQuality::Q720P60,
            "1080p" => VideoQuality::Q1080P,
            "1080p+" => VideoQuality::Q1080PP,
            "1080p60" => VideoQuality::Q1080P60,
            "4k" => VideoQuality::Q4K,
            "hdr" => VideoQuality::QHdr,
            "8k" => VideoQuality::Q8K,
            _ => VideoQuality::Q1080P,
        };

        // 创建解析选项
        let options = if url.contains("/bangumi/play/") {
            ParserOptions::Bangumi {
                config: DownloadConfig {
                    resolution: video_quality,
                    need_audio: true,
                    need_video: true,
                    need_subtitle: false,
                    need_danmaku: false,
                    concurrency: 3,
                    episode_range: parts.map(|s| s.to_string()),
                    merge: true,
                    output_dir: output_dir.to_string(),
                    output_format: "mp4".to_string(),
                },
            }
        } else {
            ParserOptions::CommonVideo {
                config: DownloadConfig {
                    resolution: video_quality,
                    need_audio: true,
                    need_video: true,
                    need_subtitle: false,
                    need_danmaku: false,
                    concurrency: 3,
                    episode_range: parts.map(|s| s.to_string()),
                    merge: true,
                    output_dir: output_dir.to_string(),
                    output_format: "mp4".to_string(),
                },
            }
        };

        // 创建客户端
        let client = self.auth_manager.get_authed_client(uuid::Uuid::new_v4()).await?;

        // 解析视频信息
        let mut parser = VideoParser::new(client.clone(), true);
        let parsed_metas = parser.parse(url, &options).await?;

        // 开始下载
        let task_id = uuid::Uuid::new_v4().to_string();
        {
            let mut downloads = self.active_downloads.lock().await;
            downloads.insert(task_id.clone(), "downloading".to_string());
        }

        // 异步执行下载
        let downloads_clone = self.active_downloads.clone();
        let task_id_clone = task_id.clone();
        let download_manager = self.download_manager.clone();

        tokio::spawn(async move {
            let mut task = parsed_metas.download_items.clone();
            if let Some(manager) = download_manager.lock().await.as_ref() {
                let result = manager.download(&mut task).await;
                let mut downloads = downloads_clone.lock().await;
                if result.is_ok() {
                    downloads.insert(task_id_clone, "completed".to_string());
                } else {
                    downloads.insert(task_id_clone, "failed".to_string());
                }
            }
        });

        Ok(json!([{
            "type": "text",
            "text": format!("开始下载: {} (任务ID: {})", url, task_id)
        }]))
    }

    async fn handle_parse_info(&self, args: Value) -> anyhow::Result<Value> {
        let url = args["url"].as_str()
            .ok_or_else(|| anyhow::anyhow!("缺少url参数"))?;

        // 创建客户端
        let client = self.auth_manager.get_authed_client(uuid::Uuid::new_v4()).await?;

        // 解析视频信息
        let mut parser = VideoParser::new(client.clone(), true);
        let options = ParserOptions::CommonVideo {
            config: DownloadConfig {
                resolution: VideoQuality::Q1080P,
                need_audio: false,
                need_video: false,
                need_subtitle: false,
                need_danmaku: false,
                concurrency: 1,
                episode_range: None,
                merge: false,
                output_dir: "./downloads".to_string(),
                output_format: "mp4".to_string(),
            },
        };

        let parsed_metas = parser.parse(url, &options).await?;

        Ok(json!([{
            "type": "text",
            "text": json!({
                "title": parsed_metas.title,
                "url": url,
                "available_qualities": ["360p", "480p", "720p", "1080p", "4k"],
                "episodes": parsed_metas.download_items.len()
            }).to_string()
        }]))
    }

    async fn handle_list_downloads(&self, _args: Value) -> anyhow::Result<Value> {
        let downloads = self.active_downloads.lock().await;
        let downloads_list: Vec<Value> = downloads
            .iter()
            .map(|(id, status)| json!({
                "task_id": id,
                "status": status,
                "progress": if status == "completed" { "100%" } else { "进行中" }
            }))
            .collect();

        Ok(json!([{
            "type": "text",
            "text": json!({
                "active_downloads": downloads_list
            }).to_string()
        }]))
    }

    async fn handle_cancel_download(&self, args: Value) -> anyhow::Result<Value> {
        let task_id = args["task_id"].as_str()
            .ok_or_else(|| anyhow::anyhow!("缺少task_id参数"))?;
        
        let mut downloads = self.active_downloads.lock().await;
        if downloads.remove(task_id).is_some() {
            Ok(json!([{
                "type": "text",
                "text": format!("已取消任务: {}", task_id)
            }]))
        } else {
            Ok(json!([{
                "type": "text",
                "text": "任务不存在"
            }]))
        }
    }

    async fn handle_login_status(&self, _args: Value) -> anyhow::Result<Value> {
        // 检查登录状态的简化实现
        Ok(json!([{
            "type": "text",
            "text": json!({
            "logged_in": false,
            "user_info": null
            }).to_string()
        }]))
    }

    async fn handle_qr_login(&self, _args: Value) -> anyhow::Result<Value> {
        // 二维码登录的简化实现
        Ok(json!([{
            "type": "text",
            "text": json!({
                "qr_code": "二维码生成中...",
                "login_url": "https://passport.bilibili.com/qrcode/h5/login",
            "message": "请使用B站APP扫描二维码登录"
            }).to_string()
        }]))
    }

    async fn get_active_downloads_content(&self) -> String {
        let downloads = self.active_downloads.lock().await;
        json!({
            "active_downloads": downloads.len(),
            "tasks": downloads.iter().map(|(id, status)| json!({
                "task_id": id,
                "status": status
            })).collect::<Vec<_>>()
        }).to_string()
    }

    async fn get_download_history_content(&self) -> String {
        // 简化的历史记录实现
        json!({
            "history": []
        }).to_string()
    }

    async fn get_settings_content(&self) -> String {
        json!({
            "default_quality": "1080p",
            "output_directory": "./downloads",
            "concurrency": 3,
            "auto_merge": true
        }).to_string()
    }
}
