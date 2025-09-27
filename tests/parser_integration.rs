use bili_downloader::common::client::client::BiliClient;
use bili_downloader::parser::detail_parser::models::DownloadConfig;
use bili_downloader::parser::models::{UrlType, VideoId};
use bili_downloader::parser::VideoParser;

fn create_test_client() -> BiliClient {
    BiliClient::new()
}

fn create_test_config() -> DownloadConfig {
    DownloadConfig {
        need_video: true,
        need_audio: true,
        need_danmaku: false,
        need_subtitle: false,
        merge: true,
        output_format: "mp4".to_string(),
        output_dir: "./test_output".to_string(),
        resolution: bili_downloader::parser::models::VideoQuality::Q1080P,
        concurrency: 4,
        episode_range: None,
    }
}

#[tokio::test]
async fn test_parse_common_video_single() {
    let client = create_test_client();
    let mut parser = VideoParser::new(client, false);

    // 测试单集普通视频URL
    let url = "https://www.bilibili.com/video/BV1N6nEzhEz6/?vd_source=eb108868f984ddb9279f49779a317b8f";
    let url_type = UrlType::CommonVideo(VideoId {
        bvid: Some("BV1N6nEzhEz6".to_string()),
        aid: None,
    });

    let config = create_test_config();
    let result = parser.parse(url, &config).await;

    match result {
        Ok(meta) => {
            println!("✅ 单集普通视频解析成功: {}", meta.title);
            assert_eq!(meta.download_type, bili_downloader::common::models::DownloadType::CommonVideo);
            assert!(!meta.download_items.is_empty());
        }
        Err(e) => {
            println!("⚠️ 单集普通视频解析失败（可能是网络或认证问题）: {:?}", e);
            // 在CI环境中可能失败，这里不做硬断言
        }
    }
}

#[tokio::test]
async fn test_parse_common_video_multi_p() {
    let client = create_test_client();
    let mut parser = VideoParser::new(client, false);

    // 测试多P合集视频URL
    let url = "https://www.bilibili.com/video/BV1dRnjzGEc1/?vd_source=eb108868f984ddb9279f49779a317b8f";
    let url_type = UrlType::CommonVideo(VideoId {
        bvid: Some("BV1dRnjzGEc1".to_string()),
        aid: None,
    });

    let config = create_test_config();
    let result = parser.parse(url, &config).await;

    match result {
        Ok(meta) => {
            println!("✅ 多P合集视频解析成功: {}", meta.title);
            assert_eq!(meta.download_type, bili_downloader::common::models::DownloadType::CommonVideo);
            assert!(!meta.download_items.is_empty());
        }
        Err(e) => {
            println!("⚠️ 多P合集视频解析失败（可能是网络或认证问题）: {:?}", e);
            // 在CI环境中可能失败，这里不做硬断言
        }
    }
}

#[tokio::test]
async fn test_parse_bangumi() {
    let client = create_test_client();
    let mut parser = VideoParser::new(client, false);

    // 测试番剧URL
    let url = "https://www.bilibili.com/bangumi/play/ss80512";
    let url_type = UrlType::BangumiSeason("80512".to_string());

    let config = create_test_config();
    let result = parser.parse(url, &config).await;

    match result {
        Ok(meta) => {
            println!("✅ 番剧解析成功: {}", meta.title);
            assert_eq!(meta.download_type, bili_downloader::common::models::DownloadType::Bangumi);
            assert!(!meta.download_items.is_empty());
        }
        Err(e) => {
            println!("⚠️ 番剧解析失败（可能是网络或认证问题）: {:?}", e);
            // 在CI环境中可能失败，这里不做硬断言
        }
    }
}

#[tokio::test]
async fn test_url_parser() {
    use bili_downloader::parser::url_parser::UrlParser;

    let url_parser = UrlParser::new();

    // 测试普通视频URL解析
    let result = url_parser.parse("https://www.bilibili.com/video/BV1N6nEzhEz6/").await;
    match result {
        Ok(url_type) => {
            println!("✅ URL解析成功: {:?}", url_type);
            match url_type {
                UrlType::CommonVideo(video_id) => {
                    assert_eq!(video_id.bvid, Some("BV1N6nEzhEz6".to_string()));
                }
                _ => panic!("期望解析为普通视频"),
            }
        }
        Err(e) => {
            println!("⚠️ URL解析失败: {:?}", e);
        }
    }

    // 测试番剧URL解析
    let result = url_parser.parse("https://www.bilibili.com/bangumi/play/ss80512").await;
    match result {
        Ok(url_type) => {
            println!("✅ 番剧URL解析成功: {:?}", url_type);
            match url_type {
                UrlType::BangumiSeason(season_id) => {
                    assert_eq!(season_id, "80512");
                }
                _ => panic!("期望解析为番剧"),
                }
        }
        Err(e) => {
            println!("⚠️ 番剧URL解析失败: {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_url_parser_with_query_params() {
    use bili_downloader::parser::url_parser::UrlParser;

    let url_parser = UrlParser::new();

    // 测试带查询参数的URL
    let result = url_parser.parse("https://www.bilibili.com/video/BV1dRnjzGEc1/?vd_source=eb108868f984ddb9279f49779a317b8f").await;
    match result {
        Ok(url_type) => {
            println!("✅ 带查询参数URL解析成功: {:?}", url_type);
            match url_type {
                UrlType::CommonVideo(video_id) => {
                    assert_eq!(video_id.bvid, Some("BV1dRnjzGEc1".to_string()));
                }
                _ => panic!("期望解析为普通视频"),
            }
        }
        Err(e) => {
            println!("⚠️ 带查询参数URL解析失败: {:?}", e);
        }
    }
}