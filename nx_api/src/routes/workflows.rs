//! 工作流路由

use axum::{
    extract::{Path, State},
    Json,
};
use std::sync::Arc;

use super::AppState;
use crate::routes::executions::ExecutionResponse;
use crate::services::workflow_service::WorkflowServiceError;

/// 列出工作流
pub async fn list_workflows(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<WorkflowSummary>>, AppError> {
    let workflows = state
        .workflow_service
        .list_workflows()
        .map_err(AppError::from)?;
    let summaries = workflows
        .into_iter()
        .map(|w| WorkflowSummary {
            id: w.id,
            name: w.name,
            version: w.version,
            description: w.description,
            stage_count: count_stages(&w.definition),
            agent_count: count_agents(&w.definition),
        })
        .collect();
    Ok(Json(summaries))
}

/// 创建工作流
pub async fn create_workflow(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateWorkflowRequest>,
) -> Result<Json<WorkflowResponse>, AppError> {
    let workflow = state
        .workflow_service
        .create_workflow(
            payload.name,
            payload.version,
            payload.description,
            payload.definition,
        )
        .map_err(AppError::from)?;

    Ok(Json(WorkflowResponse {
        id: workflow.id,
        name: workflow.name,
        version: workflow.version,
        description: workflow.description,
        definition: workflow.definition,
        created_at: workflow.created_at.to_rfc3339(),
        updated_at: workflow.updated_at.to_rfc3339(),
    }))
}

/// 获取工作流
pub async fn get_workflow(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<WorkflowResponse>, AppError> {
    let workflow = state
        .workflow_service
        .get_workflow(&id)
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound(format!("工作流 {} 不存在", id)))?;

    Ok(Json(WorkflowResponse {
        id: workflow.id,
        name: workflow.name,
        version: workflow.version,
        description: workflow.description,
        definition: workflow.definition,
        created_at: workflow.created_at.to_rfc3339(),
        updated_at: workflow.updated_at.to_rfc3339(),
    }))
}

/// 更新工作流
pub async fn update_workflow(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateWorkflowRequest>,
) -> Result<Json<WorkflowResponse>, AppError> {
    let workflow = state
        .workflow_service
        .update_workflow(
            &id,
            payload.name,
            payload.version,
            payload.description,
            payload.definition,
        )
        .map_err(AppError::from)?;

    Ok(Json(WorkflowResponse {
        id: workflow.id,
        name: workflow.name,
        version: workflow.version,
        description: workflow.description,
        definition: workflow.definition,
        created_at: workflow.created_at.to_rfc3339(),
        updated_at: workflow.updated_at.to_rfc3339(),
    }))
}

/// 删除工作流
pub async fn delete_workflow(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<DeleteResponse>, AppError> {
    state
        .workflow_service
        .delete_workflow(&id)
        .map_err(AppError::from)?;
    Ok(Json(DeleteResponse {
        success: true,
        message: format!("工作流 {} 已删除", id),
    }))
}

/// 执行工作流
pub async fn execute_workflow(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<ExecuteWorkflowRequest>,
) -> Result<Json<ExecutionResponse>, AppError> {
    // 1. 获取工作流
    let workflow = state
        .workflow_service
        .get_workflow(&id)
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound(format!("工作流 {} 不存在", id)))?;

    // 2. 构建完整的 WorkflowDefinition JSON
    let mut workflow_def = serde_json::json!({
        "name": workflow.name,
        "version": workflow.version,
    });
    if let Some(desc) = &workflow.description {
        workflow_def["description"] = serde_json::json!(desc);
    }
    if let Some(obj) = workflow.definition.as_object() {
        for (key, value) in obj {
            if !["name", "version", "description"].contains(&key.as_str()) {
                workflow_def[key] = value.clone();
            }
        }
    }

    // 3. 转换为 YAML
    let workflow_yaml = serde_yaml::to_string(&workflow_def)
        .map_err(|e| AppError::Internal(format!("YAML 序列化失败: {}", e)))?;

    // 4. 获取工作区路径
    let current_workspace = state.current_workspace_path.read().clone();

    // 5. 真正启动执行
    let variables = payload.variables.unwrap_or(serde_json::json!({}));
    let execution_id = state
        .execution_service
        .execute_workflow(
            workflow.id.clone(),
            &workflow_yaml,
            variables.clone(),
            None,
            current_workspace,
        )
        .await
        .map_err(|e| AppError::Internal(format!("执行启动失败: {}", e)))?;

    tracing::info!("启动工作流 {} 执行，ID: {}", id, execution_id);

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

// ============ 辅助函数 ============

/// 从工作流定义中统计阶段数
fn count_stages(definition: &serde_json::Value) -> usize {
    definition
        .get("stages")
        .and_then(|s| s.as_array())
        .map(|arr| arr.len())
        .unwrap_or(0)
}

/// 从工作流定义中统计智能体数量
fn count_agents(definition: &serde_json::Value) -> usize {
    // 优先从顶层 agents 数组计数（去重）
    definition
        .get("agents")
        .and_then(|a| a.as_array())
        .map(|arr| arr.len())
        .unwrap_or_else(|| {
            // fallback: 从 stages 内部的 agents 引用计数
            definition
                .get("stages")
                .and_then(|s| s.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|stage| stage.get("agents"))
                        .filter_map(|a| a.as_array())
                        .map(|agents| agents.len())
                        .sum()
                })
                .unwrap_or(0)
        })
}

// ============ 错误类型 ============

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

/// 应用错误类型
#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    BadRequest(String),
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::Internal(msg) => {
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

impl From<WorkflowServiceError> for AppError {
    fn from(err: WorkflowServiceError) -> Self {
        match err {
            WorkflowServiceError::NotFound(id) => AppError::NotFound(id),
            WorkflowServiceError::AlreadyExists(id) => {
                AppError::BadRequest(format!("工作流 {} 已存在", id))
            }
            WorkflowServiceError::Internal(msg) => AppError::Internal(msg),
        }
    }
}

// ============ 请求/响应类型 ============

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateWorkflowRequest {
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    pub definition: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateWorkflowRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub definition: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecuteWorkflowRequest {
    #[serde(default)]
    pub variables: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowResponse {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub definition: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowSummary {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub stage_count: usize,
    pub agent_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteResponse {
    pub success: bool,
    pub message: String,
}
