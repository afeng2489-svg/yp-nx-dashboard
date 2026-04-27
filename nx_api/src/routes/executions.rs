//! 执行路由

use axum::{
    extract::{Path, State},
    Json,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;

use super::AppState;
use crate::services::execution_service::ExecutionEvent;
use crate::services::workflow_service::WorkflowServiceError;

/// 列出执行记录
pub async fn list_executions(State(state): State<Arc<AppState>>) -> Json<Vec<ExecutionSummary>> {
    let executions = state.execution_service.get_all_executions();
    let summaries: Vec<ExecutionSummary> = executions
        .into_iter()
        .map(|e| {
            let workflow_id = e.workflow_id.clone();
            let workflow_name = state
                .workflow_service
                .get_workflow(&workflow_id)
                .ok()
                .flatten()
                .map(|w| w.name)
                .unwrap_or_else(|| workflow_id.clone());
            ExecutionSummary {
                id: e.id,
                workflow_id,
                workflow_name,
                status: format!("{:?}", e.status).to_lowercase(),
                started_at: e.started_at.map(|dt| dt.to_rfc3339()),
                finished_at: e.finished_at.map(|dt| dt.to_rfc3339()),
            }
        })
        .collect();
    Json(summaries)
}

/// 获取执行详情
pub async fn get_execution(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<ExecutionResponse> {
    if let Some(execution) = state.execution_service.get_execution(&id) {
        Json(ExecutionResponse {
            id: execution.id,
            workflow_id: execution.workflow_id,
            status: format!("{:?}", execution.status).to_lowercase(),
            variables: execution.variables,
            stage_results: execution
                .stage_results
                .into_iter()
                .map(|sr| StageResult {
                    stage_name: sr.stage_name,
                    outputs: sr.outputs,
                    completed_at: sr.completed_at.map(|dt| dt.to_rfc3339()),
                })
                .collect(),
            started_at: execution.started_at.map(|dt| dt.to_rfc3339()),
            finished_at: execution.finished_at.map(|dt| dt.to_rfc3339()),
            error: execution.error,
        })
    } else {
        Json(ExecutionResponse {
            id,
            workflow_id: "unknown".to_string(),
            status: "not_found".to_string(),
            variables: serde_json::json!({}),
            stage_results: vec![],
            started_at: None,
            finished_at: None,
            error: Some("执行不存在".to_string()),
        })
    }
}

/// 取消执行
pub async fn cancel_execution(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<CancelResponse> {
    tracing::info!("取消执行: {}", id);
    let success = state.execution_service.cancel_execution(&id);
    Json(CancelResponse {
        success,
        message: if success {
            format!("执行 {} 已取消", id)
        } else {
            format!("执行 {} 不存在", id)
        },
    })
}

/// 启动执行
pub async fn start_execution(
    State(state): State<Arc<AppState>>,
    Json(req): Json<StartExecutionRequest>,
) -> Result<Json<ExecutionResponse>, ExecutionAppError> {
    tracing::info!("启动执行: workflow_id={}", req.workflow_id);

    // 克隆 variables 以便后续使用
    let variables = req.variables.clone();

    // 1. 获取工作流定义
    let workflow = state
        .workflow_service
        .get_workflow(&req.workflow_id)
        .map_err(|e| match e {
            WorkflowServiceError::NotFound(id) => {
                ExecutionAppError::NotFound(format!("工作流 {} 不存在", id))
            }
            _ => ExecutionAppError::Internal(e.to_string()),
        })?
        .ok_or_else(|| ExecutionAppError::NotFound(format!("工作流 {} 不存在", req.workflow_id)))?;

    // 2. 构建完整的 WorkflowDefinition JSON（包含 name, version 等顶层字段）
    let mut workflow_def = serde_json::json!({
        "name": workflow.name,
        "version": workflow.version,
    });

    // 合并 description
    if let Some(desc) = workflow.description {
        workflow_def["description"] = serde_json::json!(desc);
    }

    // 合并 definition 中的其他字段（stages, agents, variables, triggers）
    if let Some(obj) = workflow.definition.as_object() {
        for (key, value) in obj {
            if !["name", "version", "description"].contains(&key.as_str()) {
                workflow_def[key] = value.clone();
            }
        }
    }

    // 3. 将完整工作流定义转换为 YAML
    let workflow_yaml = serde_yaml::to_string(&workflow_def)
        .map_err(|e| ExecutionAppError::Internal(format!("工作流 YAML 序列化失败: {}", e)))?;

    // 4. 获取当前工作区路径
    let current_workspace = state.current_workspace_path.read().clone();

    // 5. 启动真实执行（异步，不等待完成）
    let execution_id = state
        .execution_service
        .execute_workflow(
            workflow.id.clone(),
            &workflow_yaml,
            variables.clone(),
            None, // 使用默认 AI 配置
            current_workspace,
        )
        .await
        .map_err(|e| ExecutionAppError::Internal(format!("执行启动失败: {}", e)))?;

    // 5. 立即返回（执行在后台运行）
    Ok(Json(ExecutionResponse {
        id: execution_id,
        workflow_id: workflow.id,
        status: "running".to_string(),
        variables,
        stage_results: vec![],
        started_at: Some(chrono::Utc::now().to_rfc3339()),
        finished_at: None,
        error: None,
    }))
}

/// 执行 WebSocket 流
pub async fn execution_ws(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    ws: axum::extract::WebSocketUpgrade,
) -> impl axum::response::IntoResponse {
    tracing::info!("执行 WebSocket 连接: {}", id);

    ws.on_upgrade(|socket: axum::extract::ws::WebSocket| async move {
        use axum::extract::ws::Message;
        use tokio::sync::broadcast::error::RecvError;

        let (mut sender, mut receive) = socket.split();

        // 先订阅，再发送快照，避免错过事件（race condition fix）
        let mut receiver = state.execution_service.subscribe();

        // 立即发送当前执行状态快照（catch-up）
        if let Some(execution) = state.execution_service.get_execution(&id) {
            let stage_results: Vec<serde_json::Value> = execution
                .stage_results
                .iter()
                .map(|sr| {
                    serde_json::json!({
                        "stage_name": sr.stage_name,
                        "outputs": sr.outputs,
                        "completed_at": sr.completed_at.map(|dt| dt.to_rfc3339()),
                    })
                })
                .collect();

            let status_json = serde_json::to_value(execution.status)
                .ok()
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| "unknown".to_string());

            let snapshot = serde_json::json!({
                "type": "snapshot",
                "execution_id": execution.id,
                "status": status_json,
                "stage_results": stage_results,
                "error": execution.error,
                "output_log": execution.output_log,
                "current_stage": execution.current_stage,
                "pending_pause": execution.pending_pause,
            });

            if let Ok(json) = serde_json::to_string(&snapshot) {
                if sender.send(Message::Text(json)).await.is_err() {
                    return; // 客户端已断开
                }
            }
        }

        let exec_service = state.execution_service.clone();
        let target_id = id.clone();

        // 接收客户端消息
        let receive_task = tokio::spawn(async move {
            while let Some(msg) = receive.next().await {
                if let Ok(Message::Text(text)) = msg {
                    tracing::debug!("收到客户端消息: {}", text);
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                        // Support both legacy `action` field and new `type` field
                        let action = json
                            .get("action")
                            .and_then(|v| v.as_str())
                            .or_else(|| json.get("type").and_then(|v| v.as_str()))
                            .unwrap_or("");
                        let normalized = match action {
                            "resume_workflow" => "resume",
                            "cancel_execution" => "cancel",
                            other => other,
                        };
                        match normalized {
                            "cancel" => {
                                if let Some(exec_id) =
                                    json.get("execution_id").and_then(|v| v.as_str())
                                {
                                    exec_service.cancel_execution(exec_id);
                                }
                            }
                            "resume" => {
                                if let Some(exec_id) =
                                    json.get("execution_id").and_then(|v| v.as_str())
                                {
                                    if let Some(value) = json.get("value").and_then(|v| v.as_str())
                                    {
                                        exec_service.resume_execution(exec_id, value.to_string());
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        });

        // 发送执行事件（处理 lagged 错误）
        let send_task = tokio::spawn(async move {
            loop {
                match receiver.recv().await {
                    Ok(event) => {
                        let event_id = match &event {
                            ExecutionEvent::Started { execution_id, .. } => execution_id.clone(),
                            ExecutionEvent::StatusChanged { execution_id, .. } => {
                                execution_id.clone()
                            }
                            ExecutionEvent::StageStarted { execution_id, .. } => {
                                execution_id.clone()
                            }
                            ExecutionEvent::StageCompleted { execution_id, .. } => {
                                execution_id.clone()
                            }
                            ExecutionEvent::Output { execution_id, .. } => execution_id.clone(),
                            ExecutionEvent::Completed { execution_id } => execution_id.clone(),
                            ExecutionEvent::Failed { execution_id, .. } => execution_id.clone(),
                            ExecutionEvent::WorkflowPaused { execution_id, .. } => {
                                execution_id.clone()
                            }
                            ExecutionEvent::WorkflowResumed { execution_id, .. } => {
                                execution_id.clone()
                            }
                        };

                        if event_id == target_id {
                            if let Ok(json) = serde_json::to_string(&event) {
                                if sender.send(Message::Text(json)).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                    Err(RecvError::Lagged(n)) => {
                        tracing::warn!("WebSocket 接收器滞后 {} 条消息，继续", n);
                        // 继续接收，不断开
                    }
                    Err(RecvError::Closed) => {
                        break;
                    }
                }
            }
        });

        tokio::select! {
            _ = receive_task => {}
            _ = send_task => {}
        }
    })
}

// ============ 请求类型 ============

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct StartExecutionRequest {
    pub workflow_id: String,
    #[serde(default = "default_variables")]
    pub variables: serde_json::Value,
}

fn default_variables() -> serde_json::Value {
    serde_json::json!({})
}

// ============ 响应类型 ============

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ExecutionResponse {
    pub id: String,
    pub workflow_id: String,
    pub status: String,
    pub variables: serde_json::Value,
    pub stage_results: Vec<StageResult>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct StageResult {
    pub stage_name: String,
    pub outputs: Vec<serde_json::Value>,
    pub completed_at: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ExecutionSummary {
    pub id: String,
    pub workflow_id: String,
    pub workflow_name: String,
    pub status: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CancelResponse {
    pub success: bool,
    pub message: String,
}

// ============ 错误类型 ============

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

/// 执行路由错误
#[derive(Debug)]
pub enum ExecutionAppError {
    NotFound(String),
    BadRequest(String),
    Internal(String),
}

impl IntoResponse for ExecutionAppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ExecutionAppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            ExecutionAppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            ExecutionAppError::Internal(msg) => {
                tracing::error!("内部错误: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "内部服务器错误".to_string(),
                )
            }
        };

        let body = serde_json::json!({
            "error": message
        });

        (status, Json(body)).into_response()
    }
}
