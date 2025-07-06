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

        // 检查 ffmpeg 是否可用
        debug!("检查系统中是否安装了 ffmpeg...");
        let ffmpeg_check = Command::new("ffmpeg")
            .arg("-version")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;

        if ffmpeg_check.is_err() || !ffmpeg_check.unwrap().success() {
            error!("❌ 未检测到 ffmpeg，请确保系统中已安装并配置了 ffmpeg 可执行路径。");
            error!("安装方法参考：https://ffmpeg.org/download.html");
            return Err(DownloadError::FfmpegNotFound);
        }

        debug!("开始合并视频和音频 -> 输出路径: {:?}", output_path);

        let output = Command::new("ffmpeg")
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
}
