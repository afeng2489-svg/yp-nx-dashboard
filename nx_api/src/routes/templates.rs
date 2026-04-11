//! 工作流模板路由

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use super::AppState;

/// 模板元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateMetadata {
    pub name: String,
    pub description: String,
    pub category: String,
}

/// 模板定义（完整结构）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateDefinition {
    pub name: String,
    pub description: String,
    pub category: String,
    pub stages: Vec<Stage>,
    pub agents: Vec<Agent>,
}

/// 阶段定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stage {
    pub name: String,
    pub agents: Vec<String>,
    #[serde(default)]
    pub parallel: bool,
}

/// Agent 定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub role: String,
    pub model: String,
    pub prompt: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
}

/// 模板摘要响应
#[derive(Debug, Serialize, Deserialize)]
pub struct TemplateSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub stage_count: usize,
    pub agent_count: usize,
}

/// 模板详情响应
#[derive(Debug, Serialize, Deserialize)]
pub struct TemplateResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub stages: Vec<Stage>,
    pub agents: Vec<Agent>,
}

/// 创建模板请求
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTemplateRequest {
    pub name: String,
    pub description: String,
    pub category: String,
    pub stages: Vec<Stage>,
    pub agents: Vec<Agent>,
}

/// 实例化模板请求
#[derive(Debug, Serialize, Deserialize)]
pub struct InstantiateTemplateRequest {
    #[serde(default)]
    pub variables: Option<serde_json::Value>,
}

/// 列表响应
#[derive(Debug, Serialize, Deserialize)]
pub struct ListResponse<T> {
    pub items: Vec<T>,
    pub total: usize,
}

/// 应用状态扩展 - 包含模板路径
pub struct TemplateState {
    pub templates_path: PathBuf,
}

/// 获取模板目录路径
fn get_templates_path() -> PathBuf {
    PathBuf::from(std::env::var("TEMPLATES_PATH").unwrap_or_else(|_| {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(PathBuf::new)
            .join("templates")
            .to_string_lossy()
            .to_string()
    }))
}

/// 列出所有模板
pub async fn list_templates(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<ListResponse<TemplateSummary>>, AppError> {
    let templates_path = get_templates_path();

    let mut templates = Vec::new();

    if templates_path.exists() {
        for entry in fs::read_dir(&templates_path).map_err(|e| AppError::Internal(e.to_string()))? {
            let entry = entry.map_err(|e| AppError::Internal(e.to_string()))?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                match parse_template_file(&path) {
                    Ok((id, template)) => {
                        templates.push(TemplateSummary {
                            id,
                            name: template.name.clone(),
                            description: template.description.clone(),
                            category: template.category.clone(),
                            stage_count: template.stages.len(),
                            agent_count: template.agents.len(),
                        });
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse template {:?}: {}", path, e);
                    }
                }
            }
        }
    }

    // 按名称排序
    templates.sort_by(|a, b| a.name.cmp(&b.name));

    let total = templates.len();

    Ok(Json(ListResponse { items: templates, total }))
}

/// 获取单个模板
pub async fn get_template(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<TemplateResponse>, AppError> {
    let templates_path = get_templates_path();
    let template_path = templates_path.join(format!("{}.yaml", id));

    if !template_path.exists() {
        return Err(AppError::NotFound(format!("Template '{}' not found", id)));
    }

    let (template_id, template) = parse_template_file(&template_path)?;

    Ok(Json(TemplateResponse {
        id: template_id,
        name: template.name,
        description: template.description,
        category: template.category,
        stages: template.stages,
        agents: template.agents,
    }))
}

/// 创建模板
pub async fn create_template(
    State(_state): State<Arc<AppState>>,
    Json(payload): Json<CreateTemplateRequest>,
) -> Result<Json<TemplateResponse>, AppError> {
    let templates_path = get_templates_path();

    // 确保目录存在
    fs::create_dir_all(&templates_path).map_err(|e| AppError::Internal(e.to_string()))?;

    // 生成 ID
    let id = payload.name.replace(' ', "-").to_lowercase();
    let template_path = templates_path.join(format!("{}.yaml", id));

    if template_path.exists() {
        return Err(AppError::BadRequest(format!("Template '{}' already exists", id)));
    }

    // 创建模板定义
    let template_def = TemplateDefinition {
        name: payload.name.clone(),
        description: payload.description.clone(),
        category: payload.category.clone(),
        stages: payload.stages.clone(),
        agents: payload.agents.clone(),
    };

    // 序列化为 YAML
    let yaml = serde_yaml::to_string(&template_def).map_err(|e| AppError::Internal(e.to_string()))?;

    // 写入文件
    fs::write(&template_path, yaml).map_err(|e| AppError::Internal(e.to_string()))?;

    tracing::info!("Created template '{}' at {:?}", id, template_path);

    Ok(Json(TemplateResponse {
        id,
        name: payload.name,
        description: payload.description,
        category: payload.category,
        stages: payload.stages,
        agents: payload.agents,
    }))
}

/// 从模板实例化工作流
pub async fn instantiate_template(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<InstantiateTemplateRequest>,
) -> Result<Json<InstantiateResponse>, AppError> {
    let templates_path = get_templates_path();
    let template_path = templates_path.join(format!("{}.yaml", id));

    if !template_path.exists() {
        return Err(AppError::NotFound(format!("Template '{}' not found", id)));
    }

    // 解析模板
    let (_, template_def) = parse_template_file(&template_path)?;

    // 创建工作流定义
    let workflow_definition = serde_json::json!({
        "stages": template_def.stages,
        "agents": template_def.agents,
        "variables": payload.variables.unwrap_or(serde_json::json!({})),
    });

    // 生成工作流 ID
    let workflow_id = uuid::Uuid::new_v4().to_string();

    // 使用工作流服务创建工作流
    let workflow = state
        .workflow_service
        .create_workflow(
            template_def.name.clone(),
            Some("1.0.0".to_string()),
            Some(template_def.description.clone()),
            workflow_definition,
        )
        .map_err(AppError::from)?;

    tracing::info!(
        "Instantiated template '{}' into workflow '{}'",
        id,
        workflow.id
    );

    Ok(Json(InstantiateResponse {
        workflow_id: workflow.id,
        name: workflow.name,
        description: workflow.description,
        created_at: workflow.created_at.to_rfc3339(),
    }))
}

/// 实例化响应
#[derive(Debug, Serialize, Deserialize)]
pub struct InstantiateResponse {
    pub workflow_id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
}

/// 解析模板文件
fn parse_template_file(path: &std::path::Path) -> Result<(String, TemplateDefinition), AppError> {
    let content = fs::read_to_string(path).map_err(|e| AppError::Internal(e.to_string()))?;

    let template: TemplateDefinition =
        serde_yaml::from_str(&content).map_err(|e| AppError::Internal(e.to_string()))?;

    let id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    Ok((id, template))
}

/// 按类别列出模板
pub async fn list_templates_by_category(
    State(_state): State<Arc<AppState>>,
    Path(category): Path<String>,
) -> Result<Json<ListResponse<TemplateSummary>>, AppError> {
    let templates_path = get_templates_path();

    let mut templates = Vec::new();

    if templates_path.exists() {
        for entry in fs::read_dir(&templates_path).map_err(|e| AppError::Internal(e.to_string()))? {
            let entry = entry.map_err(|e| AppError::Internal(e.to_string()))?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                match parse_template_file(&path) {
                    Ok((id, template)) => {
                        if template.category.to_lowercase() == category.to_lowercase() {
                            templates.push(TemplateSummary {
                                id,
                                name: template.name.clone(),
                                description: template.description.clone(),
                                category: template.category.clone(),
                                stage_count: template.stages.len(),
                                agent_count: template.agents.len(),
                            });
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse template {:?}: {}", path, e);
                    }
                }
            }
        }
    }

    // 按名称排序
    templates.sort_by(|a, b| a.name.cmp(&b.name));

    let total = templates.len();

    Ok(Json(ListResponse { items: templates, total }))
}

// ============ 错误类型 ============

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

/// 应用错误类型
#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    BadRequest(String),
    Internal(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AppError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            AppError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::Internal(msg) => {
                tracing::error!("内部错误: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "内部服务器错误".to_string())
            }
        };

        let body = serde_json::json!({
            "error": message
        });

        (status, Json(body)).into_response()
    }
}

impl From<crate::services::workflow_service::WorkflowServiceError> for AppError {
    fn from(err: crate::services::workflow_service::WorkflowServiceError) -> Self {
        match err {
            crate::services::workflow_service::WorkflowServiceError::NotFound(id) => {
                AppError::NotFound(id)
            }
            crate::services::workflow_service::WorkflowServiceError::AlreadyExists(id) => {
                AppError::BadRequest(format!("工作流 {} 已存在", id))
            }
            crate::services::workflow_service::WorkflowServiceError::Internal(msg) => {
                AppError::Internal(msg)
            }
        }
    }
}