use clap::Parser;

#[derive(Parser)]
#[command(name="BiliDL")]
#[command(version = "1.0")]
#[command(about = "B站视频下载器", long_about = None)]
pub struct Cli {
    // 是否登录
    #[arg(long, value_name = "Login")]
    pub login: bool,

    // 用户信息存放目录
    #[arg(long, value_name = "User Info Dir")]
    #[arg(value_parser = clap::value_parser!(String), value_hint = clap::ValueHint::DirPath)]
    pub user_dir: Option<String>,

    // 视频链接
    #[arg(long, value_name = "URL", default_value = "")]
    #[arg(value_parser = clap::value_parser!(String), value_hint = clap::ValueHint::Url)]
    pub url: String,

    // 保存目录（默认当前目录）
    #[arg(long, default_value = ".")]
    #[arg(value_hint = clap::ValueHint::DirPath)]
    pub output: String,

    // 视频清洗等级
    #[arg(long, default_value = "80")]
    pub quality: u32,

    #[arg(long)]
    pub cookie: Option<String>
}
