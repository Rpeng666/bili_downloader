# BiliDownloader

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust Version](https://img.shields.io/badge/rust-1.75%2B-orange.svg)
![GitHub release (latest by date)](https://img.shields.io/github/v/release/Rpeng666/bili_downloader)
![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/Rpeng666/bili_downloader/release.yml)

🚀 一个使用Rust编写的bilibili命令行下载器。极致小巧（<10MB),  开箱即食。
> 来都来了，不给个star鼓励一下嘛？Thanks♪(･ω･)ﾉ

![img](./docs/333.png)

## ✨ 特性(画饼中)

- 🔒 **多种登录方式**
  - [x] 二维码扫码登录（自动显示登录二维码）
  - [x] Cookie 文本登录（支持手动输入Cookie）
  - [x] 本地状态保存（自动记住登录信息）
- 🚄 **高效下载引擎**
  - [x] 自动音视频合并（DASH格式）
  - [x] 实时进度显示（清晰的下载状态）
  - [x] 断点续传支持（防止下载中断）
  - [x] 智能重试机制（网络异常自动重试）
  - [x] 压缩内容自动解压（deflate/gzip）
  - [x] 风控检测与处理（403 Forbidden智能分析）
- 🎯 **智能视频解析**
  - [x] 支持多种类型
    - [x] 单个普通视频
    - [ ] 番剧（单集/整季）
    - [ ] 课程视频
    - [x] 弹幕、字幕下载
  - [x] 支持 DASH 和 MP4 格式
  - [x] 友好的清晰度选择（360p到8k）
  - [x] 集数范围选择（如：1-5,7,9-12）
- 🛠 **人性化设计**
  - [x] 简洁的命令行界面
  - [x] 详细的日志输出和错误分析
  - [x] 灵活的配置选项
  - [x] 友好的错误提示和解决建议

## 📝 命令行参数

```bash
用法: BiliDL [选项]

选项：
    --url <URL>             视频链接 (支持普通视频和番剧)
    --output <DIR>          视频保存目录 [默认: .]
    --quality <QUALITY>     视频清晰度: 360p/480p/720p/720p60/1080p/1080p+/1080p60/4k/hdr/8k [默认: 1080p]
    --login                 登录B站账号 (需要下载高清视频时使用)
    --user-dir <DIR>        用户配置目录，用于保存登录状态
    --cookie <COOKIE>       手动指定Cookie (可选)
    --parts <RANGE>         指定要下载的集数范围，如: 1-3,5,7-9 (番剧/课程适用)
    --help                  显示帮助信息
    --version              显示版本信息
```

## 💡 使用示例

1. 简单下载视频：

```bash
bilidl --url "https://www.bilibili.com/video/BVxxxxxx"
```

2. 指定下载目录和质量：

```bash
bilidl --url "https://www.bilibili.com/video/BVxxxxxx" --output-dir "D:/Videos" --quality 4k
```

3. 使用登录下载高清视频：

```bash
# 首次使用需要登录
bilidl --login --user-dir "./config"

# 之后可以直接使用保存的登录状态
bilidl --url "https://www.bilibili.com/video/BVxxxxxx" --user-dir "./config" --quality 1080p60
```

4. 下载番剧指定集数：

```bash
# 下载第1-5集
bilidl --url "https://www.bilibili.com/bangumi/play/ss12345" --parts "1-5" --quality 1080p

# 下载第1,3,5集
bilidl --url "https://www.bilibili.com/bangumi/play/ss12345" --parts "1,3,5" --quality 720p
```

## 📥 快速开始

### 下载预编译版本

访问 [Releases](https://github.com/Rpeng666/bili_downloader/releases) 页面，下载适合您系统的最新版本：

- Windows: `BiliDL-Windows-x86_64.zip`
- Linux: `BiliDL-Linux-x86_64.tar.gz`
- macOS: `BiliDL-macOS-x86_64.tar.gz`

### 从源码安装

## 🔧 安装与编译

### 环境要求

- Rust 1.75 或更高版本
- FFmpeg（用于视频合并）
- 支持的操作系统：
  - Windows 10/11
  - macOS 10.15+
  - Linux（主流发行版）

### 安装 FFmpeg

Windows:
```powershell
winget install FFmpeg
```

macOS:
```bash
brew install ffmpeg
```

Linux:
```bash
# Ubuntu/Debian
sudo apt install ffmpeg

# CentOS/RHEL
sudo yum install ffmpeg
```

### 编译和安装

```bash
# 克隆仓库
git clone https://github.com/Rpeng666/bili_downloader
cd bili_downloader

# 编译
cargo build --release

## ⭐ 支持项目

如果这个项目对你有帮助，请给它一个 Star！
