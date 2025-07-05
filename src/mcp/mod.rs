#[cfg(feature = "mcp")]
pub mod server;

#[cfg(feature = "mcp")]
pub mod tools;

#[cfg(feature = "mcp")]
pub mod resources;

#[cfg(feature = "mcp")]
pub mod handlers;

#[cfg(feature = "mcp")]
pub use server::BiliMcpServer as McpServer;
