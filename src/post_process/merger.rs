use std::path::Path;
use tokio::process::Command;
use tracing::{debug, error};

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

        debug!("检查视频文件: {:?}", video_path);

        if !audio_path.exists() {
            return Err(DownloadError::FileNotFound(audio_path.to_path_buf()));
        }
        debug!("检查音频文件: {:?}", audio_path);

        debug!("开始合并视频和音频到: {:?}", output_path);

        // 检查ffmpeg是否安装
        if Command::new("ffmpeg")
            .arg("-version")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await
            .is_err()
        {
            return Err(DownloadError::FfmpegNotFound);
        }

        // 使用ffmpeg命令行工具合并视频和音频
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
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .output()
            .await?;

        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            error!("ffmpeg 执行失败，日志如下：\n{}", err_msg);
            return Err(DownloadError::FfmpegError(err_msg.to_string()));
        }
        Ok(())
    }
}
