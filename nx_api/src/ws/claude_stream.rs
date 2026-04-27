//! Claude CLI 流式 WebSocket 处理器
//!
//! 将 Claude CLI 的 stdout/stderr 实时流式传输到前端

use axum::extract::ws::{Message as WsMessage, WebSocket};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tokio::sync::mpsc;

/// Claude CLI WebSocket 消息（客户端 -> 服务端）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClaudeStreamClientMsg {
    /// 执行 Claude CLI
    Execute {
        prompt: String,
        working_directory: Option<String>,
    },
    /// 终止执行
    Cancel,
}

/// Claude CLI WebSocket 消息（服务端 -> 客户端）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClaudeStreamServerMsg {
    /// 开始执行
    Started { execution_id: String },
    /// 输出行
    Output { execution_id: String, line: String },
    /// stderr 输出
    Error { execution_id: String, line: String },
    /// 执行完成
    Completed {
        execution_id: String,
        exit_code: i32,
    },
    /// 执行失败
    Failed { execution_id: String, error: String },
}

impl ClaudeStreamServerMsg {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// Claude CLI 流式 WebSocket 处理器
#[derive(Clone)]
pub struct ClaudeStreamWsHandler;

impl ClaudeStreamWsHandler {
    pub fn new() -> Self {
        Self
    }

    /// 处理 Claude CLI 流式 WebSocket 连接
    pub async fn handle(&self, socket: WebSocket) {
        let (mut sender, mut receiver) = socket.split();

        // 生成执行 ID
        let execution_id = uuid::Uuid::new_v4().to_string();

        // 发送开始消息
        let started_msg = ClaudeStreamServerMsg::Started {
            execution_id: execution_id.clone(),
        };
        if sender
            .send(WsMessage::Text(started_msg.to_json().unwrap()))
            .await
            .is_err()
        {
            return;
        }

        // 用于取消的 channel
        let (cancel_tx, mut cancel_rx) = mpsc::channel::<()>(1);

        // 用于发送输出的 channel (发送 JSON 字符串避免序列化开销)
        let (output_tx, mut output_rx) = mpsc::channel::<String>(100);

        // 用于跟踪是否有正在执行的进程
        let (executing_tx, mut executing_rx) = mpsc::channel::<()>(1);

        // 等待客户端消息
        loop {
            tokio::select! {
                // 处理客户端消息
                msg = receiver.next() => {
                    match msg {
                        Some(Ok(WsMessage::Text(text))) => {
                            if let Ok(json) = serde_json::from_str::<ClaudeStreamClientMsg>(&text) {
                                match json {
                                    ClaudeStreamClientMsg::Execute { prompt, working_directory } => {
                                        // 重置 cancel channel
                                        let (new_cancel_tx, mut new_cancel_rx) = mpsc::channel::<()>(1);
                                        cancel_tx.send(()).await.ok();
                                        let cancel_tx = new_cancel_tx;

                                        let exec_id = execution_id.clone();
                                        let output_tx = output_tx.clone();
                                        let executing_tx = executing_tx.clone();

                                        // Spawn 执行任务
                                        tokio::spawn(async move {
                                            Self::execute_claude(prompt, working_directory, exec_id, new_cancel_rx, output_tx, executing_tx).await;
                                        });
                                    }
                                    ClaudeStreamClientMsg::Cancel => {
                                        cancel_tx.send(()).await.ok();
                                    }
                                }
                            }
                        }
                        Some(Ok(WsMessage::Close(_))) | None => {
                            cancel_tx.send(()).await.ok();
                            break;
                        }
                        Some(Ok(WsMessage::Ping(data))) => {
                            if sender.send(WsMessage::Pong(data)).await.is_err() {
                                break;
                            }
                        }
                        Some(Err(e)) => {
                            tracing::error!("WebSocket 错误: {}", e);
                            break;
                        }
                        _ => {}
                    }
                }
                // 处理 Claude CLI 输出
                json_msg = output_rx.recv() => {
                    if let Some(json_msg) = json_msg {
                        if sender.send(WsMessage::Text(json_msg)).await.is_err() {
                            break;
                        }
                    }
                }
                // 取消信号
                _ = cancel_rx.recv() => {
                    // 取消当前执行
                    break;
                }
            }
        }

        // 清理
        cancel_tx.send(()).await.ok();

        tracing::info!("Claude Stream WebSocket 会话关闭: {}", execution_id);
    }

    /// 执行 Claude CLI 并流式输出
    async fn execute_claude(
        prompt: String,
        working_directory: Option<String>,
        execution_id: String,
        mut cancel_rx: mpsc::Receiver<()>,
        output_tx: mpsc::Sender<String>,
        _executing_tx: mpsc::Sender<()>,
    ) {
        let mut cmd = Command::new("claude");
        cmd.args(["-p", "--no-session-persistence", &prompt])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        if let Some(dir) = &working_directory {
            let path = std::path::Path::new(dir);
            if !path.is_absolute() || !path.exists() || !path.is_dir() {
                let msg = ClaudeStreamServerMsg::Failed {
                    execution_id,
                    error: format!("Invalid working directory: {}", dir),
                };
                if let Ok(json) = msg.to_json() {
                    output_tx.send(json).await.ok();
                }
                return;
            }
            cmd.current_dir(dir);
        }

        tracing::info!(
            "[Claude Stream] 开始执行: prompt={}, dir={:?}",
            prompt.chars().take(50).collect::<String>(),
            working_directory
        );

        match cmd.spawn() {
            Ok(mut child) => {
                let stdout = child.stdout.take();
                let stderr = child.stderr.take();

                // Spawn stdout 读取任务
                let exec_id_stdout = execution_id.clone();
                let output_tx_stdout = output_tx.clone();
                let stdout_handle = if let Some(stdout) = stdout {
                    tokio::spawn(async move {
                        use tokio::io::AsyncBufReadExt;
                        let reader = tokio::io::BufReader::new(stdout);
                        let mut lines = reader.lines();
                        while let Ok(Some(line)) = lines.next_line().await {
                            let msg = ClaudeStreamServerMsg::Output {
                                execution_id: exec_id_stdout.clone(),
                                line,
                            };
                            if let Ok(json) = msg.to_json() {
                                output_tx_stdout.send(json).await.ok();
                            }
                        }
                    })
                } else {
                    tokio::spawn(async move {})
                };

                // Spawn stderr 读取任务
                let exec_id_stderr = execution_id.clone();
                let output_tx_stderr = output_tx.clone();
                let stderr_handle = if let Some(stderr) = stderr {
                    tokio::spawn(async move {
                        use tokio::io::AsyncBufReadExt;
                        let reader = tokio::io::BufReader::new(stderr);
                        let mut lines = reader.lines();
                        while let Ok(Some(line)) = lines.next_line().await {
                            let msg = ClaudeStreamServerMsg::Error {
                                execution_id: exec_id_stderr.clone(),
                                line,
                            };
                            if let Ok(json) = msg.to_json() {
                                output_tx_stderr.send(json).await.ok();
                            }
                        }
                    })
                } else {
                    tokio::spawn(async move {})
                };

                // 等待进程结束或取消
                loop {
                    tokio::select! {
                        status = child.wait() => {
                            match status {
                                Ok(exit_status) => {
                                    let exit_code = exit_status.code().unwrap_or(-1);
                                    let msg = ClaudeStreamServerMsg::Completed {
                                        execution_id: execution_id.clone(),
                                        exit_code,
                                    };
                                    if let Ok(json) = msg.to_json() {
                                        output_tx.send(json).await.ok();
                                    }
                                }
                                Err(e) => {
                                    let msg = ClaudeStreamServerMsg::Failed {
                                        execution_id: execution_id.clone(),
                                        error: format!("进程错误: {}", e),
                                    };
                                    if let Ok(json) = msg.to_json() {
                                        output_tx.send(json).await.ok();
                                    }
                                }
                            }
                            break;
                        }
                        _ = cancel_rx.recv() => {
                            // 用户取消，杀掉进程
                            child.kill().await.ok();
                            let msg = ClaudeStreamServerMsg::Failed {
                                execution_id: execution_id.clone(),
                                error: "用户取消".to_string(),
                            };
                            if let Ok(json) = msg.to_json() {
                                output_tx.send(json).await.ok();
                            }
                            break;
                        }
                    }
                }

                // 等待 stdout/stderr 任务完成
                let _ = stdout_handle.await;
                let _ = stderr_handle.await;
            }
            Err(e) => {
                let msg = ClaudeStreamServerMsg::Failed {
                    execution_id,
                    error: format!("启动 Claude CLI 失败: {}", e),
                };
                if let Ok(json) = msg.to_json() {
                    output_tx.send(json).await.ok();
                }
            }
        }
    }
}

impl Default for ClaudeStreamWsHandler {
    fn default() -> Self {
        Self::new()
    }
}
