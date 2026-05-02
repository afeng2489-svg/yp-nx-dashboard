//! 技能管理路由
//!
//! 提供技能的查询、执行和管理接口。

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

use super::AppState;
use crate::models::skill::{CreateSkillRequest, UpdateSkillRequest};
use crate::response::{ok, ApiErrorResponse, ApiOk};
use crate::services::skill_service::{
    ExecuteSkillRequest, ExecuteSkillResponse, SearchSkillsRequest, SkillDetail, SkillService,
    SkillStats, SkillSummary,
};
use crate::services::workflow_service::{WorkflowService, WorkflowServiceError};

/// 列出所有技能
pub async fn list_skills(State(state): State<Arc<AppState>>) -> ApiOk<Vec<SkillSummary>> {
    let skills = state.skill_service.list_skills();
    ok(skills)
}

/// 按类别获取技能
pub async fn list_by_category(
    State(state): State<Arc<AppState>>,
    Path(category): Path<String>,
) -> Result<ApiOk<Vec<SkillSummary>>, ApiErrorResponse> {
    let skills = state
        .skill_service
        .list_by_category(&category)
        .map_err(|e| ApiErrorResponse::new(StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(ok(skills))
}

/// 获取技能详情
pub async fn get_skill(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<ApiOk<SkillDetail>, ApiErrorResponse> {
    tracing::debug!("[DEBUG ROUTE] get_skill called with id={}", id);
    let detail = state.skill_service.get_skill(&id).map_err(|e| {
        let status = match e {
            crate::services::skill_service::SkillServiceError::SkillNotFound(_) => {
                StatusCode::NOT_FOUND
            }
            _ => StatusCode::BAD_REQUEST,
        };
        ApiErrorResponse::new(status, e.to_string())
    })?;
    tracing::debug!(
        "[DEBUG ROUTE] get_skill returning code_len={}",
        detail.code.as_ref().map(|c| c.len()).unwrap_or(0)
    );
    Ok(ok(detail))
}

/// 搜索技能
pub async fn search_skills(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchSkillsRequest>,
) -> ApiOk<Vec<SkillSummary>> {
    let skills = if let Some(query) = params.query {
        state.skill_service.search_skills(&query)
    } else if let Some(tag) = params.tags.as_ref().and_then(|t| t.first()) {
        state.skill_service.list_by_tag(tag)
    } else if let Some(category) = &params.category {
        state
            .skill_service
            .list_by_category(category)
            .unwrap_or_default()
    } else {
        state.skill_service.list_skills()
    };
    ok(skills)
}

/// 按标签获取技能
pub async fn list_by_tag(
    State(state): State<Arc<AppState>>,
    Path(tag): Path<String>,
) -> ApiOk<Vec<SkillSummary>> {
    let skills = state.skill_service.list_by_tag(&tag);
    ok(skills)
}

/// 获取所有类别
pub async fn list_categories(State(state): State<Arc<AppState>>) -> ApiOk<Vec<String>> {
    let categories = state.skill_service.list_categories();
    ok(categories)
}

/// 获取所有标签
pub async fn list_tags(State(state): State<Arc<AppState>>) -> ApiOk<Vec<String>> {
    let tags = state.skill_service.list_tags();
    ok(tags)
}

/// 获取技能统计
pub async fn get_stats(State(state): State<Arc<AppState>>) -> ApiOk<SkillStats> {
    let stats = state.skill_service.get_stats();
    ok(stats)
}

/// 创建技能
pub async fn create_skill(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateSkillRequest>,
) -> Result<ApiOk<SkillDetail>, ApiErrorResponse> {
    state.skill_service.create_skill(req).map(ok).map_err(|e| {
        let status = match e {
            crate::services::skill_service::SkillServiceError::AlreadyExists(_) => {
                StatusCode::CONFLICT
            }
            crate::services::skill_service::SkillServiceError::ValidationFailed(_) => {
                StatusCode::BAD_REQUEST
            }
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        ApiErrorResponse::new(status, e.to_string())
    })
}

/// 更新技能
pub async fn update_skill(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateSkillRequest>,
) -> Result<ApiOk<SkillDetail>, ApiErrorResponse> {
    state
        .skill_service
        .update_skill(&id, req)
        .map(ok)
        .map_err(|e| {
            let status = match e {
                crate::services::skill_service::SkillServiceError::SkillNotFound(_) => {
                    StatusCode::NOT_FOUND
                }
                crate::services::skill_service::SkillServiceError::ValidationFailed(_) => {
                    StatusCode::BAD_REQUEST
                }
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            ApiErrorResponse::new(status, e.to_string())
        })
}

/// 删除技能
pub async fn delete_skill(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<ApiOk<serde_json::Value>, ApiErrorResponse> {
    state.skill_service.delete_skill(&id).map_err(|e| {
        let status = match e {
            crate::services::skill_service::SkillServiceError::SkillNotFound(_) => {
                StatusCode::NOT_FOUND
            }
            crate::services::skill_service::SkillServiceError::ValidationFailed(_) => {
                StatusCode::FORBIDDEN
            }
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        ApiErrorResponse::new(status, e.to_string())
    })?;
    Ok(ok(serde_json::json!({ "deleted": true })))
}

/// 执行技能
pub async fn execute_skill(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ExecuteSkillRequest>,
) -> Result<ApiOk<ExecuteSkillResponse>, ApiErrorResponse> {
    state
        .skill_service
        .execute_skill(&req.skill_id, req.phase, req.params, req.working_dir)
        .await
        .map(ok)
        .map_err(|e| ApiErrorResponse::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// 从 agents 目录重新加载技能（文件已作为直接来源，此操作仅刷新缓存）
pub async fn import_from_agents(
    State(state): State<Arc<AppState>>,
) -> Result<ApiOk<ImportAgentsResponse>, ApiErrorResponse> {
    state
        .skill_service
        .reload_skills()
        .map(|count| {
            ok(ImportAgentsResponse {
                imported: count,
                message: format!("技能已从文件重新加载，当前共 {} 个技能", count),
            })
        })
        .map_err(|e| ApiErrorResponse::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// 导入响应的结构
#[derive(Debug, serde::Serialize)]
pub struct ImportAgentsResponse {
    pub imported: usize,
    pub message: String,
}

/// 导入技能请求
#[derive(Debug, serde::Deserialize)]
pub struct ImportSkillRequest {
    pub source: String,           // "url" | "file" | "paste"
    pub content: String,          // URL 地址或 .md 文件内容
    pub filename: Option<String>, // 可选文件名，用于推导 skill id
}

/// 导入技能（从 URL、文件内容或粘贴文本）
pub async fn import_skill(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ImportSkillRequest>,
) -> Result<ApiOk<SkillDetail>, ApiErrorResponse> {
    state
        .skill_service
        .import_skill(&req.source, &req.content, req.filename.as_deref())
        .await
        .map(ok)
        .map_err(|e| {
            let status = match e {
                crate::services::skill_service::SkillServiceError::AlreadyExists(_) => {
                    StatusCode::CONFLICT
                }
                crate::services::skill_service::SkillServiceError::ValidationFailed(_) => {
                    StatusCode::BAD_REQUEST
                }
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            ApiErrorResponse::new(status, e.to_string())
        })
}

/// 从技能生成工作流
pub async fn generate_workflow_from_skill(
    State(state): State<Arc<AppState>>,
    Path(skill_id): Path<String>,
) -> Result<ApiOk<GenerateWorkflowResponse>, ApiErrorResponse> {
    let skill = state
        .skill_service
        .get_skill(&skill_id)
        .map_err(|e| ApiErrorResponse::new(StatusCode::NOT_FOUND, e.to_string()))?;

    let (stages, agents) = match skill.category.as_str() {
        "workflow_planning" => {
            let stages = vec![
                crate::routes::templates::Stage {
                    name: "分析".to_string(),
                    agents: vec!["analyst".to_string()],
                    parallel: false,
                },
                crate::routes::templates::Stage {
                    name: "规划".to_string(),
                    agents: vec!["planner".to_string()],
                    parallel: false,
                },
            ];
            let agents = vec![
                crate::routes::templates::Agent {
                    id: "analyst".to_string(),
                    role: "analyst".to_string(),
                    model: "claude-sonnet-4-6".to_string(),
                    prompt: "You are an analyst agent specialized in understanding and breaking down tasks.\n\nYour responsibilities:\n1. Analyze the user's request and identify key components\n2. Determine requirements and constraints\n3. Identify potential risks and dependencies\n4. Provide a clear summary of what needs to be done\n\nBe concise and focus on actionable insights.".to_string(),
                    depends_on: vec![],
                },
                crate::routes::templates::Agent {
                    id: "planner".to_string(),
                    role: "planner".to_string(),
                    model: "claude-sonnet-4-6".to_string(),
                    prompt: "You are a planner agent that creates actionable implementation plans.\n\nYour responsibilities:\n1. Create a clear step-by-step plan based on the analysis\n2. Break down complex tasks into manageable units\n3. Estimate effort and time requirements\n4. Define success criteria for each step\n\nOutput a structured plan that can be easily followed.".to_string(),
                    depends_on: vec!["analyst".to_string()],
                },
            ];
            (stages, agents)
        }
        "development" | "testing" | "review" => {
            let stages = vec![
                crate::routes::templates::Stage {
                    name: "测试".to_string(),
                    agents: vec!["tester".to_string()],
                    parallel: false,
                },
                crate::routes::templates::Stage {
                    name: "实现".to_string(),
                    agents: vec!["developer".to_string()],
                    parallel: false,
                },
                crate::routes::templates::Stage {
                    name: "审查".to_string(),
                    agents: vec!["reviewer".to_string()],
                    parallel: false,
                },
            ];
            let agents = vec![
                crate::routes::templates::Agent {
                    id: "tester".to_string(),
                    role: "tester".to_string(),
                    model: "claude-haiku-4-5".to_string(),
                    prompt: format!("You are a tester agent. \n\nYour task: {}\n\nWrite failing tests first, then let the developer implement the feature.", skill.description),
                    depends_on: vec![],
                },
                crate::routes::templates::Agent {
                    id: "developer".to_string(),
                    role: "developer".to_string(),
                    model: "claude-opus-4-6".to_string(),
                    prompt: format!("You are a developer agent. \n\nYour task: {}\n\nImplement the feature following the tests.", skill.description),
                    depends_on: vec!["tester".to_string()],
                },
                crate::routes::templates::Agent {
                    id: "reviewer".to_string(),
                    role: "reviewer".to_string(),
                    model: "claude-sonnet-4-6".to_string(),
                    prompt: "You are a code reviewer. Review the implementation for quality, style, and best practices.".to_string(),
                    depends_on: vec!["developer".to_string()],
                },
            ];
            (stages, agents)
        }
        _ => {
            let stages = vec![crate::routes::templates::Stage {
                name: "执行".to_string(),
                agents: vec!["executor".to_string()],
                parallel: false,
            }];
            let agents = vec![crate::routes::templates::Agent {
                id: "executor".to_string(),
                role: "executor".to_string(),
                model: "claude-sonnet-4-6".to_string(),
                prompt: format!(
                    "You are an executor agent.\n\nTask: {}\n\nExecute the task and report results.",
                    skill.description
                ),
                depends_on: vec![],
            }];
            (stages, agents)
        }
    };

    let workflow = state
        .workflow_service
        .create_workflow(
            format!("从技能创建: {}", skill.name),
            Some("1.0.0".to_string()),
            Some(format!("由技能 {} 生成的工作流", skill.name)),
            serde_json::json!({
                "stages": stages,
                "agents": agents,
            }),
        )
        .map_err(|e| ApiErrorResponse::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(ok(GenerateWorkflowResponse {
        workflow_id: workflow.id,
        name: workflow.name,
        description: workflow.description,
        created_at: workflow.created_at.to_rfc3339(),
    }))
}

/// 从技能生成工作流的响应
#[derive(Debug, serde::Serialize)]
pub struct GenerateWorkflowResponse {
    pub workflow_id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
}
