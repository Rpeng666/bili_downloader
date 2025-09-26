use colored::*;

/// 漂亮的日志输出工具
pub struct PrettyLogger;

impl PrettyLogger {
    /// 显示成功消息
    pub fn success(message: impl AsRef<str>) {
        println!("{} {}", "✓".green().bold(), message.as_ref());
    }

    /// 显示信息消息
    pub fn info(message: impl AsRef<str>) {
        println!("{} {}", "ℹ".blue().bold(), message.as_ref());
    }

    /// 显示警告消息
    pub fn warning(message: impl AsRef<str>) {
        println!("{} {}", "⚠".yellow().bold(), message.as_ref());
    }

    /// 显示错误消息
    pub fn error(message: impl AsRef<str>) {
        println!("{} {}", "✗".red().bold(), message.as_ref());
    }

    /// 显示步骤开始
    pub fn step_start(step: impl AsRef<str>) {
        println!("\n{} {}", "▶".cyan().bold(), step.as_ref().bold());
    }

    /// 显示步骤完成
    pub fn step_complete(step: impl AsRef<str>) {
        println!("{} {}", "✓".green().bold(), step.as_ref().green());
    }

    /// 显示下载进度
    pub fn download_progress(filename: impl AsRef<str>, progress: f64) {
        let bar_width = 30;
        let filled = (progress * bar_width as f64) as usize;
        let empty = bar_width - filled;

        let bar = format!(
            "[{}{}] {:.1}%",
            "█".repeat(filled).green(),
            "░".repeat(empty).bright_black(),
            progress * 100.0
        );

        println!("{} {} {}", "⬇".blue().bold(), filename.as_ref(), bar);
    }

    /// 显示文件信息
    pub fn file_info(label: impl AsRef<str>, path: impl AsRef<str>) {
        println!("{} {}: {}", "📁".blue().bold(), label.as_ref().bold(), path.as_ref());
    }

    /// 显示用户状态
    pub fn user_status(status: impl AsRef<str>, details: impl AsRef<str>) {
        println!("{} {} - {}", "👤".green().bold(), status.as_ref().bold(), details.as_ref());
    }

    /// 显示视频信息
    pub fn video_info(title: impl AsRef<str>, quality: impl AsRef<str>) {
        println!("{} {} ({})", "🎬".magenta().bold(), title.as_ref().bold(), quality.as_ref().cyan());
    }

    /// 显示分割线
    pub fn separator() {
        println!("{}", "─".repeat(50).bright_black());
    }

    /// 显示标题
    pub fn title(text: impl AsRef<str>) {
        let text = text.as_ref();
        let padding = (48 - text.len()) / 2;
        let line = "─".repeat(padding);
        println!("{} {} {}", line.bright_black(), text.bold(), "─".repeat(48 - padding - text.len()).bright_black());
    }

    /// 显示完成总结
    pub fn completion_summary(items: Vec<impl AsRef<str>>) {
        println!("\n{}", "🎉 下载完成！".green().bold());
        for item in items {
            println!("  {}", item.as_ref());
        }
    }

    /// 显示登录提示
    pub fn login_prompt() {
        println!("\n{}", "🔐 请使用手机扫描二维码登录".cyan().bold());
        println!("{}", "   扫描完成后按 Enter 键继续...".bright_black());
    }

    /// 显示等待消息
    pub fn waiting(message: impl AsRef<str>) {
        println!("{} {}", "⏳".yellow().bold(), message.as_ref());
    }
}

/// 便捷宏用于漂亮的日志输出
#[macro_export]
macro_rules! log_success {
    ($($arg:tt)*) => {
        $crate::common::logger::PrettyLogger::success(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::common::logger::PrettyLogger::info(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_warning {
    ($($arg:tt)*) => {
        $crate::common::logger::PrettyLogger::warning(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::common::logger::PrettyLogger::error(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_step {
    ($($arg:tt)*) => {
        $crate::common::logger::PrettyLogger::step_start(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_complete {
    ($($arg:tt)*) => {
        $crate::common::logger::PrettyLogger::step_complete(format!($($arg)*))
    };
}