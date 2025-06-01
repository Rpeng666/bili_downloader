use clap::Parser;
use colored::Colorize;
use std::path::PathBuf;
use tracing::{error, info, warn};
use uuid::Uuid;

mod auth;
mod cli;
mod common;
mod downloader;
mod parser;

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

    error!("未提供登录信息，请使用 --login 选项登录");
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
    let output_dir = args.output.clone();
    info!("创建输出目录: {:?}", output_dir);
    tokio::fs::create_dir_all(&output_dir).await?;

    Ok((state_file, output_dir))
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    // 解析命令行参数
    let args = cli::Cli::parse();
    info!("开始下载视频: {}", args.url);

    // 认证处理
    let auth_manager = auth::AuthManager::new();
    let session_id = handle_auth(&auth_manager, &args).await?;
    let client = auth_manager.get_authed_client(session_id).await?;

    // 解析视频信息
    info!("开始解析视频信息");
    let mut parser = parser::VideoParser::new(client.clone(), true);
    let meta = parser.parse(&args.url).await?;
    info!("视频标题: {}", meta.title);

    let video_info = parser.get_video_info().ok_or_else(|| {
        error!("无法获取视频信息");
        "无法获取视频信息"
    })?;

    // 准备下载环境
    let (state_file, output_dir) = prepare_download_env(&args).await?;

    // 开始下载
    info!("开始下载视频");
    let downloader = downloader::VideoDownloader::new(4, state_file, output_dir, client.clone());
    downloader.download(&video_info).await?;

    info!("{}", "下载完成！".green());
    Ok(())
}
