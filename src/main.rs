use clap::Parser;
use colored::Colorize;
use uuid::Uuid;
use std::path::PathBuf;

mod cli;
mod common;
mod auth;
mod parser;
mod downloader;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[tokio::main]
async fn main() -> Result<()> {
    let args = cli::Cli::parse();
    print!("{}: {}", "正在解析视频信息".green(), args.url);

    let auth_manager = auth::AuthManager::new();
    println!("args.user_dir: {:?}", args.user_dir);
    let mut session_id: Option<Uuid> = None;
    // 先检查是否有登录状态
    if args.user_dir.is_some() {
        // 检查登录状态
        session_id = auth_manager.login_by_cookies_file(&args.user_dir.as_ref().unwrap()).await?;
        if session_id.is_some() {
            println!("{}: {}", "已登录".green(), session_id.unwrap());
        } else {
            println!("{}: {}", "未登录".red(), "请使用 --login 选项登录");
            return Ok(());
        }
    }
    if args.login {
        // 先登录
        let session_id = auth_manager.qr_login_flow().await?;
        println!("登录成功，session_id: {}", session_id);
    }

    let is_login: bool = session_id.is_some();

    let client = auth_manager.get_authed_client(session_id.unwrap()).await?;

    let mut parser = parser::VideoParser::new(client, is_login);

    // 获取元数据
    let meta = parser.parse(&args.url).await?;
    println!("视频标题: {}", meta.title);

    // 获取视频信息
    let video_info = parser.get_video_info().ok_or("无法获取视频信息")?;
    
    // 创建下载管理器
    let state_file = PathBuf::from("state.json");
    if !state_file.exists() {
        tokio::fs::write(&state_file, "[]").await?;
    }
    let download_manager = downloader::manager::DownloadManager::new(4, state_file);
    
    // 创建输出目录
    let output_dir = PathBuf::from(&args.output);
    tokio::fs::create_dir_all(&output_dir).await?;

    // 根据流类型选择下载方式
    match video_info.stream_type {
        parser::models::StreamType::Dash => {
            println!("开始下载 DASH 流视频... {:?}", video_info.video_url);
            
            // 下载视频流
            let video_task_id = download_manager.add_task(
                &video_info.video_url,
                &output_dir.join(format!("{}_video.mp4", video_info.bvid))
            ).await.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            
            println!("开始下载视频流: {}", video_info.audio_url);
            // 下载音频流
            let audio_task_id = download_manager.add_task(
                &video_info.audio_url,
                &output_dir.join(format!("{}_audio.m4a", video_info.bvid))
            ).await.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

            println!("正在下载视频和音频流...");
            
            // 等待下载完成
            loop {
                let video_status = download_manager.get_task_status(&video_task_id).await;
                let audio_status = download_manager.get_task_status(&audio_task_id).await;
                
                if video_status == Some(downloader::task::TaskStatus::Completed) 
                    && audio_status == Some(downloader::task::TaskStatus::Completed) {
                    break;
                }
                println!("video_status: {:?}", video_status);
                println!("audio_status: {:?}", audio_status);
                println!("等待下载完成...");
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }

            println!("开始合并视频和音频...");
            
            // 合并视频和音频
            let merger = downloader::merger::MediaMerger;
            merger.merge_av(
                &output_dir.join(format!("{}_video.mp4", video_info.bvid)),
                &output_dir.join(format!("{}_audio.m4a", video_info.bvid)),
                &output_dir.join(format!("{}.mp4", video_info.title))
            ).await.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

            println!("下载完成！");
        },
        parser::models::StreamType::Flv => {
            println!("开始下载 FLV 视频...");
            
            // 直接下载 FLV 文件
            let task_id = download_manager.add_task(
                &video_info.video_url,
                &output_dir.join(format!("{}.flv", video_info.title))
            ).await.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

            println!("正在下载视频...");
            
            // 等待下载完成
            loop {
                let status = download_manager.get_task_status(&task_id).await;
                if status == Some(downloader::task::TaskStatus::Completed) {
                    break;
                }
                println!("等待下载完成...");
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }

            println!("下载完成！");
        }
    }

    Ok(())
}