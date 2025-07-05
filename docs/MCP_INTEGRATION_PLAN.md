# BiliDownloader MCP 集成计划

## 概述

将BiliDownloader集成MCP（Model Context Protocol）支持，使AI助手能够直接通过标准协议操作下载器。

## MCP Server 功能设计

### 核心工具（Tools）

#### 1. `bili_download`
- **描述**: 下载B站视频/番剧/课程
- **参数**:
  - `url` (required): 视频链接
  - `quality` (optional): 清晰度 (360p/480p/720p/1080p/4k等)
  - `output_dir` (optional): 输出目录
  - `parts` (optional): 集数范围 (如 "1-5,7,9-12")
  - `login_required` (optional): 是否需要登录

#### 2. `bili_parse_info`
- **描述**: 解析视频信息但不下载
- **参数**:
  - `url` (required): 视频链接
- **返回**: 视频标题、时长、清晰度选项、集数信息等

#### 3. `bili_list_downloads`
- **描述**: 列出当前下载任务状态
- **返回**: 正在进行的下载任务列表和进度

#### 4. `bili_cancel_download`
- **描述**: 取消指定的下载任务
- **参数**:
  - `task_id` (required): 任务ID

#### 5. `bili_login_status`
- **描述**: 检查登录状态
- **返回**: 当前登录状态和用户信息

#### 6. `bili_qr_login`
- **描述**: 生成二维码登录
- **返回**: 登录二维码和状态

### 资源（Resources）

#### 1. `downloads://active`
- **描述**: 当前活跃下载任务
- **内容**: 实时下载进度和状态

#### 2. `downloads://history`
- **描述**: 下载历史记录
- **内容**: 已完成的下载记录

#### 3. `config://settings`
- **描述**: 下载器配置
- **内容**: 当前配置设置

## 技术实现

### 依赖库
```toml
[dependencies]
# 现有依赖...
mcp-sdk = "0.1"  # MCP Rust SDK
tokio-tungstenite = "0.20"  # WebSocket支持
serde_json = "1.0"
uuid = "1.0"
```

### 目录结构
```
src/
├── mcp/
│   ├── mod.rs          # MCP模块入口
│   ├── server.rs       # MCP服务器实现
│   ├── tools.rs        # 工具实现
│   ├── resources.rs    # 资源实现
│   └── handlers.rs     # 请求处理器
├── main.rs             # 主程序（CLI模式）
└── mcp_main.rs         # MCP服务器模式入口
```

## 使用场景

### 1. AI助手集成
```
用户: "帮我下载这个B站视频 https://www.bilibili.com/video/BVxxxxx，要1080P画质"
AI: 使用bili_download工具下载视频...
```

### 2. 批量操作
```
用户: "下载这个番剧的前5集"
AI: 
1. 使用bili_parse_info解析番剧信息
2. 使用bili_download下载指定集数
```

### 3. 状态监控
```
用户: "当前有什么下载任务在进行？"
AI: 使用bili_list_downloads查看当前状态...
```

## 实现阶段

### Phase 1: 基础MCP Server
- [ ] 实现基本的MCP协议支持
- [ ] 添加核心下载工具
- [ ] 支持JSON-RPC over stdio

### Phase 2: 高级功能
- [ ] 添加WebSocket传输支持
- [ ] 实现资源订阅功能
- [ ] 添加进度实时推送

### Phase 3: 生态集成
- [ ] Claude Desktop集成配置
- [ ] VS Code扩展支持
- [ ] 其他MCP客户端兼容性

## 配置示例

### Claude Desktop配置
```json
{
  "mcpServers": {
    "bili-downloader": {
      "command": "bilidl",
      "args": ["--mcp"],
      "env": {}
    }
  }
}
```

### VS Code配置
```json
{
  "mcp.servers": [
    {
      "name": "bili-downloader",
      "command": "bilidl --mcp"
    }
  ]
}
```
