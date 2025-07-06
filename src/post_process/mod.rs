pub mod merger;

use tracing::debug;

use crate::{
    downloader::models::{DownloadTask, FileType},
    parser::{detail_parser::parser_trait::ParserOptions, errors::ParseError},
};
use std::path::Path;

pub async fn post_process(
    task: &Vec<DownloadTask>,
    parser_options: &ParserOptions,
) -> Result<(), ParseError> {
    debug!("开始后处理，总任务数: {}", task.len());
    
    // 按集数或名称分组处理任务
    let task_groups = group_tasks_by_episode(task);
    debug!("分组后的任务组数: {}", task_groups.len());
    
    for (episode_key, episode_tasks) in task_groups {
        debug!("处理集数/组: {}", episode_key);
        process_single_episode(&episode_tasks, parser_options).await?;
    }
    
    debug!("所有集数后处理完成");
    Ok(())
}

/// 按集数分组任务
fn group_tasks_by_episode(tasks: &[DownloadTask]) -> std::collections::HashMap<String, Vec<&DownloadTask>> {
    use std::collections::HashMap;
    
    let mut groups: HashMap<String, Vec<&DownloadTask>> = HashMap::new();
    
    debug!("开始分组 {} 个任务:", tasks.len());
    
    for task in tasks {
        // 从任务名称中提取集数标识
        let episode_key = extract_episode_key(&task.name);
        debug!("任务 '{}' ({:?}) 分组到: '{}'", task.name, task.file_type, episode_key);
        groups.entry(episode_key).or_default().push(task);
    }
    
    // 打印分组统计
    debug!("\n========== 分组统计 ==========");
    for (key, tasks) in &groups {
        debug!("分组 '{}' 包含 {} 个任务:", key, tasks.len());
        let mut video_count = 0;
        let mut audio_count = 0;
        let mut other_count = 0;
        
        for task in tasks {
            match task.file_type {
                FileType::Video => {
                    video_count += 1;
                    debug!("  📹 视频: {}", task.name);
                }
                FileType::Audio => {
                    audio_count += 1;
                    debug!("  🎵 音频: {}", task.name);
                }
                _ => {
                    other_count += 1;
                    debug!("  📄 其它: {} ({:?})", task.name, task.file_type);
                }
            }
        }
        
        debug!("    统计: 视频{}个, 音频{}个, 其它{}个", video_count, audio_count, other_count);
        
        // 检查是否为DASH格式（需要合并）
        let is_dash = video_count > 0 && audio_count > 0;
        let is_durl = (video_count > 0 && audio_count == 0) || (video_count == 0 && audio_count > 0);
        debug!("    格式预判: DASH={}, DURL={}", is_dash, is_durl);
    }
    debug!("==============================\n");
    
    groups
}

/// 从任务名称中提取集数标识
fn extract_episode_key(name: &str) -> String {
    use regex::Regex;
    
    // 移除音视频后缀标识，但保留核心标识信息
    let clean_name = name
        .replace("-video.mp4", "")
        .replace("-audio.m4s", "")
        .replace("-video", "")
        .replace("-audio", "")
        .replace("_video", "")
        .replace("_audio", "")
        .replace(".mp4", "")
        .replace(".m4s", "")
        .replace(".xml", "");
    
    debug!("清理后的名称: '{}'", clean_name);
    
    // 尝试提取集数编号的各种模式
    let patterns = vec![
        // 番剧格式：{title} - {episode_title} - {id}
        r"^(.+?)\s*-\s*(.+?)\s*-\s*\d+$",  // "番剧名 - 第1话 - 123456"
        // 集数编号模式
        r"第(\d+)话",      // 第01话
        r"第(\d+)集",      // 第01集
        r"EP(\d+)",        // EP01
        r"ep(\d+)",        // ep01
        r"(\d+)\.",        // 01.
        r"_(\d+)_",        // _01_
        r"-(\d+)-",        // -01-
        r"\[(\d+)\]",      // [01]
        r"S\d+E(\d+)",     // S01E01
        r"P(\d+)",         // P1, P2 (分P视频)
        r"p(\d+)",         // p1, p2
    ];
    
    // 先尝试番剧格式提取
    if let Ok(re) = Regex::new(r"^(.+?)\s*-\s*(.+?)\s*-\s*\d+$") {
        if let Some(captures) = re.captures(&clean_name) {
            let title = captures.get(1).map(|m| m.as_str()).unwrap_or("");
            let episode_title = captures.get(2).map(|m| m.as_str()).unwrap_or("");
            
            // 从episode_title中提取集数
            for pattern in &["第(\\d+)话", "第(\\d+)集", "EP(\\d+)", "ep(\\d+)", "(\\d+)\\."] {
                if let Ok(ep_re) = Regex::new(pattern) {
                    if let Some(ep_captures) = ep_re.captures(episode_title) {
                        if let Some(episode_num) = ep_captures.get(1) {
                            let ep_num = episode_num.as_str().parse::<u32>().unwrap_or(0);
                            return format!("{}::Episode_{:02}", title.trim(), ep_num);
                        }
                    }
                }
            }
            
            // 如果没有找到集数，使用episode_title作为标识
            return format!("{}::{}", title.trim(), episode_title.trim());
        }
    }
    
    // 然后尝试其他集数模式
    for pattern in &patterns[1..] { // 跳过番剧格式
        if let Ok(re) = Regex::new(pattern) {
            if let Some(captures) = re.captures(&clean_name) {
                if let Some(episode_num) = captures.get(1) {
                    let ep_num = episode_num.as_str().parse::<u32>().unwrap_or(0);
                    // 如果找到了集数，提取基础名称
                    let base_name = re.replace(&clean_name, "").trim().to_string();
                    if !base_name.is_empty() {
                        return format!("{}::Episode_{:02}", base_name, ep_num);
                    } else {
                        return format!("Episode_{:02}", ep_num);
                    }
                }
            }
        }
    }
    
    // 如果没有找到明显的集数标识，使用清理后的名称作为唯一标识
    debug!("未找到集数标识，使用完整名称: '{}'", clean_name);
    clean_name
}

/// 处理单个集数的任务
async fn process_single_episode(
    episode_tasks: &[&DownloadTask],
    parser_options: &ParserOptions,
) -> Result<(), ParseError> {
    debug!("\n========== 处理单集任务 ==========");
    
    let video_tasks: Vec<&DownloadTask> = episode_tasks.iter().filter(|t| t.file_type == FileType::Video).copied().collect();
    let audio_tasks: Vec<&DownloadTask> = episode_tasks.iter().filter(|t| t.file_type == FileType::Audio).copied().collect();
    let other_tasks: Vec<&DownloadTask> = episode_tasks.iter().filter(|t| t.file_type != FileType::Video && t.file_type != FileType::Audio).copied().collect();

    debug!("找到视频任务: {}个", video_tasks.len());
    for video in &video_tasks {
        debug!("  📹 {}", video.name);
    }
    
    debug!("找到音频任务: {}个", audio_tasks.len());
    for audio in &audio_tasks {
        debug!("  🎵 {}", audio.name);
    }
    
    debug!("找到其它任务: {}个", other_tasks.len());
    for other in &other_tasks {
        debug!("  📄 {} ({:?})", other.name, other.file_type);
    }

    // 判断是DASH格式（需要合并）还是DURL格式（已合并）
    let is_dash_format = !video_tasks.is_empty() && !audio_tasks.is_empty();
    let is_durl_format = (!video_tasks.is_empty() && audio_tasks.is_empty()) || (video_tasks.is_empty() && !audio_tasks.is_empty());

    debug!("格式确认: DASH={}, DURL={}", is_dash_format, is_durl_format);

    match parser_options {
        ParserOptions::CommonVideo { config } => {
            debug!("使用普通视频配置处理");
            handle_media_processing(
                video_tasks.first().copied(), 
                audio_tasks.first().copied(), 
                config, 
                is_dash_format, 
                is_durl_format
            ).await?;
        }
        ParserOptions::Bangumi { config } => {
            debug!("使用番剧配置处理");
            handle_media_processing(
                video_tasks.first().copied(), 
                audio_tasks.first().copied(), 
                config, 
                is_dash_format, 
                is_durl_format
            ).await?;
        }
        ParserOptions::Course { .. } => {
            debug!("使用课程配置处理");
            // 课程视频通常是单一流，直接移动
            if let Some(video_task) = video_tasks.first() {
                move_single_file(video_task, "课程视频").await?;
            } else if let Some(audio_task) = audio_tasks.first() {
                move_single_file(audio_task, "课程音频").await?;
            }
        }
    }
    
    // 处理其它类型的文件（如弹幕）
    for other_task in &other_tasks {
        debug!("移动其它文件: {} ({:?})", other_task.name, other_task.file_type);
        move_single_file(other_task, &format!("{:?}", other_task.file_type)).await?;
    }
    
    debug!("================================\n");
    Ok(())
}

/// 处理媒体文件的合并或移动
async fn handle_media_processing(
    video: Option<&DownloadTask>,
    audio: Option<&DownloadTask>,
    config: &crate::parser::detail_parser::models::DownloadConfig,
    is_dash_format: bool,
    is_durl_format: bool,
) -> Result<(), ParseError> {
    debug!("\n========== 媒体处理 ==========");
    debug!("配置: merge={}, need_video={}, need_audio={}", config.merge, config.need_video, config.need_audio);
    debug!("格式: DASH={}, DURL={}", is_dash_format, is_durl_format);
    
    if is_dash_format && config.merge && config.need_video && config.need_audio {
        // DASH格式：需要合并video和audio
        debug!("执行DASH格式音视频合并");
        
        let video_task = video.ok_or(ParseError::ParseError("视频文件未找到".to_string()))?;
        let audio_task = audio.ok_or(ParseError::ParseError("音频文件未找到".to_string()))?;
        
        debug!("合并源文件:");
        debug!("  📹 视频: {}", video_task.output_path);
        debug!("  🎵 音频: {}", audio_task.output_path);
        
        // 构造输出文件名 - 使用视频任务的名称，但去除后缀
        let output_name = clean_filename_for_output(&video_task.name);
        let output_path = Path::new(&config.output_dir)
            .join(&output_name)
            .with_extension("mp4");
            
        debug!("  🎬 输出: {:?}", output_path);
        
        merger::MediaMerger::merge_av(
            Path::new(&video_task.output_path),
            Path::new(&audio_task.output_path),
            &output_path,
        )
        .await
        .map_err(|e| ParseError::ParseError(format!("合并失败: {}", e)))?;
        
        debug!("✅ DASH格式合并完成");
    } else if is_durl_format {
        // DURL格式：已经是合并的流，只需要移动到目标位置
        debug!("执行DURL格式文件移动");
        if let Some(video_task) = video {
            move_file_to_output(video_task, config, "DURL视频").await?;
        } else if let Some(audio_task) = audio {
            move_file_to_output(audio_task, config, "DURL音频").await?;
        }
        debug!("✅ DURL格式文件移动完成");
    } else {
        debug!("⏭️  跳过后处理：不满足处理条件");
        debug!("   条件检查: DASH需要merge=true + need_video=true + need_audio=true");
        debug!("   条件检查: DURL需要单独的视频或音频流");
    }
    debug!("==========================\n");
    Ok(())
}

/// 清理文件名以用于输出
fn clean_filename_for_output(name: &str) -> String {
    name
        .replace("-video.mp4", "")
        .replace("-audio.m4s", "")
        .replace("-video", "")
        .replace("-audio", "")
        .replace("_video", "")
        .replace("_audio", "")
        .replace(".mp4", "")
        .replace(".m4s", "")
        .replace(".xml", "")
        .trim()
        .to_string()
}

/// 移动文件到输出目录
async fn move_file_to_output(
    task: &DownloadTask,
    config: &crate::parser::detail_parser::models::DownloadConfig,
    file_type: &str,
) -> Result<(), ParseError> {
    use tokio::fs;
    use std::path::Path;
    
    let source_path = Path::new(&task.output_path);
    
    if !source_path.exists() {
        return Err(ParseError::ParseError(format!("源文件不存在: {:?}", source_path)));
    }
    
    // 构造目标文件名
    let output_name = clean_filename_for_output(&task.name);
    let extension = source_path.extension().and_then(|ext| ext.to_str()).unwrap_or("mp4");
    let target_path = Path::new(&config.output_dir)
        .join(&output_name)
        .with_extension(extension);
    
    // 确保目标目录存在
    if let Some(target_dir) = target_path.parent() {
        fs::create_dir_all(target_dir).await
            .map_err(|e| ParseError::ParseError(format!("创建目录失败: {}", e)))?;
    }
    
    debug!("移动{}文件:", file_type);
    debug!("  从: {:?}", source_path);
    debug!("  到: {:?}", target_path);
    
    // 如果目标文件已存在，创建唯一文件名
    let final_target = if target_path.exists() {
        let mut counter = 1;
        loop {
            let stem = target_path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
            let ext = target_path.extension().and_then(|s| s.to_str()).unwrap_or("");
            let new_name = if ext.is_empty() {
                format!("{}_{}", stem, counter)
            } else {
                format!("{}_{}.{}", stem, counter, ext)
            };
            let new_path = target_path.with_file_name(new_name);
            if !new_path.exists() {
                break new_path;
            }
            counter += 1;
        }
    } else {
        target_path
    };
    
    // 执行文件移动
    fs::rename(source_path, &final_target).await
        .map_err(|e| ParseError::ParseError(format!("文件移动失败: {}", e)))?;
    
    debug!("✅ {}文件移动成功: {:?}", file_type, final_target);
    Ok(())
}

/// 移动单个文件到目标位置（通用版本，无需配置）
async fn move_single_file(task: &DownloadTask, file_type: &str) -> Result<(), ParseError> {
    use std::path::Path;
    
    let source_path = Path::new(&task.output_path);
    
    if !source_path.exists() {
        debug!("⚠️  源文件不存在，跳过移动: {:?}", source_path);
        return Ok(());
    }
    
    // 对于通用移动，保持文件在原位置
    debug!("✅ {}文件保持原位置: {:?}", file_type, source_path);
    Ok(())
}
