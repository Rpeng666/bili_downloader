# BiliDownloader

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust Version](https://img.shields.io/badge/rust-1.75%2B-orange.svg)
![GitHub release (latest by date)](https://img.shields.io/github/v/release/Rpeng666/bili_downloader)
![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/Rpeng666/bili_downloader/release.yml)

🚀 一个使用Rust编写的bilibili命令行下载器。极致小巧（<10MB),  开箱即食。
> 来都来了，不给个star鼓励一下嘛？Thanks♪(･ω･)ﾉ

![img](./docs/333.png)

## ✨ 特性

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
    - [x] 番剧（单集/整季）
    - [x] 课程视频
    - [x] 弹幕、字幕下载
  - [x] 支持 DASH 和 MP4 格式
  - [x] 友好的清晰度选择（360p到8k）
  - [x] 集数范围选择（如：1-5,7,9-12）
- 🛠 **人性化设计**
  - [x] 简洁的命令行界面
  - [x] 详细的日志输出和错误分析
  - [x] 灵活的配置选项
  - [x] 友好的错误提示和解决建议
- 🤖 **AI助手集成**
  - [x] MCP (Model Context Protocol) 支持(施工中)


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

5. 启动MCP服务器模式（AI助手集成）（施工中）：

```bash
# 启动MCP服务器模式
bilidl --mcp

# 在Claude Desktop或其他MCP客户端中使用
# AI助手可以直接通过自然语言控制下载器
```

## 🤖 MCP (Model Context Protocol) 集成 (施工中)

BiliDownloader 支持 MCP，可以与 AI 助手（如 Claude Desktop）无缝集成。

### Claude Desktop 集成

1. **编译带MCP支持的版本**：

```bash
cargo build --release --features mcp
```

2. **配置 Claude Desktop**：

在 Claude Desktop 的配置文件中添加：

```json
{
  "mcpServers": {
    "bili-downloader": {
      "command": "/path/to/bilidl",
      "args": ["--mcp"],
      "env": {}
    }
  }
}
```

3. **使用 AI 助手控制下载器**：

```
用户: "帮我下载这个B站视频，要4K画质"
AI: 我来帮你下载这个视频...
[使用 bili_download 工具执行下载]

用户: "查看当前下载进度"
AI: [使用 bili_list_downloads 工具查看状态]

用户: "下载这个番剧的前5集"
AI: [使用 bili_parse_info 解析信息，然后下载指定集数]
```

### 可用的MCP工具

- `bili_download`: 下载视频/番剧/课程
- `bili_parse_info`: 解析视频信息
- `bili_list_downloads`: 查看下载状态
- `bili_cancel_download`: 取消下载任务
- `bili_login_status`: 检查登录状态
- `bili_qr_login`: 二维码登录

### 可用的MCP资源

- `downloads://active`: 活跃下载任务
- `downloads://history`: 下载历史
- `config://settings`: 配置设置

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

# 标准编译（仅CLI功能）
cargo build --release

# 编译带MCP支持的版本（用于AI助手集成）
cargo build --release --features mcp

# 安装到系统（可选）
cargo install --path .
```

## ⭐ 支持项目

如果这个项目对你有帮助，请给它一个 Star！
