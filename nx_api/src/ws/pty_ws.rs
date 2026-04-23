//! PTY 终端 WebSocket 处理器
//!
//! 将 PTY 会话的原始终端输出通过 WebSocket 推送给前端 xterm.js，
//! 并将前端键盘输入转发回 PTY stdin。
//!
//! 消息格式：
//! - 服务端 -> 客户端：Binary 帧（原始终端字节）
//! - 客户端 -> 服务端：Binary 帧（原始键盘字节）或 Text 帧（控制消息 JSON）

use axum::extract::ws::{Message as AxumMessage, WebSocket};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};

use crate::services::claude_terminal::ClaudeTerminalManager;

/// 客户端发来的控制消息（Text 帧）
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum PtyClientMessage {
    /// 发送一个任务文本（自动追加 \n）
    Task { text: String },
    /// 调整终端尺寸
    Resize { rows: u16, cols: u16 },
    /// 关闭会话
    Close,
}

/// 服务端发给客户端的控制消息（Text 帧）
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum PtyServerMessage {
    /// 会话已就绪
    Ready { session_id: String },
    /// 会话已结束
    Closed,
    /// 错误
    Error { message: String },
}

/// PTY WebSocket 处理函数
///
/// 前端连接 `/ws/teams/:team_id/pty/:session_id` 后：
/// 1. 订阅该会话的终端输出，以 Binary 帧推送给前端
/// 2. 将前端的 Binary 帧（键盘输入）写入 PTY stdin
/// 3. 将前端的 Text 帧（控制消息）解析处理（派发任务、resize 等）
pub async fn handle_pty_ws(
    socket: WebSocket,
    session_id: String,
    manager: ClaudeTerminalManager,
) {
    let (mut sender, mut receiver) = socket.split();

    // 获取会话
    let session = match manager.get_session(&session_id) {
        Some(s) => s,
        None => {
            let msg = serde_json::to_string(&PtyServerMessage::Error {
                message: format!("Session {} not found", session_id),
            }).unwrap_or_default();
            let _ = sender.send(AxumMessage::Text(msg)).await;
            return;
        }
    };

    tracing::info!("[PtyWS] 客户端连接，session: {}", session_id);

    // 发送就绪消息
    let ready_msg = serde_json::to_string(&PtyServerMessage::Ready {
        session_id: session_id.clone(),
    }).unwrap_or_default();
    if sender.send(AxumMessage::Text(ready_msg)).await.is_err() {
        return;
    }

    // 订阅 PTY 输出
    let mut output_rx = session.subscribe_output();

    loop {
        tokio::select! {
            // PTY 输出 -> 前端（Binary 帧）
            output = output_rx.recv() => {
                match output {
                    Ok(data) => {
                        if sender.send(AxumMessage::Binary(data.into())).await.is_err() {
                            tracing::debug!("[PtyWS] 发送失败，客户端断开");
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("[PtyWS] 落后 {} 帧", n);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::info!("[PtyWS] PTY 输出通道关闭，会话结束");
                        let _ = sender.send(AxumMessage::Text(
                            serde_json::to_string(&PtyServerMessage::Closed).unwrap_or_default()
                        )).await;
                        break;
                    }
                }
            }

            // 前端输入 -> PTY
            msg = receiver.next() => {
                match msg {
                    Some(Ok(AxumMessage::Binary(data))) => {
                        // 原始键盘输入（xterm.js onData 事件）
                        session.send_input(data.to_vec());
                    }
                    Some(Ok(AxumMessage::Text(text))) => {
                        // 控制消息
                        match serde_json::from_str::<PtyClientMessage>(&text) {
                            Ok(PtyClientMessage::Task { text }) => {
                                tracing::info!("[PtyWS] 派发任务: {:.60}...", text);
                                session.dispatch_task(&text);
                            }
                            Ok(PtyClientMessage::Resize { rows, cols }) => {
                                manager.resize_session(&session_id, rows, cols);
                            }
                            Ok(PtyClientMessage::Close) => {
                                manager.close_session(&session_id);
                                break;
                            }
                            Err(e) => {
                                tracing::debug!("[PtyWS] 无法解析控制消息: {}", e);
                            }
                        }
                    }
                    Some(Ok(AxumMessage::Ping(data))) => {
                        let _ = sender.send(AxumMessage::Pong(data)).await;
                    }
                    Some(Ok(AxumMessage::Close(_))) | None => {
                        tracing::info!("[PtyWS] 客户端断开: {}", session_id);
                        break;
                    }
                    Some(Err(e)) => {
                        tracing::warn!("[PtyWS] WS 错误: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        }
    }
}
