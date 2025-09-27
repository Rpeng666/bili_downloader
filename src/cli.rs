use clap::{Parser, ValueEnum};
use std::path::PathBuf;

/// 视频清晰度选项
#[derive(Debug, Clone, ValueEnum)]
pub enum QualityOption {
    /// 流畅 360P
    #[value(name = "360p")]
    Q360P,
    /// 清晰 480P
    #[value(name = "480p")]
    Q480P,
    /// 高清 720P
    #[value(name = "720p")]
    Q720P,
    /// 高清 720P60
    #[value(name = "720p60")]
    Q720P60,
    /// 高清 1080P
    #[value(name = "1080p")]
    Q1080P,
    /// 高清 1080P+
    #[value(name = "1080p+")]
    Q1080PP,
    /// 高清 1080P60
    #[value(name = "1080p60")]
    Q1080P60,
    /// 超清 4K
    #[value(name = "4k")]
    Q4K,
    /// HDR 真彩色
    #[value(name = "hdr")]
    QHdr,
    /// 超高清 8K
    #[value(name = "8k")]
    Q8K,
}

/// B站视频下载器 - 支持下载B站视频、番剧、课程等内容
#[derive(Parser, Debug)]
#[command(name = "bilidl")]
#[command(version = "1.0")]
#[command(author = "rpeng252@gmail.com")]
#[command(about = "B站视频下载工具 - 支持高清视频、番剧、课程下载", long_about = r#"
B站视频下载器 (bilidl) - 一个功能强大的B站内容下载工具

功能特性:
• 支持下载普通视频、番剧、课程等多种内容
• 支持多种视频质量选择 (360P 到 8K)
• 支持批量下载和选择性下载
• 支持登录下载高清/付费内容
• 自动音视频合并 (需要FFmpeg)
• 支持弹幕和字幕下载

使用示例:
  # 下载单个视频
  bilidl --url "https://www.bilibili.com/video/BV1xx411x7x1"

  # 下载高清视频 (需要先登录)
  bilidl --login
  bilidl --url "https://www.bilibili.com/video/BV1xx411x7x1" --quality 1080p

  # 下载番剧指定集数
  bilidl --url "https://www.bilibili.com/bangumi/play/ss12345" --parts "1-3,5"

  # 仅登录保存认证信息
  bilidl --login

  # 启动MCP服务器 (stdio模式)
  bilidl --mcp

  # 启动HTTP MCP服务器
  bilidl --http 127.0.0.1:3000

注意事项:
• 下载高清/付费内容需要先使用 --login 进行登录
• 首次使用会自动下载FFmpeg用于音视频合并
• 支持的URL格式: 视频/av/bv, 番剧/ss/ep, 课程/cheese
• MCP服务器支持AI助手集成，提供视频下载API
"#)]
pub struct Cli {
    /// 视频/番剧/课程链接 (支持多种B站URL格式)
    #[arg(long, value_name = "URL")]
    #[arg(value_parser = clap::value_parser!(String))]
    #[arg(value_hint = clap::ValueHint::Url)]
    #[arg(help = r#"B站内容链接，支持以下格式:
• 普通视频: https://www.bilibili.com/video/BVxxx 或 https://www.bilibili.com/video/avxxx
• 番剧: https://www.bilibili.com/bangumi/play/ssxxx 或 https://www.bilibili.com/bangumi/play/epxxx
• 课程: https://www.bilibili.com/cheese/play/ssxxx"#)]
    pub url: Option<String>,

    /// 登录B站账号 (用于下载高清/付费内容)
    #[arg(long)]
    #[arg(help = r#"使用二维码扫描登录B站账号
登录后可以下载更高清的视频和付费内容
登录信息会保存到本地，下次使用时无需重新登录"#)]
    pub login: bool,

    /// 用户配置目录 (存放登录信息等)
    #[arg(long, value_name = "DIR")]
    #[arg(value_hint = clap::ValueHint::DirPath)]
    #[arg(help = "指定存放用户配置和登录信息的目录，默认使用系统配置目录")]
    pub user_dir: Option<PathBuf>,

    /// 视频保存目录
    #[arg(long, value_name = "DIR")]
    #[arg(default_value = ".")]
    #[arg(value_hint = clap::ValueHint::DirPath)]
    #[arg(help = "下载文件的保存目录，默认为当前目录")]
    pub output_dir: PathBuf,

    /// 视频质量选择
    #[arg(long)]
    #[arg(value_enum)]
    #[arg(default_value = "1080p")]
    #[arg(help = r#"选择视频下载质量:
• 360p/480p/720p: 基础质量，无需登录
• 720p60/1080p/1080p+/1080p60: 高清质量，建议登录后使用
• 4k/8k: 超高清质量，需要登录且视频支持
• hdr: HDR质量，特殊设备支持"#)]
    pub quality: QualityOption,

    /// Cookie字符串 (高级用户选项)
    #[arg(long, value_name = "COOKIE")]
    #[arg(help = "手动指定B站Cookie字符串，通常不需要手动设置")]
    pub cookie: Option<String>,

    /// 集数范围 (仅用于番剧/课程批量下载)
    #[arg(long, value_name = "RANGE")]
    #[arg(help = r#"指定下载的集数范围，格式示例:
• "1-5": 下载第1到5集
• "1,3,5": 下载第1、3、5集
• "1-3,5-7": 下载第1-3集和第5-7集
• 不指定则下载全部集数"#)]
    pub parts: Option<String>,

    /// 是否下载视频流
    #[arg(long)]
    #[arg(help = "是否下载视频画面，默认启用")]
    #[arg(default_value = "true")]
    pub need_video: bool,

    /// 是否下载音频流
    #[arg(long)]
    #[arg(help = "是否下载音频，默认启用")]
    #[arg(default_value = "true")]
    pub need_audio: bool,

    /// 是否下载字幕
    #[arg(long)]
    #[arg(help = "是否下载字幕文件 (如果视频有字幕)")]
    pub need_subtitle: bool,

    /// 是否下载弹幕
    #[arg(long)]
    #[arg(help = "是否下载弹幕文件，默认启用")]
    pub need_danmaku: bool,

    /// 是否合并音视频
    #[arg(long)]
    #[arg(help = "下载完成后是否自动合并音视频文件 (需要FFmpeg)，默认启用")]
    #[arg(default_value = "true")]
    pub merge: bool,

    /// 下载并发数
    #[arg(long, value_name = "NUM")]
    #[arg(default_value_t = 3)]
    #[arg(help = "同时下载的线程数，建议1-8之间，默认3")]
    pub concurrency: usize,

    /// 启动MCP服务器模式 (开发者选项)
    #[arg(long)]
    #[arg(help = "启动MCP (Model Context Protocol) 服务器模式，用于AI助手集成")]
    pub mcp: bool,
}
