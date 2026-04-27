//! 终端 WebSocket 处理器
//!
//! 桥接前端 xterm 和后端 PTY

use axum::extract::ws::{Message as WsMessage, WebSocket};
use futures_util::{SinkExt, StreamExt, TryStreamExt};
use nx_session::pty::PtyManager;
use tokio::sync::{mpsc, oneshot};

/// PTY 命令
enum PtyCommand {
    Write(String),
    Resize { rows: u16, cols: u16 },
    Read { tx: mpsc::Sender<String> },
    Terminate,
}

/// 终端 WebSocket 处理器
#[derive(Clone)]
pub struct TerminalWsHandler;

impl TerminalWsHandler {
    pub fn new() -> Self {
        Self
    }

    /// 处理终端 WebSocket 连接
    pub async fn handle(&self, socket: WebSocket) {
        let (mut sender, mut receiver) = socket.split();

        // 创建 channel 用于与后台线程通信
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<PtyCommand>(100);

        // 创建后台线程处理 PTY
        let handle = std::thread::spawn(move || {
            let pty_manager = PtyManager::new();

            // 创建 PTY 会话
            let pty_session_id = match pty_manager.create_session(Some("bash"), None, None, None) {
                Ok(id) => id,
                Err(e) => {
                    tracing::error!("创建 PTY 会话失败: {}", e);
                    return;
                }
            };

            tracing::info!("终端 PTY 会话创建: {}", pty_session_id);

            // 事件循环
            loop {
                // 阻塞等待命令
                match cmd_rx.blocking_recv() {
                    Some(PtyCommand::Write(data)) => {
                        if let Err(e) = pty_manager.write(&pty_session_id, data.as_bytes()) {
                            tracing::error!("写入 PTY 失败: {}", e);
                        }
                    }
                    Some(PtyCommand::Resize { rows, cols }) => {
                        if let Err(e) = pty_manager.resize(&pty_session_id, rows, cols) {
                            tracing::error!("调整 PTY 大小失败: {}", e);
                        }
                    }
                    Some(PtyCommand::Read { tx }) => match pty_manager.read(&pty_session_id, 50) {
                        Ok(outputs) => {
                            for output in outputs {
                                let data = String::from_utf8_lossy(&output.data).to_string();
                                if tx.blocking_send(data).is_err() {
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("读取 PTY 输出失败: {}", e);
                        }
                    },
                    Some(PtyCommand::Terminate) => {
                        let _ = pty_manager.terminate(&pty_session_id);
                        break;
                    }
                    None => {
                        let _ = pty_manager.terminate(&pty_session_id);
                        break;
                    }
                }
            }

            tracing::info!("终端 PTY 会话关闭: {}", pty_session_id);
        });

        // 发送欢迎消息
        let welcome = "\x1b[36m[NexusFlow]\x1b[0m 终端已连接\r\n\r\n";
        if sender
            .send(WsMessage::Text(welcome.to_string()))
            .await
            .is_err()
        {
            let _ = cmd_tx.send(PtyCommand::Terminate).await;
            return;
        }

        // 创建读取结果的 channel
        let (read_tx, mut read_rx) = mpsc::channel::<String>(100);

        // 事件循环
        loop {
            tokio::select! {
                // 发送 PTY 输出到 WebSocket
                data = read_rx.recv() => {
                    if let Some(data) = data {
                        let msg = serde_json::json!({
                            "type": "output",
                            "data": data
                        }).to_string();
                        if sender.send(WsMessage::Text(msg)).await.is_err() {
                            let _ = cmd_tx.send(PtyCommand::Terminate).await;
                            break;
                        }
                    }
                }
                // 处理 WebSocket 输入
                msg = receiver.try_next() => {
                    match msg {
                        Ok(Some(WsMessage::Text(text))) => {
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                                match json.get("type").and_then(|v| v.as_str()) {
                                    Some("input") => {
                                        if let Some(data) = json.get("data").and_then(|v| v.as_str()) {
                                            let _ = cmd_tx.send(PtyCommand::Write(data.to_string())).await;
                                        }
                                    }
                                    Some("resize") => {
                                        let rows = json.get("rows").and_then(|v| v.as_u64()).unwrap_or(24) as u16;
                                        let cols = json.get("cols").and_then(|v| v.as_u64()).unwrap_or(80) as u16;
                                        let _ = cmd_tx.send(PtyCommand::Resize { rows, cols }).await;
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Ok(Some(WsMessage::Binary(data))) => {
                            let _ = cmd_tx.send(PtyCommand::Write(String::from_utf8_lossy(&data).to_string())).await;
                        }
                        Ok(Some(WsMessage::Ping(data))) => {
                            #[allow(clippy::collapsible_match)]
                            if sender.send(WsMessage::Pong(data)).await.is_err() {
                                let _ = cmd_tx.send(PtyCommand::Terminate).await;
                                break;
                            }
                        }
                        Ok(Some(WsMessage::Close(_))) | Ok(None) => {
                            let _ = cmd_tx.send(PtyCommand::Terminate).await;
                            break;
                        }
                        Ok(Some(WsMessage::Pong(_))) => {}
                        Err(e) => {
                            // "Connection reset without closing handshake" 是正常的断连，不算错误
                            let error_str = e.to_string();
                            if error_str.contains("reset without closing") || error_str.contains("connection closed") {
                                tracing::warn!("WebSocket 客户端断连: {}", error_str);
                            } else {
                                tracing::error!("WebSocket 错误: {}", e);
                            }
                            let _ = cmd_tx.send(PtyCommand::Terminate).await;
                            break;
                        }
                        _ => {}
                    }
                }
                // 定期读取 PTY 输出
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(50)) => {
                    let tx = read_tx.clone();
                    let _ = cmd_tx.send(PtyCommand::Read { tx }).await;
                }
            }
        }

        // 等待后台线程结束
        let _ = handle.join();

        tracing::info!("终端 WebSocket 会话关闭");
    }
}

impl Default for TerminalWsHandler {
    fn default() -> Self {
        Self::new()
    }
}
