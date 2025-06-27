use clap::Parser;
use std::path::PathBuf;

/// B站视频下载器
#[derive(Parser, Debug)]
#[command(name = "bilidl")]
#[command(version = "1.0")]
#[command(author = "rpeng252@gmail.com")]
#[command(about = "一个简单的B站视频下载工具", long_about = None)]
pub struct Cli {
    /// 视频链接 (支持普通视频和番剧)
    #[arg(long, value_name = "URL")]
    #[arg(value_parser = clap::value_parser!(String))]
    #[arg(value_hint = clap::ValueHint::Url)]
    pub url: String,

    /// 登录B站账号 (需要下载高清视频时使用)
    #[arg(long)]
    #[arg(help = "使用二维码登录B站账号")]
    pub login: bool,

    /// 用户配置目录
    #[arg(long, value_name = "DIR")]
    #[arg(value_hint = clap::ValueHint::DirPath)]
    pub user_dir: Option<PathBuf>,

    /// 视频保存目录
    #[arg(long, value_name = "DIR")]
    #[arg(default_value = ".")]
    #[arg(value_hint = clap::ValueHint::DirPath)]
    pub output_dir: PathBuf,

    /// 视频质量 (1-116)
    #[arg(long)]
    #[arg(value_name = "QUALITY")]
    #[arg(default_value = "80")]
    #[arg(help = "视频质量: 116=4K, 80=1080P, 64=720P, 32=480P, 16=360P")]
    pub quality: u32,

    /// Cookie字符串 (可选)
    #[arg(long, value_name = "COOKIE")]
    #[arg(help = "手动指定Cookie")]
    pub cookie: Option<String>,

    /// 集数范围 (可选，仅用于番剧或课程的批量下载)
    #[arg(long, value_name = "RANGE")]
    #[arg(help = "指定要下载的集数范围，如: 1-3,5,7-9")]
    pub parts: Option<String>,

    #[arg(long, value_name = "是否下载视频", default_value_t = true)]
    pub need_video: bool,
    #[arg(long, value_name = "是否下载音频", default_value_t = true)]
    pub need_audio: bool,
    #[arg(long, value_name = "是否下载字幕", default_value_t = true)]
    pub need_subtitle: bool,
    #[arg(long, value_name = "是否下载弹幕", default_value_t = true)]
    pub need_danmaku: bool,
    #[arg(long, value_name = "是否合并音视频", default_value_t = true)]
    pub merge: bool,
    #[arg(long, value_name = "并发数", default_value_t = 3)]
    pub concurrency: usize,
}
