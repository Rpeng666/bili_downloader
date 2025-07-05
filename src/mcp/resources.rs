use serde_json::{json, Value};

/// 定义BiliDownloader的MCP资源
pub fn get_resource_definitions() -> Vec<Value> {
    vec![
        json!({
            "uri": "downloads://active",
            "name": "Active Downloads",
            "description": "当前活跃的下载任务列表",
            "mimeType": "application/json"
        }),
        
        json!({
            "uri": "downloads://history",
            "name": "Download History", 
            "description": "下载历史记录",
            "mimeType": "application/json"
        }),
        
        json!({
            "uri": "config://settings",
            "name": "Downloader Settings",
            "description": "下载器配置设置",
            "mimeType": "application/json"
        })
    ]
}

/// 获取资源内容
pub async fn get_resource_content(uri: &str) -> anyhow::Result<Value> {
    match uri {
        "downloads://active" => {
            // TODO: 返回实际的活跃下载任务
            Ok(json!({
                "active_downloads": [],
                "total_count": 0
            }))
        },
        
        "downloads://history" => {
            // TODO: 返回下载历史
            Ok(json!({
                "history": [],
                "total_count": 0
            }))
        },
        
        "config://settings" => {
            // TODO: 返回配置设置
            Ok(json!({
                "default_quality": "1080p",
                "output_directory": "./downloads",
                "concurrency": 3,
                "auto_merge": true
            }))
        },
        
        _ => Err(anyhow::anyhow!("未知的资源URI: {}", uri))
    }
}
