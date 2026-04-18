//! 通用命令执行 WebSocket 处理器
//!
//! 在指定工作目录中执行任意命令，实时流式传输 stdout/stderr

use axum::extract::ws::{Message as WsMessage, WebSocket};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tokio::sync::mpsc;

/// 客户端 → 服务端消息
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RunCommandClientMsg {
    Execute {
        command: String,
        working_directory: String,
    },
    Cancel,
}

/// 服务端 → 客户端消息
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RunCommandServerMsg {
    Started { pid: u32 },
    Stdout { data: String },
    Stderr { data: String },
    Exit { code: i32 },
    Error { message: String },
}

impl RunCommandServerMsg {
    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

#[derive(Clone)]
pub struct RunCommandWsHandler;

impl RunCommandWsHandler {
    pub fn new() -> Self {
        Self
    }

    pub async fn handle(&self, socket: WebSocket) {
        let (mut sender, mut receiver) = socket.split();
        let (output_tx, mut output_rx) = mpsc::channel::<String>(256);
        let (cancel_tx, _) = mpsc::channel::<()>(1);

        loop {
            tokio::select! {
                msg = receiver.next() => {
                    match msg {
                        Some(Ok(WsMessage::Text(text))) => {
                            if let Ok(client_msg) = serde_json::from_str::<RunCommandClientMsg>(&text) {
                                match client_msg {
                                    RunCommandClientMsg::Execute { command, working_directory } => {
                                        let output_tx = output_tx.clone();
                                        let cancel_tx_clone = cancel_tx.clone();
                                        tokio::spawn(async move {
                                            Self::run_command(command, working_directory, output_tx, cancel_tx_clone).await;
                                        });
                                    }
                                    RunCommandClientMsg::Cancel => {
                                        break;
                                    }
                                }
                            }
                        }
                        Some(Ok(WsMessage::Close(_))) | None => break,
                        Some(Ok(WsMessage::Ping(data))) => {
                            if sender.send(WsMessage::Pong(data)).await.is_err() {
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                json_msg = output_rx.recv() => {
                    if let Some(json_msg) = json_msg {
                        if sender.send(WsMessage::Text(json_msg)).await.is_err() {
                            break;
                        }
                    }
                }
            }
        }

        tracing::info!("[RunCommand] WebSocket session closed");
    }

    async fn run_command(
        command: String,
        working_directory: String,
        output_tx: mpsc::Sender<String>,
        cancel_tx: mpsc::Sender<()>,
    ) {
        let dir = std::path::Path::new(&working_directory);
        if !dir.is_absolute() || !dir.exists() || !dir.is_dir() {
            output_tx
                .send(RunCommandServerMsg::Error { message: format!("无效的工作目录: {}", working_directory) }.to_json())
                .await
                .ok();
            return;
        }

        tracing::info!("[RunCommand] 执行: {} in {}", command, working_directory);

        let mut cmd = Command::new("sh");
        cmd.args(["-c", &command])
            .current_dir(dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .stdin(std::process::Stdio::null());

        match cmd.spawn() {
            Ok(mut child) => {
                let pid = child.id().unwrap_or(0);
                output_tx.send(RunCommandServerMsg::Started { pid }.to_json()).await.ok();

                let stdout = child.stdout.take();
                let stderr = child.stderr.take();

                let out_tx1 = output_tx.clone();
                let stdout_handle = if let Some(stdout) = stdout {
                    tokio::spawn(async move {
                        use tokio::io::AsyncBufReadExt;
                        let mut lines = tokio::io::BufReader::new(stdout).lines();
                        while let Ok(Some(line)) = lines.next_line().await {
                            if out_tx1.send(RunCommandServerMsg::Stdout { data: line }.to_json()).await.is_err() {
                                break;
                            }
                        }
                    })
                } else {
                    tokio::spawn(async {})
                };

                let out_tx2 = output_tx.clone();
                let stderr_handle = if let Some(stderr) = stderr {
                    tokio::spawn(async move {
                        use tokio::io::AsyncBufReadExt;
                        let mut lines = tokio::io::BufReader::new(stderr).lines();
                        while let Ok(Some(line)) = lines.next_line().await {
                            if out_tx2.send(RunCommandServerMsg::Stderr { data: line }.to_json()).await.is_err() {
                                break;
                            }
                        }
                    })
                } else {
                    tokio::spawn(async {})
                };

                tokio::select! {
                    status = child.wait() => {
                        // Wait for streams to flush before sending exit
                        let _ = stdout_handle.await;
                        let _ = stderr_handle.await;
                        let code = status.map(|s| s.code().unwrap_or(-1)).unwrap_or(-1);
                        output_tx.send(RunCommandServerMsg::Exit { code }.to_json()).await.ok();
                    }
                    _ = cancel_tx.closed() => {
                        child.kill().await.ok();
                        output_tx.send(RunCommandServerMsg::Exit { code: -1 }.to_json()).await.ok();
                    }
                }
            }
            Err(e) => {
                output_tx
                    .send(RunCommandServerMsg::Error { message: format!("启动命令失败: {}", e) }.to_json())
                    .await
                    .ok();
            }
        }
    }
}

impl Default for RunCommandWsHandler {
    fn default() -> Self {
        Self::new()
    }
}
