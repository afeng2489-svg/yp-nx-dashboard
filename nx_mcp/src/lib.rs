//! NexusFlow MCP 服务器
//!
//! Model Context Protocol (MCP) 服务器实现。
//! 提供工具、资源和提示模板供 MCP 客户端使用。

pub mod server;
pub mod tools;
pub mod transport;

pub use server::{McpError, McpServer};
pub use tools::{Tool, ToolHandler, ToolInput, ToolOutput};
