use std::path::Path;
use tokio::process::Command;
use super::error::DownloadError;

pub struct  MediaMerger;

impl MediaMerger {
    pub  async fn merge_av(&self, video_path: &Path, audio_path: &Path, output_path: &Path) -> Result<(), DownloadError> {
        // 使用ffmpeg命令行工具合并视频和音频
        let status = Command::new("ffmpeg")
            .arg("-i")
            .arg(video_path)
            .arg("-i")
            .arg(audio_path)
            .arg("-c:v")
            .arg("copy")
            .arg("-c:a")
            .arg("aac")
            .arg(output_path)
            .status()
            .await?;

        if status.success() {
            Ok(())
        } else {
            Err(DownloadError::MergeError("Failed to merge video and audio".to_string()))
        }
        
    }
}