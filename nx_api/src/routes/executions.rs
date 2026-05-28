//! 执行路由

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;

use super::AppState;
use crate::services::execution_service::{ApprovalEvent, ExecutionEvent, PendingPause};
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
                total_tokens: e.total_tokens,
                total_cost_usd: e.total_cost_usd,
                team_id: e.team_id.clone(),
                project_id: e.project_id.clone(),
                trigger_source: e.trigger_source.clone(),
                approval_events: e.approval_events.clone(),
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
                    quality_gate_result: sr.quality_gate_result,
                })
                .collect(),
            started_at: execution.started_at.map(|dt| dt.to_rfc3339()),
            finished_at: execution.finished_at.map(|dt| dt.to_rfc3339()),
            error: execution.error,
            total_tokens: execution.total_tokens,
            total_cost_usd: execution.total_cost_usd,
            team_id: execution.team_id.clone(),
            project_id: execution.project_id.clone(),
            trigger_source: execution.trigger_source.clone(),
            approval_events: execution.approval_events.clone(),
            pending_pause: execution.pending_pause.clone(),
            current_stage: execution.current_stage.clone(),
            resumed_from: execution.resumed_from.clone(),
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
            total_tokens: 0,
            total_cost_usd: 0.0,
            team_id: None,
            project_id: None,
            trigger_source: None,
            approval_events: vec![],
            pending_pause: None,
            current_stage: None,
            resumed_from: None,
        })
    }
}

/// 删除执行记录（单条）
pub async fn delete_execution(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    // 从内存中移除
    state.execution_service.remove_from_memory(&id);
    // 从数据库删除
    let deleted = state.execution_service.delete_from_db(&id).unwrap_or(false);
    if deleted {
        (StatusCode::OK, Json(serde_json::json!({"ok": true})))
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"ok": false, "error": "not found"})),
        )
    }
}

/// 批量删除执行记录
pub async fn delete_executions_batch(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DeleteBatchRequest>,
) -> Json<serde_json::Value> {
    for id in &req.ids {
        state.execution_service.remove_from_memory(id);
    }
    let deleted = state
        .execution_service
        .delete_many_from_db(&req.ids)
        .unwrap_or(0);
    Json(serde_json::json!({"ok": true, "deleted": deleted}))
}

#[derive(serde::Deserialize)]
pub struct DeleteBatchRequest {
    pub ids: Vec<String>,
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

#[derive(Debug, serde::Deserialize)]
pub struct ResolveExecutionRequest {
    pub approved: bool,
    #[serde(default)]
    pub comment: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct ResolveExecutionResponse {
    pub ok: bool,
    pub execution_id: String,
}

/// POST /api/v1/executions/:id/resolve — 审批 approve/reject 并驱动引擎继续
pub async fn resolve_execution(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<ResolveExecutionRequest>,
) -> Result<Json<ResolveExecutionResponse>, ExecutionAppError> {
    state
        .execution_service
        .resolve_execution(&id, req.approved, req.comment)
        .map_err(ExecutionAppError::BadRequest)?;

    Ok(Json(ResolveExecutionResponse {
        ok: true,
        execution_id: id,
    }))
}

/// POST /api/v1/executions/:id/retry-stage — AF-UX-09 从失败阶段重试
pub async fn retry_stage_execution(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<RetryStageRequest>,
) -> Result<Json<serde_json::Value>, ExecutionAppError> {
    let execution = state
        .execution_service
        .get_execution(&id)
        .ok_or_else(|| ExecutionAppError::NotFound(format!("执行 {} 不存在", id)))?;

    let status = format!("{:?}", execution.status).to_lowercase();
    if status != "failed" && status != "cancelled" {
        return Err(ExecutionAppError::BadRequest(
            "仅失败或已取消的执行可重试".into(),
        ));
    }

    let workflow = state
        .workflow_service
        .get_workflow(&execution.workflow_id)
        .map_err(|e| ExecutionAppError::Internal(e.to_string()))?
        .ok_or_else(|| {
            ExecutionAppError::NotFound(format!("工作流 {} 不存在", execution.workflow_id))
        })?;

    let from_stage = req.from_stage.or(execution.current_stage.clone()).or_else(|| {
        execution
            .stage_results
            .last()
            .map(|s| s.stage_name.clone())
    });
    let from_stage_for_resp = from_stage.clone();

    let mut variables = if execution.variables.is_object() {
        execution.variables.clone()
    } else {
        serde_json::json!({})
    };
    if let Some(stage) = from_stage {
        if let Some(obj) = variables.as_object_mut() {
            obj.insert("retry_from_stage".into(), serde_json::json!(stage));
        }
    }
    if let Some(p) = req.approval_policy {
        if let Some(obj) = variables.as_object_mut() {
            obj.insert("approval_policy".into(), serde_json::json!(p));
        }
    }
    if let Some(m) = req.text_lane_cost_mode {
        if let Some(obj) = variables.as_object_mut() {
            obj.insert("text_lane_cost_mode".into(), serde_json::json!(m));
        }
    }
    if let Some(s) = req.skip_quality_gate_for_stage {
        if let Some(obj) = variables.as_object_mut() {
            obj.insert("skip_quality_gate_for_stage".into(), serde_json::json!(s));
        }
    }

    let mut workflow_def = serde_json::json!({
        "name": workflow.name,
        "version": workflow.version,
    });
    if let Some(desc) = &workflow.description {
        workflow_def["description"] = serde_json::json!(desc);
    }
    if let Some(obj) = workflow.definition.as_object() {
        for (k, v) in obj {
            if !["name", "version", "description"].contains(&k.as_str()) {
                workflow_def[k] = v.clone();
            }
        }
    }
    let workflow_yaml = serde_yaml::to_string(&workflow_def)
        .map_err(|e| ExecutionAppError::Internal(e.to_string()))?;

    let launch = Some(crate::services::execution_service::ExecutionLaunchContext {
        team_id: execution.team_id.clone(),
        project_id: execution.project_id.clone(),
        trigger_source: execution.trigger_source.clone().or(Some("factory".into())),
    });

    let exec_id = state
        .execution_service
        .execute_workflow(
            workflow.id.clone(),
            &workflow_yaml,
            variables,
            None,
            execution.workspace_path.clone(),
            launch,
        )
        .await
        .map_err(|e| ExecutionAppError::Internal(format!("重试启动失败: {e}")))?;

    let stage_names: Vec<String> = workflow
        .definition
        .get("stages")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|s| s.get("name").and_then(|n| n.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default();

    if let Err(e) = state.execution_service.seed_retry_lineage(
        &exec_id,
        &id,
        &stage_names,
        from_stage_for_resp.as_deref(),
    ) {
        tracing::warn!("重试继承父 Run 阶段失败: {}", e);
    }

    if let Some(ref artifact_repo) = state.artifact_repo {
        match artifact_repo.clone_execution_artifacts(&id, &exec_id) {
            Ok(n) if n > 0 => tracing::info!("重试继承 {} 条产物记录", n),
            Ok(_) => {}
            Err(e) => tracing::warn!("重试继承产物失败: {}", e),
        }
    }

    Ok(Json(serde_json::json!({
        "ok": true,
        "data": { "execution_id": exec_id, "retry_from_stage": from_stage_for_resp, "resumed_from": id }
    })))
}

#[derive(Debug, serde::Deserialize)]
pub struct RetryStageRequest {
    #[serde(default)]
    pub from_stage: Option<String>,
    #[serde(default)]
    pub approval_policy: Option<String>,
    #[serde(default)]
    pub text_lane_cost_mode: Option<String>,
    #[serde(default)]
    pub skip_quality_gate_for_stage: Option<String>,
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
            None,
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
        total_tokens: 0,
        total_cost_usd: 0.0,
        team_id: None,
        project_id: None,
        trigger_source: None,
        approval_events: vec![],
        pending_pause: None,
        current_stage: None,
        resumed_from: None,
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
                    let mut obj = serde_json::json!({
                        "stage_name": sr.stage_name,
                        "outputs": sr.outputs,
                        "completed_at": sr.completed_at.map(|dt| dt.to_rfc3339()),
                    });
                    if let Some(ref qg) = sr.quality_gate_result {
                        obj["quality_gate_result"] = qg.clone();
                    }
                    obj
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
                            ExecutionEvent::TokenUsage { execution_id, .. } => execution_id.clone(),
                            ExecutionEvent::BudgetWarning { execution_id, .. } => {
                                execution_id.clone()
                            }
                            ExecutionEvent::BudgetExceeded { execution_id, .. } => {
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
    #[serde(default)]
    pub total_tokens: i64,
    #[serde(default)]
    pub total_cost_usd: f64,
    #[serde(default)]
    pub team_id: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub trigger_source: Option<String>,
    #[serde(default)]
    pub approval_events: Vec<ApprovalEvent>,
    #[serde(default)]
    pub pending_pause: Option<PendingPause>,
    #[serde(default)]
    pub current_stage: Option<String>,
    #[serde(default)]
    pub resumed_from: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct StageResult {
    pub stage_name: String,
    pub outputs: Vec<serde_json::Value>,
    pub completed_at: Option<String>,
    #[serde(default)]
    pub quality_gate_result: Option<serde_json::Value>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ExecutionSummary {
    pub id: String,
    pub workflow_id: String,
    pub workflow_name: String,
    pub status: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    #[serde(default)]
    pub total_tokens: i64,
    #[serde(default)]
    pub total_cost_usd: f64,
    #[serde(default)]
    pub team_id: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub trigger_source: Option<String>,
    #[serde(default)]
    pub approval_events: Vec<ApprovalEvent>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CancelResponse {
    pub success: bool,
    pub message: String,
}

// ============ 错误类型 ============

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

// ============ Git 回滚 + PR 描述 ============

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct RollbackRequest {
    pub action: String,
    pub initial_branch: String,
    pub exec_branch: String,
}

#[derive(Debug, serde::Serialize)]
pub struct RollbackResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, serde::Serialize)]
pub struct GitInfoResponse {
    pub branch_info: crate::services::git_watcher::BranchInfo,
    pub commits: Vec<crate::services::git_watcher::CommitInfo>,
}

#[derive(Debug, serde::Serialize)]
pub struct PrDescriptionResponse {
    pub description: String,
}

/// 获取执行的 Git 信息（分支 + commit 列表）
pub async fn get_git_info(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<GitInfoResponse>, ExecutionAppError> {
    let branch_info = state.git_service.get_branch_info(&id);

    if !branch_info.is_git_repo {
        return Ok(Json(GitInfoResponse {
            branch_info,
            commits: vec![],
        }));
    }

    let current = branch_info.current_branch.as_deref().unwrap_or("main");
    let exec_branch = &branch_info.exec_branch;

    let commits = state
        .git_service
        .list_commits(current, exec_branch)
        .unwrap_or_default();

    Ok(Json(GitInfoResponse {
        branch_info,
        commits,
    }))
}

/// 获取 commit 的 diff
pub async fn get_commit_diff(
    State(state): State<Arc<AppState>>,
    Path((id, hash)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ExecutionAppError> {
    let diff = state
        .git_service
        .get_commit_diff(&hash)
        .map_err(|e| ExecutionAppError::Internal(e))?;

    Ok(Json(serde_json::json!({
        "execution_id": id,
        "commit_hash": hash,
        "diff": diff,
    })))
}

/// 回滚执行
pub async fn rollback_execution(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<RollbackRequest>,
) -> Result<Json<RollbackResponse>, ExecutionAppError> {
    let result = match req.action.as_str() {
        "revert" => state
            .git_service
            .rollback_revert(&req.initial_branch, &req.exec_branch),
        "keep" => state
            .git_service
            .rollback_keep(&req.initial_branch, &req.exec_branch),
        "branch" => state
            .git_service
            .rollback_branch(&id, &req.initial_branch, &req.exec_branch),
        _ => Err(format!("未知的回滚操作: {}", req.action)),
    };

    match result {
        Ok(()) => Ok(Json(RollbackResponse {
            success: true,
            message: format!("回滚操作 {} 完成", req.action),
        })),
        Err(e) => Ok(Json(RollbackResponse {
            success: false,
            message: e,
        })),
    }
}

/// 获取 PR 描述
pub async fn get_pr_description(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<PrDescriptionResponse>, ExecutionAppError> {
    let branch_info = state.git_service.get_branch_info(&id);

    let current = branch_info.current_branch.as_deref().unwrap_or("main");
    let exec_branch = &branch_info.exec_branch;

    let description = state
        .git_service
        .generate_pr_description(current, exec_branch)
        .map_err(|e| ExecutionAppError::Internal(e))?;

    Ok(Json(PrDescriptionResponse { description }))
}

// ============ Cost API ============

use axum::extract::Query;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CostByDayQuery {
    #[serde(default = "default_days")]
    pub days: u32,
}

fn default_days() -> u32 {
    30
}

/// GET /api/v1/costs/summary — 总体花费统计
pub async fn cost_summary(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let result = state
        .execution_repo
        .as_ref()
        .map(|repo| repo.cost_summary());

    match result {
        Some(Ok((total_tokens, total_cost_usd, total_executions))) => Json(serde_json::json!({
            "total_tokens": total_tokens,
            "total_cost_usd": total_cost_usd,
            "total_executions": total_executions,
        })),
        _ => Json(serde_json::json!({
            "total_tokens": 0,
            "total_cost_usd": 0.0,
            "total_executions": 0,
        })),
    }
}

/// GET /api/v1/costs/by-day — 按天聚合 token/cost
pub async fn cost_by_day(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CostByDayQuery>,
) -> Json<serde_json::Value> {
    let result = state
        .execution_repo
        .as_ref()
        .map(|repo| repo.cost_by_day(query.days));

    match result {
        Some(Ok(rows)) => Json(serde_json::json!({
            "days": query.days,
            "data": rows.into_iter().map(|(day, tokens, cost)| serde_json::json!({
                "date": day,
                "tokens": tokens,
                "cost_usd": cost,
            })).collect::<Vec<_>>(),
        })),
        _ => Json(serde_json::json!({"days": query.days, "data": []})),
    }
}

/// GET /api/v1/costs/by-workflow — 按工作流聚合 cost
pub async fn cost_by_workflow(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let result = state
        .execution_repo
        .as_ref()
        .map(|repo| repo.cost_by_workflow());

    match result {
        Some(Ok(rows)) => Json(serde_json::json!({
            "workflows": rows.into_iter().map(|(workflow_id, tokens, cost, count)| serde_json::json!({
                "workflow_id": workflow_id,
                "total_tokens": tokens,
                "total_cost_usd": cost,
                "execution_count": count,
            })).collect::<Vec<_>>(),
        })),
        _ => Json(serde_json::json!({"workflows": []})),
    }
}
