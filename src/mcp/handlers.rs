use serde_json::Value;
use crate::mcp::server::BiliMcpServer;

/// MCP请求处理器
pub struct McpRequestHandler {
    server: BiliMcpServer,
}

impl McpRequestHandler {
    pub fn new() -> Self {
        Self {
            server: BiliMcpServer::new(),
        }
    }

    /// 处理工具调用请求
    pub async fn handle_tool_call(&mut self, tool_name: &str, args: Value) -> anyhow::Result<Value> {
        match tool_name {
            "bili_download" => self.server.tool_bili_download(args).await,
            "bili_parse_info" => self.server.tool_bili_parse_info(args).await,
            "bili_list_downloads" => self.server.tool_bili_list_downloads(args).await,
            "bili_cancel_download" => self.server.tool_bili_cancel_download(args).await,
            "bili_login_status" => self.server.tool_bili_login_status(args).await,
            "bili_qr_login" => self.server.tool_bili_qr_login(args).await,
            _ => Err(anyhow::anyhow!("未知的工具: {}", tool_name))
        }
    }

    /// 处理资源读取请求
    pub async fn handle_resource_read(&self, uri: &str) -> anyhow::Result<Value> {
        crate::mcp::resources::get_resource_content(uri).await
    }

    /// 获取可用工具列表
    pub fn get_available_tools(&self) -> Vec<Value> {
        crate::mcp::tools::get_tool_definitions()
    }

    /// 获取可用资源列表
    pub fn get_available_resources(&self) -> Vec<Value> {
        crate::mcp::resources::get_resource_definitions()
    }
}
