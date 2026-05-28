//! 终端 WebSocket 处理器
//!
//! 桥接前端 xterm 和后端 PTY — 二进制直通（与 team PTY WS 一致）

use axum::extract::ws::{Message as WsMessage, WebSocket};
use futures_util::{SinkExt, StreamExt, TryStreamExt};
use nx_session::pty::PtyManager;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// 客户端控制消息（Text 帧）
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMessage {
    Resize { rows: u16, cols: u16 },
}

/// 服务端控制消息（Text 帧）
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ServerMessage {
    Ready,
    SessionEnded { exit_code: i32 },
    Error { message: String },
}

enum PtyCommand {
    Write(Vec<u8>),
    Resize { rows: u16, cols: u16 },
    Read { tx: mpsc::Sender<Vec<u8>> },
    Terminate,
}

/// 终端 WebSocket 处理器
#[derive(Clone)]
pub struct TerminalWsHandler;

impl TerminalWsHandler {
    pub fn new() -> Self {
        Self
    }

    /// 处理终端 WebSocket 连接；`cwd` 为 PTY 启动目录（须为有效目录）
    pub async fn handle(&self, socket: WebSocket, cwd: Option<String>) {
        let validated_cwd = cwd.and_then(|p| {
            let path = std::path::Path::new(&p);
            if path.is_dir() {
                Some(p)
            } else {
                tracing::warn!("终端 cwd 无效或非目录: {}", p);
                None
            }
        });

        let (mut sender, mut receiver) = socket.split();

        let (cmd_tx, mut cmd_rx) = mpsc::channel::<PtyCommand>(100);
        let (exit_tx, mut exit_rx) = tokio::sync::mpsc::channel::<i32>(1);

        let pty_cwd = validated_cwd.clone();
        std::thread::spawn(move || {
            let pty_manager = PtyManager::new();

            let shell = std::env::var("SHELL").unwrap_or_else(|_| {
                #[cfg(target_os = "macos")]
                {
                    "/bin/zsh".to_string()
                }
                #[cfg(not(target_os = "macos"))]
                {
                    "/bin/bash".to_string()
                }
            });
            let shell = if shell.is_empty() {
                "/bin/bash".to_string()
            } else {
                shell
            };

            let mut env = std::collections::HashMap::new();
            env.insert("TERM".to_string(), "xterm-256color".to_string());

            let pty_session_id = match pty_manager.create_session(
                Some(shell.as_str()),
                None,
                pty_cwd.as_deref(),
                Some(&env),
            ) {
                Ok(id) => id,
                Err(e) => {
                    tracing::error!("创建 PTY 会话失败: {}", e);
                    return;
                }
            };

            if let Some(ref dir) = pty_cwd {
                tracing::info!("终端 PTY 会话创建: {} cwd={}", pty_session_id, dir);
            } else {
                tracing::info!("终端 PTY 会话创建: {}", pty_session_id);
            }

            loop {
                match cmd_rx.blocking_recv() {
                    Some(PtyCommand::Write(data)) => {
                        if let Err(e) = pty_manager.write(&pty_session_id, &data) {
                            tracing::error!("写入 PTY 失败: {}", e);
                        }
                    }
                    Some(PtyCommand::Resize { rows, cols }) => {
                        if let Err(e) = pty_manager.resize(&pty_session_id, rows, cols) {
                            tracing::error!("调整 PTY 大小失败: {}", e);
                        }
                    }
                    Some(PtyCommand::Read { tx }) => {
                        match pty_manager.read(&pty_session_id, 50) {
                            Ok(outputs) => {
                                for output in outputs {
                                    if tx.blocking_send(output.data).is_err() {
                                        break;
                                    }
                                }
                            }
                            Err(e) => tracing::debug!("PTY 读取: {}", e),
                        }
                        if let Some(code) = pty_manager.poll_process_exit(&pty_session_id) {
                            let _ = exit_tx.blocking_send(code);
                            let _ = pty_manager.terminate(&pty_session_id);
                            break;
                        }
                    }
                    Some(PtyCommand::Terminate) | None => {
                        let _ = pty_manager.terminate(&pty_session_id);
                        break;
                    }
                }
            }

            tracing::info!("终端 PTY 会话关闭: {}", pty_session_id);
        });

        let ready = serde_json::to_string(&ServerMessage::Ready).unwrap_or_default();
        if sender.send(WsMessage::Text(ready)).await.is_err() {
            let _ = cmd_tx.send(PtyCommand::Terminate).await;
            return;
        }

        let (read_tx, mut read_rx) = mpsc::channel::<Vec<u8>>(256);

        loop {
            tokio::select! {
                data = read_rx.recv() => {
                    if let Some(data) = data {
                        if sender.send(WsMessage::Binary(data.into())).await.is_err() {
                            let _ = cmd_tx.send(PtyCommand::Terminate).await;
                            break;
                        }
                    }
                }
                msg = receiver.try_next() => {
                    match msg {
                        Ok(Some(WsMessage::Binary(data))) => {
                            let _ = cmd_tx.send(PtyCommand::Write(data.to_vec())).await;
                        }
                        Ok(Some(WsMessage::Text(text))) => {
                            if let Ok(ClientMessage::Resize { rows, cols }) =
                                serde_json::from_str(&text)
                            {
                                let _ = cmd_tx.send(PtyCommand::Resize { rows, cols }).await;
                            } else if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                                match json.get("type").and_then(|v| v.as_str()) {
                                    Some("input") => {
                                        if let Some(data) = json.get("data").and_then(|v| v.as_str()) {
                                            let _ = cmd_tx.send(PtyCommand::Write(data.as_bytes().to_vec())).await;
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
                        Ok(Some(WsMessage::Ping(data))) => {
                            if sender.send(WsMessage::Pong(data)).await.is_err() {
                                let _ = cmd_tx.send(PtyCommand::Terminate).await;
                                break;
                            }
                        }
                        Ok(Some(WsMessage::Close(_))) | Ok(None) => {
                            let _ = cmd_tx.send(PtyCommand::Terminate).await;
                            break;
                        }
                        Err(e) => {
                            let error_str = e.to_string();
                            if !error_str.contains("reset without closing")
                                && !error_str.contains("connection closed")
                            {
                                tracing::error!("WebSocket 错误: {}", e);
                            }
                            let _ = cmd_tx.send(PtyCommand::Terminate).await;
                            break;
                        }
                        _ => {}
                    }
                }
                exit_code = exit_rx.recv() => {
                    if let Some(code) = exit_code {
                        let msg = serde_json::to_string(&ServerMessage::SessionEnded { exit_code: code })
                            .unwrap_or_default();
                        let _ = sender.send(WsMessage::Text(msg)).await;
                    }
                    let _ = cmd_tx.send(PtyCommand::Terminate).await;
                    break;
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(16)) => {
                    let tx = read_tx.clone();
                    let _ = cmd_tx.send(PtyCommand::Read { tx }).await;
                }
            }
        }

        tracing::info!("终端 WebSocket 会话关闭");
    }
}

impl Default for TerminalWsHandler {
    fn default() -> Self {
        Self::new()
    }
}
