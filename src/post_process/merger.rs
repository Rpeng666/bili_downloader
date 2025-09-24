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

        // 尝试使用打包的 FFmpeg（如果启用 bundled-ffmpeg 特性）
        #[cfg(feature = "bundled-ffmpeg")]
        {
            match Self::try_merge_with_bundled_ffmpeg(video_path, audio_path, output_path).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    warn!("打包的 FFmpeg 不可用，回退到系统 FFmpeg: {}", e);
                    // 继续到外部 FFmpeg
                }
            }
        }

        // 使用外部 FFmpeg
        Self::merge_with_external_ffmpeg(video_path, audio_path, output_path).await
    }

    #[cfg(feature = "bundled-ffmpeg")]
    async fn try_merge_with_bundled_ffmpeg(
        video_path: &Path,
        audio_path: &Path,
        output_path: &Path,
    ) -> Result<(), DownloadError> {
        use ffmpeg_next as ffmpeg;

        // 使用 spawn_blocking 来运行阻塞的 ffmpeg 操作
        tokio::task::spawn_blocking(move || {
            // 初始化 ffmpeg
            ffmpeg::init().map_err(|e| DownloadError::FfmpegError(format!("FFmpeg 初始化失败: {}", e)))?;

            // 打开输入文件
            let mut ictx_video = ffmpeg::format::input(&video_path)
                .map_err(|e| DownloadError::FfmpegError(format!("打开视频文件失败: {}", e)))?;
            let mut ictx_audio = ffmpeg::format::input(&audio_path)
                .map_err(|e| DownloadError::FfmpegError(format!("打开音频文件失败: {}", e)))?;

            // 创建输出文件
            let mut octx = ffmpeg::format::output(&output_path)
                .map_err(|e| DownloadError::FfmpegError(format!("创建输出文件失败: {}", e)))?;

            // 复制视频流
            let video_stream = ictx_video.streams().best(ffmpeg::media::Type::Video)
                .ok_or_else(|| DownloadError::FfmpegError("未找到视频流".to_string()))?;
            let video_out_stream = octx.add_stream(ffmpeg::encoder::find(ffmpeg::codec::Id::H264))
                .map_err(|e| DownloadError::FfmpegError(format!("添加视频流失败: {}", e)))?;
            video_out_stream.set_parameters(video_stream.parameters());

            // 复制音频流
            let audio_stream = ictx_audio.streams().best(ffmpeg::media::Type::Audio)
                .ok_or_else(|| DownloadError::FfmpegError("未找到音频流".to_string()))?;
            let audio_out_stream = octx.add_stream(ffmpeg::encoder::find(ffmpeg::codec::Id::AAC))
                .map_err(|e| DownloadError::FfmpegError(format!("添加音频流失败: {}", e)))?;
            audio_out_stream.set_parameters(audio_stream.parameters());

            // 写入文件头
            octx.write_header()
                .map_err(|e| DownloadError::FfmpegError(format!("写入文件头失败: {}", e)))?;

            // 复制包
            for (stream, packet) in ictx_video.packets() {
                if stream.index() == video_stream.index() {
                    octx.write_packet(&packet)
                        .map_err(|e| DownloadError::FfmpegError(format!("写入视频包失败: {}", e)))?;
                }
            }

            for (stream, packet) in ictx_audio.packets() {
                if stream.index() == audio_stream.index() {
                    octx.write_packet(&packet)
                        .map_err(|e| DownloadError::FfmpegError(format!("写入音频包失败: {}", e)))?;
                }
            }

            // 写入文件尾
            octx.write_trailer()
                .map_err(|e| DownloadError::FfmpegError(format!("写入文件尾失败: {}", e)))?;

            info!("✅ 视频与音频合并成功 (使用打包的 FFmpeg)，输出文件: {:?}", output_path);
            Ok(())
        })
        .await
        .map_err(|e| DownloadError::FfmpegError(format!("异步任务失败: {}", e)))?
    }

    async fn merge_with_external_ffmpeg(
        video_path: &Path,
        audio_path: &Path,
        output_path: &Path,
    ) -> Result<(), DownloadError> {
        // 获取 ffmpeg 路径（支持环境变量）
        let ffmpeg_cmd = std::env::var("FFMPEG_PATH").unwrap_or_else(|_| "ffmpeg".to_string());

        // 检查 ffmpeg 是否可用
        debug!("检查系统中是否安装了 ffmpeg...");
        let ffmpeg_check = Command::new(&ffmpeg_cmd)
            .arg("-version")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;

        if ffmpeg_check.is_err() || !ffmpeg_check.unwrap().success() {
            error!("❌ 未检测到 ffmpeg，请确保系统中已安装并配置了 ffmpeg 可执行路径。");
            error!("安装方法参考：https://ffmpeg.org/download.html");
            error!("或者设置环境变量 FFMPEG_PATH 指向 ffmpeg 可执行文件路径");
            return Err(DownloadError::FfmpegNotFound);
        }

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

        info!("✅ 视频与音频合并成功 (使用系统 FFmpeg)，输出文件: {:?}", output_path);
        Ok(())
    }
}
