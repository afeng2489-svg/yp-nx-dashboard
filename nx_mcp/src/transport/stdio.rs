//! 标准 I/O 传输
//!
//! 通过 stdin/stdout 与 MCP 客户端通信。

use serde_json::Value;
use tokio::io::{stdin, stdout, AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::server::McpServer;

/// 标准 I/O 传输处理器
pub struct StdioTransport;

impl StdioTransport {
    /// 创建新的 stdio 传输
    pub fn new() -> Self {
        Self
    }

    /// 运行传输处理循环
    pub async fn run(&self, server: McpServer) -> anyhow::Result<()> {
        let stdin = stdin();
        let mut stdout = stdout();
        let mut reader = BufReader::new(stdin);
        let mut json_buffer = String::new();

        loop {
            json_buffer.clear();

            // 读取一行 JSON
            match reader.read_line(&mut json_buffer).await {
                Ok(0) => break, // EOF
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("读取错误: {}", e);
                    break;
                }
            }

            // 解析消息
            let input: Value = match serde_json::from_str(&json_buffer) {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!("JSON 解析错误: {}", e);
                    continue;
                }
            };

            // 处理消息
            let response = self.handle_message(input, &server).await;

            // 发送响应
            if let Some(resp) = response {
                let json = serde_json::to_string(&resp).unwrap_or_else(|e| {
                    tracing::error!("序列化错误: {}", e);
                    String::new()
                });

                stdout.write_all(json.as_bytes()).await?;
                stdout.write_all(b"\n").await?;
            }
        }

        Ok(())
    }

    /// 处理消息
    async fn handle_message(&self, input: Value, server: &McpServer) -> Option<Value> {
        let method = input.get("method")?.as_str()?;
        let id = input.get("id").cloned();

        match method {
            "initialize" => {
                tracing::info!("收到初始化请求");
                Some(serde_json::json!({
                    "id": id,
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "serverInfo": {
                            "name": server.server_info().name,
                            "version": server.server_info().version
                        },
                        "capabilities": {
                            "tools": {}
                        }
                    }
                }))
            }
            "notifications/initialized" => {
                tracing::info!("客户端已初始化");
                None
            }
            "tools/list" => {
                let tools = server.list_tools().await;
                Some(serde_json::json!({
                    "id": id,
                    "result": {
                        "tools": tools
                    }
                }))
            }
            "tools/call" => {
                let name = input.get("params")?.get("name")?.as_str()?.to_string();
                let arguments = input.get("params")?.get("arguments")?.clone();

                match server.call_tool(&name, arguments).await {
                    Ok(result) => Some(serde_json::json!({
                        "id": id,
                        "result": {
                            "content": [
                                {
                                    "type": "text",
                                    "text": result.to_string()
                                }
                            ]
                        }
                    })),
                    Err(e) => Some(serde_json::json!({
                        "id": id,
                        "error": {
                            "code": -32603,
                            "message": e.to_string()
                        }
                    })),
                }
            }
            _ => {
                tracing::warn!("未知方法: {}", method);
                Some(serde_json::json!({
                    "id": id,
                    "error": {
                        "code": -32601,
                        "message": format!("方法不存在: {}", method)
                    }
                }))
            }
        }
    }
}

impl Default for StdioTransport {
    fn default() -> Self {
        Self::new()
    }
}
