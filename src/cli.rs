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
    pub output: PathBuf,

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
}
