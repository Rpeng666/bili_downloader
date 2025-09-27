#[cfg(feature = "mcp")]
pub mod server;

#[cfg(feature = "mcp")]
pub use server::BiliMcpServer as McpServer;
