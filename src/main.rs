use clap::Parser;
use colored::Colorize;
use std::path::PathBuf;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::parser::{
    detail_parser::{models::DownloadConfig, parser_trait::ParserOptions},
    models::VideoQuality,
};

mod auth;
mod cli;
mod common;
mod downloader;
mod parser;
mod post_process;

#[cfg(feature = "mcp")]
mod mcp;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// 处理用户认证
async fn handle_auth(auth_manager: &auth::AuthManager, args: &cli::Cli) -> Result<Uuid> {
    // 如果提供了cookie，直接使用
    if let Some(cookie) = &args.cookie {
        info!("使用提供的cookie进行登录");
        let id_opt = auth_manager.login_by_cookies(cookie).await?;
        if let Some(id) = id_opt {
            return Ok(id);
        } else {
            return Err("使用提供的cookie登录失败".into());
        }
    }

    // 如果指定了用户目录，尝试从文件加载
    if let Some(user_dir) = &args.user_dir {
        info!("尝试从用户目录加载登录状态");
        if let Ok(Some(id)) = auth_manager
            .login_by_cookies(&user_dir.to_str().unwrap().to_string())
            .await
        {
            info!("{}: {}", "已登录".green(), id);
            return Ok(id);
        }
        warn!("用户目录中未找到有效的登录信息");
    }

    // 如果需要登录，执行登录流程
    if args.login {
        info!("开始二维码登录流程");
        let id = auth_manager.qr_login_flow().await?;
        info!("{}: {}", "登录成功".green(), id);
        return Ok(id);
    }

    Err("需要登录信息".into())
}

/// 准备下载环境
async fn prepare_download_env(args: &cli::Cli) -> Result<(PathBuf, PathBuf)> {
    // 创建状态文件
    let state_file = PathBuf::from("state.json");
    if !state_file.exists() {
        info!("创建下载状态文件");
        tokio::fs::write(&state_file, "[]").await?;
    }

    // 创建输出目录
    let output_dir = args.output_dir.clone();
    info!("创建输出目录: {:?}", output_dir);
    tokio::fs::create_dir_all(&output_dir).await?;

    Ok((state_file, output_dir))
}

/// 从命令行参数生成解析选项
fn create_parser_options(args: &cli::Cli, url: &str) -> ParserOptions {
    // 将命令行的 quality 选项转换为 VideoQuality 枚举
    let quality = match args.quality {
        cli::QualityOption::Q360P => VideoQuality::Q360P, // 流畅 360P
        cli::QualityOption::Q480P => VideoQuality::Q480P, // 清晰 480P
        cli::QualityOption::Q720P => VideoQuality::Q720P, // 高清 720P
        cli::QualityOption::Q720P60 => VideoQuality::Q720P60, // 高清 720P60
        cli::QualityOption::Q1080P => VideoQuality::Q1080P, // 高清 1080P
        cli::QualityOption::Q1080PP => VideoQuality::Q1080PP, // 高清 1080P+
        cli::QualityOption::Q1080P60 => VideoQuality::Q1080P60, // 高清 1080P60
        cli::QualityOption::Q4K => VideoQuality::Q4K,     // 超清 4K
        cli::QualityOption::QHdr => VideoQuality::QHdr,   // HDR 真彩色
        cli::QualityOption::Q8K => VideoQuality::Q8K,     // 超高清 8K
    };

    debug!(
        "命令行质量参数: {:?} -> {:?} ({})",
        args.quality, quality, quality as i32
    );

    // 根据URL类型返回对应的选项
    if url.contains("/bangumi/play/") {
        // 后期可能需要更复杂的逻辑来区分番剧和课程
        ParserOptions::Bangumi {
            config: DownloadConfig {
                resolution: quality,
                need_audio: args.need_audio,
                need_video: args.need_video,
                need_subtitle: args.need_subtitle,
                need_danmaku: args.need_danmaku,
                concurrency: args.concurrency,
                episode_range: args.parts.clone(),
                merge: args.merge,
                output_dir: args
                    .output_dir
                    .clone()
                    .to_str()
                    .unwrap_or("./downloads")
                    .to_string(),
                output_format: "mp4".to_string(),
            },
        }
    } else if url.contains("/cheese/play/") {
        // 课程解析选项
        ParserOptions::Course {
            config: DownloadConfig {
                resolution: quality,
                need_audio: args.need_audio,
                need_video: args.need_video,
                need_subtitle: args.need_subtitle,
                need_danmaku: args.need_danmaku,
                concurrency: args.concurrency,
                episode_range: args.parts.clone(),
                merge: args.merge,
                output_dir: args
                    .output_dir
                    .clone()
                    .to_str()
                    .unwrap_or("./downloads")
                    .to_string(),
                output_format: "mp4".to_string(),
            },
        }
    } else {
        ParserOptions::CommonVideo {
            config: DownloadConfig {
                resolution: quality,
                need_audio: args.need_audio,
                need_video: args.need_video,
                need_subtitle: args.need_subtitle,
                need_danmaku: args.need_danmaku,
                concurrency: args.concurrency,
                episode_range: args.parts.clone(),
                merge: args.merge,
                output_dir: args
                    .output_dir
                    .clone()
                    .to_str()
                    .unwrap_or("./downloads")
                    .to_string(),
                output_format: "mp4".to_string(),
            },
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // 解析命令行参数
    let args = cli::Cli::parse();

    // 检查是否启动MCP服务器模式
    #[cfg(feature = "mcp")]
    if args.mcp {
        info!("🚀 启动 MCP 服务器模式");
        let mut mcp_server = mcp::McpServer::new();
        return mcp_server.run().await.map_err(|e| e.into());
    }

    #[cfg(not(feature = "mcp"))]
    if args.mcp {
        error!("MCP功能未启用。请使用 --features mcp 重新编译");
        return Err("MCP功能未启用".into());
    }

    info!("开始下载视频: {}", args.url);

    // 认证处理
    let auth_manager = auth::AuthManager::new();
    let mut session_id = Uuid::new_v4(); // 默认会话ID

    if args.login || args.cookie.is_some() || args.user_dir.is_some() {
        session_id = handle_auth(&auth_manager, &args).await?;
    } else {
        warn!("未提供登录信息，可能无法下载受限内容");
    }

    let client = auth_manager.get_authed_client(session_id).await?;

    // 创建解析选项
    let options = create_parser_options(&args, &args.url);

    // 解析视频信息
    info!("开始解析...");
    let mut parser = parser::VideoParser::new(client.clone(), true);
    let parsed_metas = parser.parse(&args.url, &options).await.map_err(|e| {
        error!("解析失败: {}", e);
        e
    })?;

    // 可能有多个视频需要下载
    info!("标题: << {} >>", parsed_metas.title);
    debug!("解析结果: {:?}", parsed_metas);

    // 准备下载环境
    let (state_file, output_dir) = prepare_download_env(&args).await?;

    // 开始下载
    let mut task = parsed_metas.download_items.clone();
    let downloader = downloader::VideoDownloader::new(4, state_file, client.clone());
    downloader.download(&mut task).await?;

    // 后处理
    if let Err(e) = parsed_metas.post_process(&task, &options).await {
        error!("后处理失败: {}", e);
    } else {
        info!("后处理完成");
    }
    info!("{}", "下载完成！".green());
    Ok(())
}
