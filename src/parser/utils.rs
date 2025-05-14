use serde_json::Value;

use super::models::ParseType;

pub struct FormatTool;

impl FormatTool {
    // 格式化时长
    pub fn format_duration(episode: &Value, flag: ParseType) -> String {
        let duration = match flag {
            ParseType::Video => {
                if let Some(arc) = episode.get("arc") {
                    arc.get("duration").and_then(Value::as_f64).unwrap_or(0.0)
                } else {
                    episode.get("duration").and_then(Value::as_f64).unwrap_or(0.0)
                }
            }
            ParseType::Bangumi => {
                if let Some(dur) = episode.get("duration") {
                    dur.as_f64().unwrap_or(0.0) / 1000.0
                } else {
                    return "--:--".to_string();
                }
            }
        };

        let hours = (duration / 3600.0).floor() as u32;
        let mins = ((duration - (hours * 3600) as f64) / 60.0).floor() as u32;
        let secs = (duration - (hours * 3600 + mins * 60) as f64).round() as u32;

        if hours != 0 {
            format!("{:02}:{:02}:{:02}", hours, mins, secs)
        } else {
            format!("{:02}:{:02}", mins, secs)
        }
    }

    // 格式化下载速度
    pub fn format_speed(speed: u64) -> String {
        if speed > 1024 * 1024 * 1024 {
            format!("{:.1} GB/s", speed as f64 / 1024.0 / 1024.0 / 1024.0)
        } else if speed > 1024 * 1024 {
            format!("{:.1} MB/s", speed as f64 / 1024.0 / 1024.0)
        } else if speed > 1024 {
            format!("{:.1} KB/s", speed as f64 / 1024.0)
        } else {
            "0 KB/s".to_string()
        }
    }

    // 格式化文件大小
    pub fn format_size(size: u64) -> String {
        if size == 0 {
            "0 MB".to_string()
        } else if size > 1024 * 1024 * 1024 {
            format!("{:.2} GB", size as f64 / 1024.0 / 1024.0 / 1024.0)
        } else if size > 1024 * 1024 {
            format!("{:.1} MB", size as f64 / 1024.0 / 1024.0)
        } else {
            format!("{:.1} KB", size as f64 / 1024.0)
        }
    }

    // // 格式化剧集标题
    // pub fn format_bangumi_title(episode: &Value, main_episode: bool) -> String {
    //     use crate::utils::parse::bangumi::BangumiInfo;
    //     use crate::config::Config;

    //     if BangumiInfo::TYPE_ID == 2 && main_episode {
    //         format!("《{}》{}", BangumiInfo::TITLE, episode["title"].as_str().unwrap_or(""))
    //     } else {
    //         if let Some(copy) = episode.get("share_copy") {
    //             if Config::MISC.show_episode_full_name {
    //                 return copy.as_str().unwrap_or("").to_string();
    //             } else {
    //                 for key in &["show_title", "long_title"] {
    //                     if let Some(val) = episode.get(*key) {
    //                         if let Some(s) = val.as_str() {
    //                             if !s.is_empty() {
    //                                 return s.to_string();
    //                             }
    //                         }
    //                     }
    //                 }
    //                 return copy.as_str().unwrap_or("").to_string();
    //             }
    //         } else {
    //             return episode["report"]["ep_title"].as_str().unwrap_or("").to_string();
    //         }
    //     }
    // }

    // 格式化数据量
    pub fn format_data_count(data: u64) -> String {
        if data >= 100_000_000 {
            format!("{:.1}亿", data as f64 / 1e8)
        } else if data >= 10_000 {
            format!("{:.1}万", data as f64 / 1e4)
        } else {
            data.to_string()
        }
    }

    // 格式化带宽
    pub fn format_bandwidth(bandwidth: u64) -> String {
        if bandwidth > 1024 * 1024 {
            format!("{:.1} mbps", bandwidth as f64 / 1024.0 / 1024.0)
        } else {
            format!("{:.1} kbps", bandwidth as f64 / 1024.0)
        }
    }
}
