//! WebSocket 处理器
//!
//! 处理 WebSocket 连接，路由消息到 ExecutionService

use axum::extract::ws::{Message as AxumMessage, WebSocket};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::services::events::{ExecutionEvent, ExecutionStatus, WorkflowOption};
use crate::services::execution_service::ExecutionService;

/// WebSocket 消息协议（客户端 -> 服务端）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// 执行工作流
    ExecuteWorkflow {
        workflow_id: String,
        variables: Option<serde_json::Value>,
    },
    /// 取消执行
    CancelExecution { execution_id: String },
    /// 订阅执行事件
    Subscribe { execution_id: String },
    /// 取消订阅
    Unsubscribe { execution_id: String },
    /// 恢复暂停的工作流
    ResumeWorkflow { execution_id: String, value: String },
}

/// WebSocket 消息协议（服务端 -> 客户端）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// 执行已开始
    ExecutionStarted {
        execution_id: String,
        workflow_id: String,
    },
    /// 状态变更
    StatusChanged {
        execution_id: String,
        status: String,
    },
    /// 阶段开始
    StageStarted {
        execution_id: String,
        stage_name: String,
    },
    /// 阶段完成
    StageCompleted {
        execution_id: String,
        stage_name: String,
        output: serde_json::Value,
        quality_gate_result: Option<serde_json::Value>,
    },
    /// 输出行
    Output { execution_id: String, line: String },
    /// 执行完成
    Completed { execution_id: String },
    /// 执行失败
    Failed { execution_id: String, error: String },
    /// 错误消息
    Error { message: String },
    /// 工作流暂停，等待用户选择
    WorkflowPaused {
        execution_id: String,
        stage_name: String,
        question: String,
        options: Vec<WorkflowOption>,
    },
    /// 工作流已从暂停恢复
    WorkflowResumed {
        execution_id: String,
        stage_name: String,
        chosen_value: String,
    },
    /// Token/Cost 用量更新
    TokenUsage {
        execution_id: String,
        total_tokens: i64,
        total_cost_usd: f64,
    },
    /// 预算告警
    BudgetWarning {
        execution_id: String,
        current_usd: f64,
        limit_usd: f64,
        percentage: f64,
    },
    /// 预算超限
    BudgetExceeded {
        execution_id: String,
        current_usd: f64,
        limit_usd: f64,
    },
}

impl ServerMessage {
    /// 将消息序列化为 JSON 字符串
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// WebSocket 会话状态
#[derive(Default)]
struct WsSession {
    /// 订阅的执行 ID 集合
    subscriptions: HashSet<String>,
}

/// WebSocket 处理器
#[derive(Debug, Clone)]
pub struct WebSocketHandler;

impl WebSocketHandler {
    /// 处理 WebSocket 连接
    ///
    /// # Arguments
    /// * `socket` - WebSocket 连接
    /// * `execution_service` - 执行服务（用于订阅事件）
    pub async fn handle(socket: WebSocket, execution_service: ExecutionService) {
        let (mut sender, mut receiver) = socket.split();
        let mut session = WsSession::default();

        // 订阅执行事件
        let mut event_rx = execution_service.subscribe();

        loop {
            tokio::select! {
                // 处理客户端消息
                msg = receiver.next() => {
                    match msg {
                        Some(Ok(AxumMessage::Text(text))) => {
                            if let Err(e) = Self::handle_client_message(
                                &text,
                                &execution_service,
                                &mut sender,
                                &mut session,
                            ).await {
                                tracing::error!("Error handling client message: {}", e);
                                let error_msg = ServerMessage::Error {
                                    message: e.to_string(),
                                };
                                if let Ok(json) = error_msg.to_json() {
                                    let _ = sender.send(AxumMessage::Text(json)).await;
                                }
                            }
                        }
                        Some(Ok(AxumMessage::Binary(data))) => {
                            tracing::debug!("Received binary message: {} bytes", data.len());
                        }
                        Some(Ok(AxumMessage::Ping(data))) => {
                            tracing::debug!("Received Ping");
                            // 自动回复 Pong
                            if sender.send(AxumMessage::Pong(data)).await.is_err() {
                                break;
                            }
                        }
                        Some(Ok(AxumMessage::Pong(_))) => {
                            tracing::debug!("Received Pong");
                        }
                        Some(Ok(AxumMessage::Close(close_frame))) => {
                            tracing::info!("WebSocket close: {:?}", close_frame);
                            break;
                        }
                        Some(Err(e)) => {
                            let error_str = e.to_string();
                            if error_str.contains("reset without closing")
                                || error_str.contains("connection closed")
                            {
                                tracing::warn!("WebSocket 客户端断连: {}", error_str);
                            } else {
                                tracing::error!("WebSocket 错误: {}", e);
                            }
                            break;
                        }
                        None => {
                            tracing::debug!("WebSocket 消息流结束");
                            break;
                        }
                    }
                }

                // 处理执行事件
                event = event_rx.recv() => {
                    match event {
                        Ok(exec_event) => {
                            let execution_id = Self::get_execution_id(&exec_event);
                            if session.subscriptions.contains(&execution_id) {
                                if let Some(msg) = Self::event_to_message(exec_event) {
                                    if let Ok(json) = msg.to_json() {
                                        if sender.send(AxumMessage::Text(json)).await.is_err() {
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                            tracing::warn!("Event channel lagged behind, skipping events");
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            tracing::info!("Event channel closed, ending session");
                            break;
                        }
                    }
                }
            }
        }
    }

    /// 获取事件关联的执行 ID
    fn get_execution_id(event: &ExecutionEvent) -> String {
        match event {
            ExecutionEvent::Started { execution_id, .. } => execution_id.clone(),
            ExecutionEvent::StatusChanged { execution_id, .. } => execution_id.clone(),
            ExecutionEvent::StageStarted { execution_id, .. } => execution_id.clone(),
            ExecutionEvent::StageCompleted { execution_id, .. } => execution_id.clone(),
            ExecutionEvent::Output { execution_id, .. } => execution_id.clone(),
            ExecutionEvent::Completed { execution_id } => execution_id.clone(),
            ExecutionEvent::Failed { execution_id, .. } => execution_id.clone(),
            ExecutionEvent::WorkflowPaused { execution_id, .. } => execution_id.clone(),
            ExecutionEvent::WorkflowResumed { execution_id, .. } => execution_id.clone(),
            ExecutionEvent::TokenUsage { execution_id, .. } => execution_id.clone(),
            ExecutionEvent::BudgetWarning { execution_id, .. } => execution_id.clone(),
            ExecutionEvent::BudgetExceeded { execution_id, .. } => execution_id.clone(),
        }
    }

    /// 将执行事件转换为 WebSocket 消息
    fn event_to_message(event: ExecutionEvent) -> Option<ServerMessage> {
        match event {
            ExecutionEvent::Started {
                execution_id,
                workflow_id,
            } => Some(ServerMessage::ExecutionStarted {
                execution_id,
                workflow_id,
            }),
            ExecutionEvent::StatusChanged {
                execution_id,
                status,
            } => Some(ServerMessage::StatusChanged {
                execution_id,
                status: format!("{:?}", status),
            }),
            ExecutionEvent::StageStarted {
                execution_id,
                stage_name,
            } => Some(ServerMessage::StageStarted {
                execution_id,
                stage_name,
            }),
            ExecutionEvent::StageCompleted {
                execution_id,
                stage_name,
                output,
                quality_gate_result,
            } => Some(ServerMessage::StageCompleted {
                execution_id,
                stage_name,
                output,
                quality_gate_result,
            }),
            ExecutionEvent::Output { execution_id, line } => {
                Some(ServerMessage::Output { execution_id, line })
            }
            ExecutionEvent::Completed { execution_id } => {
                Some(ServerMessage::Completed { execution_id })
            }
            ExecutionEvent::Failed {
                execution_id,
                error,
            } => Some(ServerMessage::Failed {
                execution_id,
                error,
            }),
            ExecutionEvent::WorkflowPaused {
                execution_id,
                stage_name,
                question,
                options,
            } => Some(ServerMessage::WorkflowPaused {
                execution_id,
                stage_name,
                question,
                options,
            }),
            ExecutionEvent::WorkflowResumed {
                execution_id,
                stage_name,
                chosen_value,
            } => Some(ServerMessage::WorkflowResumed {
                execution_id,
                stage_name,
                chosen_value,
            }),
            ExecutionEvent::TokenUsage {
                execution_id,
                total_tokens,
                total_cost_usd,
            } => Some(ServerMessage::TokenUsage {
                execution_id,
                total_tokens,
                total_cost_usd,
            }),
            ExecutionEvent::BudgetWarning {
                execution_id,
                current_usd,
                limit_usd,
                percentage,
            } => Some(ServerMessage::BudgetWarning {
                execution_id,
                current_usd,
                limit_usd,
                percentage,
            }),
            ExecutionEvent::BudgetExceeded {
                execution_id,
                current_usd,
                limit_usd,
            } => Some(ServerMessage::BudgetExceeded {
                execution_id,
                current_usd,
                limit_usd,
            }),
        }
    }

    /// 处理客户端消息
    async fn handle_client_message(
        text: &str,
        execution_service: &ExecutionService,
        sender: &mut futures_util::stream::SplitSink<WebSocket, AxumMessage>,
        session: &mut WsSession,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let msg: ClientMessage = serde_json::from_str(text)?;

        match msg {
            ClientMessage::ExecuteWorkflow {
                workflow_id,
                variables,
            } => {
                tracing::info!("Execute workflow: {}", workflow_id);

                let execution = execution_service.start_execution(
                    workflow_id.clone(),
                    variables.unwrap_or_else(|| serde_json::json!({})),
                );

                session.subscriptions.insert(execution.id.clone());

                let response = ServerMessage::ExecutionStarted {
                    execution_id: execution.id,
                    workflow_id,
                };
                let json = response.to_json()?;
                sender.send(AxumMessage::Text(json)).await?;
            }

            ClientMessage::CancelExecution { execution_id } => {
                tracing::info!("Cancel execution: {}", execution_id);

                if execution_service.cancel_execution(&execution_id) {
                    let response = ServerMessage::StatusChanged {
                        execution_id,
                        status: format!("{:?}", ExecutionStatus::Cancelled),
                    };
                    let json = response.to_json()?;
                    sender.send(AxumMessage::Text(json)).await?;
                }
            }

            ClientMessage::Subscribe { execution_id } => {
                tracing::debug!("Subscribe to execution: {}", execution_id);
                session.subscriptions.insert(execution_id);
            }

            ClientMessage::Unsubscribe { execution_id } => {
                tracing::debug!("Unsubscribe from execution: {}", execution_id);
                session.subscriptions.remove(&execution_id);
            }

            ClientMessage::ResumeWorkflow {
                execution_id,
                value,
            } => {
                tracing::info!("Resume workflow: {} with value: {}", execution_id, value);
                if !execution_service.resume_execution(&execution_id, value) {
                    let response = ServerMessage::Error {
                        message: format!("Execution {} is not paused or not found", execution_id),
                    };
                    let json = response.to_json()?;
                    sender.send(AxumMessage::Text(json)).await?;
                }
            }
        }

        Ok(())
    }
}
