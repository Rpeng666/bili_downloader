use serde_json::{json, Value};

/// 定义BiliDownloader的MCP工具
pub fn get_tool_definitions() -> Vec<Value> {
    vec![
        json!({
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
        }),
        
        json!({
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
        }),
        
        json!({
            "name": "bili_list_downloads",
            "description": "列出当前下载任务状态",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        
        json!({
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
        }),
        
        json!({
            "name": "bili_login_status",
            "description": "检查当前B站登录状态",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        
        json!({
            "name": "bili_qr_login",
            "description": "生成二维码进行B站登录",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        })
    ]
}
