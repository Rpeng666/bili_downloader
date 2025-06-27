pub mod merger;

use crate::{
    downloader::models::{DownloadTask, FileType},
    parser::{detail_parser::parser_trait::ParserOptions, errors::ParseError},
};
use std::path::Path;

pub async fn post_process(
    task: &Vec<DownloadTask>,
    parser_options: &ParserOptions,
) -> Result<(), ParseError> {
    let video = task.iter().find(|t| t.file_type == FileType::Video);
    let audio = task.iter().find(|t| t.file_type == FileType::Audio);
    match parser_options {
        ParserOptions::CommonVideo { config } => {
            if config.merge && config.need_video && config.need_audio {
                merger::MediaMerger::merge_av(
                    Path::new(
                        &video
                            .ok_or(ParseError::ParseError("视频文件未找到".to_string()))?
                            .output_path,
                    ),
                    Path::new(
                        &audio
                            .ok_or(ParseError::ParseError("音频文件未找到".to_string()))?
                            .output_path,
                    ),
                    &Path::new(&config.output_dir)
                        .join(
                            video
                                .as_ref()
                                .map(|p| &p.name)
                                .unwrap_or(&"video".to_string()),
                        )
                        .with_extension("mp4"),
                )
                .await
                .map_err(|e| ParseError::ParseError(format!("合并失败: {}", e)))?;
            }
        }
        ParserOptions::Bangumi { config } => {
            if config.merge && config.need_video && config.need_audio {
                merger::MediaMerger::merge_av(
                    Path::new(
                        &video
                            .ok_or(ParseError::ParseError("视频文件未找到".to_string()))?
                            .output_path,
                    ),
                    Path::new(
                        &audio
                            .ok_or(ParseError::ParseError("音频文件未找到".to_string()))?
                            .output_path,
                    ),
                    &Path::new(&config.output_dir)
                        .join(
                            video
                                .as_ref()
                                .map(|p| &p.name)
                                .unwrap_or(&"video".to_string()),
                        )
                        .with_extension("mp3"),
                )
                .await
                .map_err(|e| ParseError::ParseError(format!("合并失败: {}", e)))?;
            }
        }
        _ => {
            return Err(ParseError::ParseError("不支持的下载类型".to_string()));
        }
    }
    Ok(())
}
