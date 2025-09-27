use crate::parser::detail_parser::models::DashItem;
use crate::parser::errors::ParseError;
use crate::parser::models::VideoQuality;
use tracing::{debug, warn};

/// 选择最佳的视频流
pub fn select_video_stream(
    streams: &[DashItem],
    resolution: VideoQuality,
) -> Result<Option<String>, ParseError> {
    if streams.is_empty() {
        return Err(ParseError::ParseError(
            "没有可用的视频流。可能原因：1. 视频需要大会员权限 2. 当前清晰度不可用 3. Cookie已过期，请重新登录".to_string()
        ));
    }

    debug!("可用的视频流数量: {}", streams.len());
    for (i, stream) in streams.iter().enumerate() {
        debug!(
            "流 {}: 清晰度ID={}, width={:?}, height={:?}",
            i, stream.id, stream.width, stream.height
        );
    }

    let target_quality_id = resolution as i32;
    debug!("目标清晰度ID: {}", target_quality_id);

    // 首先尝试精确匹配清晰度ID
    if let Some(stream) = streams.iter().find(|s| s.id == target_quality_id) {
        debug!("找到精确匹配的清晰度: ID={}", stream.id);
        return Ok(Some(stream.base_url.clone()));
    }

    // 如果没有精确匹配，选择最接近且不超过目标清晰度的流
    let mut suitable_streams: Vec<_> = streams
        .iter()
        .filter(|s| s.id <= target_quality_id)
        .collect();

    if !suitable_streams.is_empty() {
        // 按清晰度ID降序排序，选择最高的
        suitable_streams.sort_by(|a, b| b.id.cmp(&a.id));
        let selected = suitable_streams[0];
        debug!(
            "选择最接近的清晰度: ID={} (目标: {})",
            selected.id, target_quality_id
        );
        return Ok(Some(selected.base_url.clone()));
    }

    // 如果所有流的清晰度都高于目标，选择最低的
    let mut all_streams = streams.to_vec();
    all_streams.sort_by(|a, b| a.id.cmp(&b.id));
    let fallback = &all_streams[0];

    // 检查是否是高质量视频权限问题
    let highest_available_quality = all_streams.last().map(|s| s.id).unwrap_or(0);
    if target_quality_id >= 112 && highest_available_quality < target_quality_id {
        // 112是1080P+
        warn!(
            "目标清晰度 {} 可能需要大会员权限，最高可用清晰度: {}",
            target_quality_id, highest_available_quality
        );
        warn!("💡 提示：1080P+、4K等高清晰度通常需要大会员权限，请确保已登录大会员账号");
    }

    debug!("目标清晰度过低，降级到最低可用清晰度: ID={}", fallback.id);

    Ok(Some(fallback.base_url.clone()))
}

/// 选择最佳的音频流
pub fn select_audio_stream(streams: &[DashItem]) -> Result<Option<String>, ParseError> {
    if streams.is_empty() {
        return Err(ParseError::ParseError(
            "没有可用的音频流。可能原因：1. 视频源异常 2. 网络问题 3. Cookie已过期".to_string(),
        ));
    }

    debug!("可用的音频流数量: {}", streams.len());
    for (i, stream) in streams.iter().enumerate() {
        debug!(
            "音频流 {}: 清晰度ID={}, 编码={}, 带宽={}",
            i, stream.id, stream.codecs, stream.bandwidth
        );
    }

    // 按音频质量（带宽）降序排序，选择最高质量的音频
    let mut sorted_streams = streams.to_vec();
    sorted_streams.sort_by(|a, b| b.bandwidth.cmp(&a.bandwidth));

    let selected = &sorted_streams[0];
    debug!(
        "选择最高质量音频流: ID={}, 带宽={}",
        selected.id, selected.bandwidth
    );

    Ok(Some(selected.base_url.clone()))
}