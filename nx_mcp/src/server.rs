//! MCP 服务器

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::tools::{Tool, ToolHandler};

/// MCP 服务器
pub struct McpServer {
    /// 服务器名称
    name: String,
    /// 服务器版本
    version: String,
    /// 已注册的工具
    tools: Arc<RwLock<HashMap<String, Tool>>>,
    /// 工具处理器
    tool_handlers: HashMap<String, Box<dyn ToolHandler>>,
}

impl McpServer {
    /// 创建新的 MCP 服务器
    pub fn new(name: String, version: String) -> Self {
        Self {
            name,
            version,
            tools: Arc::new(RwLock::new(HashMap::new())),
            tool_handlers: HashMap::new(),
        }
    }

    /// 注册工具
    pub async fn register_tool(&mut self, tool: Tool, handler: Box<dyn ToolHandler>) {
        let tool_name = tool.name.clone();
        let mut tools = self.tools.write().await;
        tools.insert(tool_name.clone(), tool);
        self.tool_handlers.insert(tool_name, handler);
    }

    /// 获取所有工具
    pub async fn list_tools(&self) -> Vec<Tool> {
        let tools = self.tools.read().await;
        tools.values().cloned().collect()
    }

    /// 调用工具
    pub async fn call_tool(
        &self,
        name: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, McpError> {
        let handler = self
            .tool_handlers
            .get(name)
            .ok_or_else(|| McpError::ToolNotFound(name.to_string()))?;

        handler.handle(input).await
    }

    /// 获取服务器信息
    pub fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: self.name.clone(),
            version: self.version.clone(),
        }
    }
}

/// 服务器信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

/// MCP 错误
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("工具未找到: {0}")]
    ToolNotFound(String),

    #[error("工具执行错误: {0}")]
    ToolExecution(String),

    #[error("协议错误: {0}")]
    Protocol(String),

    #[error("IO 错误: {0}")]
    Io(String),
}

impl From<std::io::Error> for McpError {
    fn from(e: std::io::Error) -> Self {
        McpError::Io(e.to_string())
    }
}

/// MCP 协议消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpMessage {
    /// 初始化请求
    Initialize {
        protocol_version: String,
        client_info: ClientInfo,
    },
    /// 初始化响应
    Initialized { server_info: ServerInfo },
    /// 列出工具请求
    ListTools {},
    /// 列出工具响应
    ListToolsResult { tools: Vec<Tool> },
    /// 调用工具请求
    CallTool {
        name: String,
        arguments: serde_json::Value,
    },
    /// 调用工具响应
    CallToolResult {
        content: Vec<ToolContent>,
        is_error: bool,
    },
    /// 资源列表请求
    ListResources {},
    /// 资源列表响应
    ListResourcesResult { resources: Vec<Resource> },
}

/// 客户端信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

/// 资源
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

/// 工具内容
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolContent {
    /// 文本内容
    Text { text: String },
    /// 图片内容
    Image { data: String, mime_type: String },
    /// 资源内容
    Resource { resource: Resource },
}
