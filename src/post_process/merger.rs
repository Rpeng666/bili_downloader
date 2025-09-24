use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, error, info};

use crate::downloader::error::DownloadError;

pub struct MediaMerger;

impl MediaMerger {
    pub async fn merge_av(
        video_path: &Path,
        audio_path: &Path,
        output_path: &Path,
    ) -> Result<(), DownloadError> {
        // 检查输入文件是否存在
        if !video_path.exists() {
            return Err(DownloadError::FileNotFound(video_path.to_path_buf()));
        }
        debug!("✅ 视频文件存在: {:?}", video_path);

        if !audio_path.exists() {
            return Err(DownloadError::FileNotFound(audio_path.to_path_buf()));
        }
        debug!("✅ 音频文件存在: {:?}", audio_path);

        debug!("开始合并视频和音频 -> 输出路径: {:?}", output_path);

        Self::merge_with_external_ffmpeg(video_path, audio_path, output_path).await
    }

    async fn merge_with_external_ffmpeg(
        video_path: &Path,
        audio_path: &Path,
        output_path: &Path,
    ) -> Result<(), DownloadError> {
        // 获取 ffmpeg 路径（支持环境变量和自动检测）
        let ffmpeg_cmd = Self::find_ffmpeg_path().await?;

        let output = Command::new(&ffmpeg_cmd)
            .arg("-i")
            .arg(video_path)
            .arg("-i")
            .arg(audio_path)
            .arg("-c:v")
            .arg("copy")
            .arg("-c:a")
            .arg("aac") // 使用AAC编码音频
            .arg("-y") // 自动覆盖
            .arg(output_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            error!("❌ ffmpeg 合并失败，错误日志如下：\n{}", err_msg);

            // 加入用户友好的提示
            error!(
                "请检查以下几点：\n\
                1. 输入文件路径是否正确；\n\
                2. 视频/音频文件编码格式是否兼容；\n\
                3. 是否有写入权限到输出路径：{:?};",
                output_path
            );

            return Err(DownloadError::FfmpegError(err_msg.to_string()));
        }

        info!("✅ 视频与音频合并成功，输出文件: {:?}", output_path);
        Ok(())
    }

    async fn find_ffmpeg_path() -> Result<String, DownloadError> {
        // 首先检查环境变量
        if let Ok(path) = std::env::var("FFMPEG_PATH") {
            if Self::check_ffmpeg(&path).await {
                return Ok(path);
            }
        }

        // 检查同级目录的 FFmpeg（打包版本）
        let exe_dir = std::env::current_exe()
            .map_err(|e| DownloadError::FfmpegError(format!("获取可执行文件路径失败: {}", e)))?
            .parent()
            .unwrap()
            .to_path_buf();

        let bundled_paths = [
            exe_dir.join("ffmpeg.exe"),  // Windows
            exe_dir.join("ffmpeg"),      // Unix
        ];

        for path in &bundled_paths {
            if let Some(path_str) = path.to_str() {
                if Self::check_ffmpeg(path_str).await {
                    return Ok(path_str.to_string());
                }
            }
        }

        // 检查常见路径
        let common_paths = [
            "ffmpeg",           // PATH 中
            "ffmpeg.exe",       // Windows
            "./ffmpeg",         // 当前目录
            "./ffmpeg.exe",     // 当前目录 Windows
            "/usr/bin/ffmpeg",  // Linux
            "/usr/local/bin/ffmpeg", // macOS/Linux
            "C:\\ffmpeg\\bin\\ffmpeg.exe", // Windows 常见安装路径
        ];

        for path in &common_paths {
            if Self::check_ffmpeg(path).await {
                return Ok(path.to_string());
            }
        }

        // 如果都没找到，提供安装指导
        error!("❌ 未检测到 ffmpeg，请安装 FFmpeg：");
        error!("  Windows (Chocolatey): choco install ffmpeg");
        error!("  Ubuntu/Debian: sudo apt install ffmpeg");
        error!("  macOS (Homebrew): brew install ffmpeg");
        error!("  或从 https://ffmpeg.org/download.html 下载");
        error!("  安装后重新运行，或设置环境变量 FFMPEG_PATH 指向 ffmpeg 可执行文件");

        Err(DownloadError::FfmpegNotFound)
    }

    async fn check_ffmpeg(path: &str) -> bool {
        Command::new(path)
            .arg("-version")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }
}
