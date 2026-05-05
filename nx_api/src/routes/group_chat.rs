//! Group Chat Routes
//!
//! REST API endpoints for multi-agent group discussion.

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::error::ApiError;
use crate::models::group_chat::{
    ConcludeDiscussionRequest, CreateGroupSessionRequest, DiscussionTurnInfo, GetMessagesRequest,
    GroupConclusion, GroupMessage, GroupSession, GroupSessionDetail, NextSpeakerInfo,
    SendMessageRequest, StartDiscussionRequest, UpdateGroupSessionRequest,
};
use crate::routes::AppState;
use crate::services::group_chat_service::GroupChatServiceError;

#[derive(Deserialize)]
pub struct ListSessionsQuery {
    pub team_id: Option<String>,
}

/// Create a new group session
pub async fn create_session(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateGroupSessionRequest>,
) -> Result<Json<GroupSession>, ApiError> {
    let service = &state.group_chat_service;

    let session = service
        .create_session(request)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(session))
}

/// List sessions by team
pub async fn list_sessions(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListSessionsQuery>,
) -> Result<Json<Vec<GroupSession>>, ApiError> {
    let service = &state.group_chat_service;

    let sessions = if let Some(team_id) = &query.team_id {
        service.get_sessions_by_team(team_id).await?
    } else {
        service.get_all_sessions().await?
    };

    Ok(Json(sessions))
}

/// Get session by ID
pub async fn get_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<GroupSessionDetail>, ApiError> {
    let service = &state.group_chat_service;

    let session = service
        .get_session_detail(&id)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(session))
}

/// Update session
pub async fn update_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(request): Json<UpdateGroupSessionRequest>,
) -> Result<Json<GroupSession>, ApiError> {
    let service = &state.group_chat_service;

    let session = service
        .update_session(&id, request)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(session))
}

/// Delete session
pub async fn delete_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<()>, ApiError> {
    let service = &state.group_chat_service;

    service.delete_session(&id).await.map_err(ApiError::from)?;

    Ok(Json(()))
}

/// Start discussion
pub async fn start_discussion(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(request): Json<StartDiscussionRequest>,
) -> Result<Json<DiscussionTurnInfo>, ApiError> {
    let service = &state.group_chat_service;

    let turn_info = service
        .start_discussion(&id, request)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(turn_info))
}

/// Get messages for a session
pub async fn get_messages(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(request): Query<GetMessagesRequest>,
) -> Result<Json<Vec<GroupMessage>>, ApiError> {
    let service = &state.group_chat_service;

    let messages = service
        .get_messages(&id, request)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(messages))
}

/// Send a message
pub async fn send_message(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(request): Json<SendMessageRequest>,
) -> Result<Json<GroupMessage>, ApiError> {
    tracing::info!("[Route] send_message 被调用，session_id: {}", id);
    let service = &state.group_chat_service;

    let message = service
        .send_message(&id, request.clone())
        .await
        .map_err(ApiError::from)?;

    // 检测需求关键词并自动创建 Pipeline
    if let Some(pipeline_service) = &state.pipeline_service {
        if is_requirement_message(&request.content) {
            if let Err(e) = auto_create_pipeline(&state, &id, &request.content).await {
                tracing::warn!("自动创建 Pipeline 失败: {}", e);
            }
        }
    }

    Ok(Json(message))
}

/// Get next speaker
pub async fn get_next_speaker(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Option<NextSpeakerInfo>>, ApiError> {
    let service = &state.group_chat_service;

    let next = service
        .get_next_speaker(&id)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(next.map(|(role_id, role_name)| NextSpeakerInfo {
        role_id,
        role_name,
    })))
}

/// Advance to next speaker
pub async fn advance_speaker(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<()>, ApiError> {
    let service = &state.group_chat_service;

    service.advance_speaker(&id).await.map_err(ApiError::from)?;

    Ok(Json(()))
}

/// Conclude discussion
pub async fn conclude_discussion(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(request): Json<ConcludeDiscussionRequest>,
) -> Result<Json<GroupConclusion>, ApiError> {
    let service = &state.group_chat_service;

    let conclusion = service
        .conclude_discussion(&id, request)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(conclusion))
}

/// Execute all specified roles in parallel (async — returns execution_ids immediately)
///
/// All Claude CLI calls are spawned concurrently, so total wall-clock time ≈ max(individual times)
/// instead of sum(individual times). Each execution can be monitored via
/// `GET /ws/agent-executions/{execution_id}`.
pub async fn execute_round(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<ExecuteRoundRequest>,
) -> Result<Json<Vec<RoundExecutionInfo>>, ApiError> {
    if body.role_ids.is_empty() {
        return Ok(Json(vec![]));
    }

    let mut executions = Vec::new();

    for role_id in &body.role_ids {
        let execution_id = uuid::Uuid::new_v4().to_string();
        let cancel_token = tokio_util::sync::CancellationToken::new();
        state
            .agent_execution_manager
            .register_cancel_token(&execution_id, cancel_token.clone());

        let _ = state.agent_execution_manager.event_sender().send(
            crate::ws::agent_execution::AgentExecutionEvent::Started {
                execution_id: execution_id.clone(),
                agent_role: role_id.clone(),
                task_summary: format!("Parallel round: {}", role_id),
                role_id: Some(role_id.clone()),
                session_id: None,
            },
        );

        let service = state.group_chat_service.clone();
        let exec_id = execution_id.clone();
        let role = role_id.clone();
        let session_id = id.clone();
        let tx = state.agent_execution_manager.event_sender();
        let manager = state.agent_execution_manager.clone();

        // Spawn each bot concurrently — all run at the same time
        tokio::spawn(async move {
            let start = std::time::Instant::now();
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
            interval.tick().await;

            let task_future = service.execute_role_turn(&session_id, &role);
            tokio::pin!(task_future);

            let result = loop {
                tokio::select! {
                    res = &mut task_future => { break res; }
                    _ = interval.tick() => {
                        let _ = tx.send(crate::ws::agent_execution::AgentExecutionEvent::Thinking {
                            execution_id: exec_id.clone(),
                            elapsed_secs: start.elapsed().as_secs(),
                        });
                    }
                    _ = cancel_token.cancelled() => {
                        let _ = tx.send(crate::ws::agent_execution::AgentExecutionEvent::Cancelled {
                            execution_id: exec_id.clone(),
                        });
                        manager.remove_execution(&exec_id);
                        return;
                    }
                }
            };

            match result {
                Ok(message) => {
                    let result_str = serde_json::to_string(&message).unwrap_or_default();
                    let _ = tx.send(crate::ws::agent_execution::AgentExecutionEvent::Completed {
                        execution_id: exec_id.clone(),
                        result: result_str,
                        duration_ms: start.elapsed().as_millis() as u64,
                    });
                }
                Err(e) => {
                    let _ = tx.send(crate::ws::agent_execution::AgentExecutionEvent::Failed {
                        execution_id: exec_id.clone(),
                        error: e.to_string(),
                    });
                }
            }
            manager.remove_execution(&exec_id);
        });

        executions.push(RoundExecutionInfo {
            role_id: role_id.clone(),
            execution_id,
        });
    }

    Ok(Json(executions))
}

#[derive(serde::Deserialize)]
pub struct ExecuteRoundRequest {
    pub role_ids: Vec<String>,
}

#[derive(serde::Serialize)]
pub struct RoundExecutionInfo {
    pub role_id: String,
    pub execution_id: String,
}

/// Execute a role's turn (async — returns execution_id immediately)
pub async fn execute_role_turn(
    State(state): State<Arc<AppState>>,
    Path((id, role_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let execution_id = uuid::Uuid::new_v4().to_string();
    let event_tx = state.agent_execution_manager.event_sender();
    let cancel_token = tokio_util::sync::CancellationToken::new();
    state
        .agent_execution_manager
        .register_cancel_token(&execution_id, cancel_token.clone());

    // 发送 Started 事件
    let _ = event_tx.send(crate::ws::agent_execution::AgentExecutionEvent::Started {
        execution_id: execution_id.clone(),
        agent_role: role_id.clone(),
        task_summary: format!("Role turn: {}", role_id),
        role_id: Some(role_id.clone()),
        session_id: None,
    });

    let service = state.group_chat_service.clone();
    let exec_id = execution_id.clone();
    let tx = event_tx.clone();
    let manager = state.agent_execution_manager.clone();

    tokio::spawn(async move {
        let start = std::time::Instant::now();
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
        interval.tick().await;

        let task_future = service.execute_role_turn(&id, &role_id);
        tokio::pin!(task_future);

        let result = loop {
            tokio::select! {
                res = &mut task_future => { break res; }
                _ = interval.tick() => {
                    let _ = tx.send(crate::ws::agent_execution::AgentExecutionEvent::Thinking {
                        execution_id: exec_id.clone(),
                        elapsed_secs: start.elapsed().as_secs(),
                    });
                }
                _ = cancel_token.cancelled() => {
                    let _ = tx.send(crate::ws::agent_execution::AgentExecutionEvent::Cancelled {
                        execution_id: exec_id.clone(),
                    });
                    manager.remove_execution(&exec_id);
                    return;
                }
            }
        };

        match result {
            Ok(message) => {
                let result_str = serde_json::to_string(&message).unwrap_or_default();
                let _ = tx.send(crate::ws::agent_execution::AgentExecutionEvent::Completed {
                    execution_id: exec_id.clone(),
                    result: result_str,
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
            Err(e) => {
                let _ = tx.send(crate::ws::agent_execution::AgentExecutionEvent::Failed {
                    execution_id: exec_id.clone(),
                    error: e.to_string(),
                });
            }
        }
        manager.remove_execution(&exec_id);
    });

    // 立即返回 execution_id
    Ok(Json(serde_json::json!({
        "execution_id": execution_id,
        "status": "processing"
    })))
}

/// POST /api/v1/group-sessions/:id/auto-pipeline
pub async fn auto_pipeline(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<AutoPipelineRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pipeline = auto_create_pipeline(&state, &id, &body.requirement)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(pipeline))
}

#[derive(serde::Deserialize)]
pub struct AutoPipelineRequest {
    pub requirement: String,
}

fn is_requirement_message(content: &str) -> bool {
    let keywords = [
        "做一个",
        "开发",
        "实现",
        "需要",
        "构建",
        "创建",
        "帮我做",
        "帮我开发",
    ];
    keywords.iter().any(|kw| content.contains(kw))
}

async fn auto_create_pipeline(
    state: &Arc<AppState>,
    session_id: &str,
    requirement: &str,
) -> Result<serde_json::Value, String> {
    let pipeline_service = state
        .pipeline_service
        .as_ref()
        .ok_or("Pipeline service not available")?;

    let session = state
        .group_chat_service
        .get_session_detail(session_id)
        .await
        .map_err(|e| e.to_string())?;

    let pipeline = pipeline_service
        .create_pipeline(&session_id, &session.session.team_id)
        .map_err(|e| e.to_string())?;

    // 从团队真实角色构建 role_id 映射
    let role_map = build_role_map(state, &session.session.team_id);
    let steps = build_steps_from_requirement(&pipeline.id, requirement, &role_map);
    pipeline_service
        .add_steps(&steps)
        .map_err(|e| e.to_string())?;

    pipeline_service
        .start(&pipeline.id)
        .map_err(|e| e.to_string())?;

    tracing::info!(
        "自动创建 Pipeline {} for session {}",
        pipeline.id,
        session_id
    );

    Ok(serde_json::json!({
        "pipeline_id": pipeline.id,
        "status": "running",
        "steps": steps.len(),
        "requirement": requirement
    }))
}

fn build_steps_from_requirement(
    pipeline_id: &str,
    requirement: &str,
    role_map: &std::collections::HashMap<&str, String>,
) -> Vec<crate::models::pipeline::PipelineStep> {
    use crate::models::pipeline::{PipelinePhase, PipelineStep, StepStatus};
    use chrono::Utc;
    use uuid::Uuid;

    let resolve = |key: &str| {
        role_map
            .get(key)
            .cloned()
            .unwrap_or_else(|| key.to_string())
    };

    let phases = [
        (PipelinePhase::RequirementsAnalysis, format!("需求：{}\n\n请分析以上需求，输出详细的需求文档，包括功能列表、用户故事、验收标准。", requirement), resolve("analyst")),
        (PipelinePhase::ArchitectureDesign, format!("需求：{}\n\n请设计系统架构，包括技术选型、模块划分、数据库设计、API 接口设计。", requirement), resolve("architect")),
        (PipelinePhase::ProjectInit, format!("需求：{}\n\n请初始化项目结构，创建目录、配置文件、依赖管理文件。", requirement), resolve("architect")),
        (PipelinePhase::BackendDev, format!("需求：{}\n\n请实现后端 API，包括数据模型、业务逻辑、接口实现。", requirement), resolve("backend")),
        (PipelinePhase::FrontendDev, format!("需求：{}\n\n请实现前端界面，包括页面组件、状态管理、API 调用。", requirement), resolve("frontend")),
        (PipelinePhase::ApiIntegration, format!("需求：{}\n\n请完成前后端联调，确保接口对接正确，处理跨域、认证等问题。", requirement), resolve("fullstack")),
        (PipelinePhase::Testing, format!("需求：{}\n\n请编写并运行测试，包括单元测试、集成测试，确保覆盖率达到 80%。", requirement), resolve("tester")),
        (PipelinePhase::Documentation, format!("需求：{}\n\n请编写项目文档，包括 README、API 文档、部署说明。", requirement), resolve("writer")),
        (PipelinePhase::Packaging, format!("需求：{}\n\n请完成打包部署配置，包括 Dockerfile、CI/CD 配置、环境变量说明。", requirement), resolve("devops")),
    ];

    let now = Utc::now();
    phases
        .into_iter()
        .map(|(phase, instruction, role_id)| PipelineStep {
            id: Uuid::new_v4().to_string(),
            pipeline_id: pipeline_id.to_string(),
            task_id: format!("task-{}", Uuid::new_v4()),
            phase,
            role_id,
            instruction,
            depends_on: vec![],
            status: StepStatus::Pending,
            output: None,
            retry_count: 0,
            max_retries: 3,
            created_at: now,
            started_at: None,
            completed_at: None,
        })
        .collect()
}

/// 从团队真实角色构建关键词 → role_id 映射
/// 按 trigger_keywords 匹配，找不到时 fallback 到第一个角色
fn build_role_map<'a>(
    state: &Arc<AppState>,
    team_id: &str,
) -> std::collections::HashMap<&'a str, String> {
    let mut map = std::collections::HashMap::new();
    let roles = state
        .teams_state
        .team_service
        .list_roles(team_id)
        .unwrap_or_default();
    if roles.is_empty() {
        return map;
    }

    let role_keys = [
        ("analyst", &["分析", "需求", "pm", "产品"][..]),
        ("architect", &["架构", "设计", "architect"][..]),
        ("backend", &["后端", "backend", "api", "服务"][..]),
        ("frontend", &["前端", "frontend", "ui", "界面"][..]),
        ("fullstack", &["全栈", "fullstack", "联调"][..]),
        ("tester", &["测试", "test", "qa"][..]),
        ("writer", &["文档", "doc", "writer"][..]),
        ("devops", &["运维", "devops", "部署", "deploy"][..]),
    ];

    for (key, keywords) in &role_keys {
        let matched = roles.iter().find(|r| {
            let name_lower = r.name.to_lowercase();
            let kws_lower: Vec<String> = r
                .trigger_keywords
                .iter()
                .map(|k| k.to_lowercase())
                .collect();
            keywords
                .iter()
                .any(|kw| name_lower.contains(kw) || kws_lower.iter().any(|k| k.contains(kw)))
        });
        // fallback: 找不到匹配角色时用第一个角色的 id
        let role_id = matched.unwrap_or(&roles[0]).id.clone();
        map.insert(key, role_id);
    }
    map
}

impl From<GroupChatServiceError> for ApiError {
    fn from(err: GroupChatServiceError) -> Self {
        match err {
            GroupChatServiceError::SessionNotFound(id) => {
                ApiError::NotFound(format!("Group session not found: {}", id))
            }
            GroupChatServiceError::SessionNotActive(id) => {
                ApiError::BadRequest(format!("Group session not active: {}", id))
            }
            GroupChatServiceError::RoleNotFound(id) => {
                ApiError::NotFound(format!("Role not found: {}", id))
            }
            GroupChatServiceError::TeamNotFound(id) => {
                ApiError::NotFound(format!("Team not found: {}", id))
            }
            GroupChatServiceError::MaxTurnsReached => {
                ApiError::BadRequest("Maximum turns reached".to_string())
            }
            GroupChatServiceError::ClaudeCli(msg) => {
                ApiError::Internal(format!("Claude CLI error: {}", msg))
            }
            _ => ApiError::Internal(err.to_string()),
        }
    }
}
