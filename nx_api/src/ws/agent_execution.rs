//! Agent 执行 WebSocket 处理器
//!
//! 提供 Agent 任务异步执行的实时状态推送，支持：
//! - 思考中心跳（每 5 秒）
//! - 部分输出流式推送
//! - 完成/失败/取消事件
//! - 客户端发起取消

use axum::extract::ws::{Message as AxumMessage, WebSocket};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

/// Agent 执行事件（通过 broadcast channel 分发）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentExecutionEvent {
    /// 任务已开始
    Started {
        execution_id: String,
        agent_role: String,
        task_summary: String,
    },
    /// Agent 思考中（心跳）
    Thinking {
        execution_id: String,
        elapsed_secs: u64,
    },
    /// 部分输出
    Output {
        execution_id: String,
        partial_output: String,
    },
    /// 任务完成
    Completed {
        execution_id: String,
        result: String,
        duration_ms: u64,
    },
    /// 任务失败
    Failed {
        execution_id: String,
        error: String,
    },
    /// 任务已取消
    Cancelled {
        execution_id: String,
    },
}

impl AgentExecutionEvent {
    /// 获取事件关联的 execution_id
    pub fn execution_id(&self) -> &str {
        match self {
            Self::Started { execution_id, .. }
            | Self::Thinking { execution_id, .. }
            | Self::Output { execution_id, .. }
            | Self::Completed { execution_id, .. }
            | Self::Failed { execution_id, .. }
            | Self::Cancelled { execution_id, .. } => execution_id,
        }
    }
}

/// 客户端 -> 服务端消息
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMessage {
    /// 取消执行
    Cancel,
}

/// Agent 执行状态管理器
///
/// 全局单例，管理所有正在进行的 agent 执行的事件广播和取消令牌。
#[derive(Clone)]
pub struct AgentExecutionManager {
    /// 事件广播通道发送端
    event_tx: broadcast::Sender<AgentExecutionEvent>,
    /// 取消令牌注册表
    cancel_tokens: std::sync::Arc<parking_lot::RwLock<std::collections::HashMap<String, CancellationToken>>>,
}

impl AgentExecutionManager {
    /// 创建新的管理器
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(256);
        Self {
            event_tx,
            cancel_tokens: std::sync::Arc::new(parking_lot::RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// 获取事件发送端（用于后台任务发送事件）
    pub fn event_sender(&self) -> broadcast::Sender<AgentExecutionEvent> {
        self.event_tx.clone()
    }

    /// 订阅事件（用于 WS handler）
    pub fn subscribe(&self) -> broadcast::Receiver<AgentExecutionEvent> {
        self.event_tx.subscribe()
    }

    /// 注册取消令牌
    pub fn register_cancel_token(&self, execution_id: &str, token: CancellationToken) {
        self.cancel_tokens.write().insert(execution_id.to_string(), token);
    }

    /// 取消指定执行
    pub fn cancel_execution(&self, execution_id: &str) -> bool {
        if let Some(token) = self.cancel_tokens.write().remove(execution_id) {
            token.cancel();
            true
        } else {
            false
        }
    }

    /// 清理已完成的执行
    pub fn remove_execution(&self, execution_id: &str) {
        self.cancel_tokens.write().remove(execution_id);
    }
}

impl Default for AgentExecutionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// WebSocket handler：订阅特定 execution_id 的事件
pub async fn handle_agent_execution_ws(
    socket: WebSocket,
    execution_id: String,
    manager: AgentExecutionManager,
) {
    let (mut sender, mut receiver) = socket.split();
    let mut event_rx = manager.subscribe();

    tracing::info!("[AgentExecWS] 客户端连接，订阅 execution_id: {}", execution_id);

    loop {
        tokio::select! {
            // 处理客户端消息（取消请求）
            msg = receiver.next() => {
                match msg {
                    Some(Ok(AxumMessage::Text(text))) => {
                        match serde_json::from_str::<ClientMessage>(&text) {
                            Ok(ClientMessage::Cancel) => {
                                tracing::info!("[AgentExecWS] 收到取消请求: {}", execution_id);
                                if manager.cancel_execution(&execution_id) {
                                    let event = AgentExecutionEvent::Cancelled {
                                        execution_id: execution_id.clone(),
                                    };
                                    if let Ok(json) = serde_json::to_string(&event) {
                                        let _ = sender.send(AxumMessage::Text(json)).await;
                                    }
                                }
                                break;
                            }
                            Err(e) => {
                                tracing::debug!("[AgentExecWS] 无法解析客户端消息: {}", e);
                            }
                        }
                    }
                    Some(Ok(AxumMessage::Ping(data))) => {
                        let _ = sender.send(AxumMessage::Pong(data)).await;
                    }
                    Some(Ok(AxumMessage::Close(_))) | None => {
                        tracing::info!("[AgentExecWS] 客户端断开: {}", execution_id);
                        break;
                    }
                    Some(Err(e)) => {
                        tracing::warn!("[AgentExecWS] WS 错误: {}", e);
                        break;
                    }
                    _ => {}
                }
            }

            // 转发匹配的执行事件
            event = event_rx.recv() => {
                match event {
                    Ok(ref exec_event) if exec_event.execution_id() == execution_id => {
                        let is_terminal = matches!(
                            exec_event,
                            AgentExecutionEvent::Completed { .. }
                            | AgentExecutionEvent::Failed { .. }
                            | AgentExecutionEvent::Cancelled { .. }
                        );

                        if let Ok(json) = serde_json::to_string(&exec_event) {
                            if sender.send(AxumMessage::Text(json)).await.is_err() {
                                break;
                            }
                        }

                        // 终态事件发送后关闭连接
                        if is_terminal {
                            tracing::info!("[AgentExecWS] 终态事件，关闭连接: {}", execution_id);
                            break;
                        }
                    }
                    Ok(_) => {
                        // 不匹配的 execution_id，忽略
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("[AgentExecWS] 落后 {} 条事件", n);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        tracing::info!("[AgentExecWS] 事件通道已关闭");
                        break;
                    }
                }
            }
        }
    }
}
