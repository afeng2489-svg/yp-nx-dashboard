//! Team routes
//!
//! REST API endpoints for team management.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

use nx_memory::{SearchRequest, Transcript, MemoryChunk, MessageRole};

use crate::models::team::{
    AssignSkillRequest, CreateRoleRequest, CreateTeamRequest, ExecuteRoleTaskRequest,
    ExecuteRoleTaskResponse, ExecuteTeamTaskRequest, SkillPriority, Team, TeamMessage, TeamRole,
    TelegramBotConfig, TelegramConfigRequest, UpdateRoleRequest, UpdateTeamRequest,
};
use crate::routes::AppState;
use crate::services::team_service::{RoleWithSkills, TeamWithRoles};

/// API response type
type ApiResponse<T> = Result<Json<T>, AppError>;

/// Application error
#[derive(Debug)]
pub struct AppError {
    status: StatusCode,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        (self.status, Json(serde_json::json!({ "error": self.message }))).into_response()
    }
}

impl From<crate::services::team_service::TeamServiceError> for AppError {
    fn from(err: crate::services::team_service::TeamServiceError) -> Self {
        match err {
            crate::services::team_service::TeamServiceError::TeamNotFound(id) => {
                AppError { status: StatusCode::NOT_FOUND, message: format!("Team not found: {}", id) }
            }
            crate::services::team_service::TeamServiceError::RoleNotFound(id) => {
                AppError { status: StatusCode::NOT_FOUND, message: format!("Role not found: {}", id) }
            }
            crate::services::team_service::TeamServiceError::TelegramConfigNotFound(role_id) => {
                AppError { status: StatusCode::NOT_FOUND, message: format!("Telegram config not found for role: {}", role_id) }
            }
            _ => AppError { status: StatusCode::INTERNAL_SERVER_ERROR, message: err.to_string() },
        }
    }
}

impl From<crate::services::agent_team_service::AgentTeamServiceError> for AppError {
    fn from(err: crate::services::agent_team_service::AgentTeamServiceError) -> Self {
        AppError { status: StatusCode::INTERNAL_SERVER_ERROR, message: err.to_string() }
    }
}

// Team endpoints

/// List all teams
pub async fn list_teams(
    State(state): State<Arc<AppState>>,
) -> ApiResponse<Vec<Team>> {
    let teams = state.teams_state.team_service.list_teams()?;
    Ok(Json(teams))
}

/// Create a new team
pub async fn create_team(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateTeamRequest>,
) -> ApiResponse<Team> {
    let team = state.teams_state.team_service.create_team(request)?;
    Ok(Json(team))
}

/// Get a team by ID
pub async fn get_team(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<String>,
) -> ApiResponse<TeamWithRoles> {
    let team = state.teams_state.team_service.get_team_with_roles(&team_id)?;
    Ok(Json(team))
}

/// Update a team
pub async fn update_team(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<String>,
    Json(request): Json<UpdateTeamRequest>,
) -> ApiResponse<Team> {
    let team = state.teams_state.team_service.update_team(&team_id, request)?;
    Ok(Json(team))
}

/// Delete a team
pub async fn delete_team(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<String>,
) -> ApiResponse<bool> {
    let deleted = state.teams_state.team_service.delete_team(&team_id)?;
    Ok(Json(deleted))
}

// Role endpoints

/// List roles in a team
pub async fn list_roles(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<String>,
) -> ApiResponse<Vec<TeamRole>> {
    let roles = state.teams_state.team_service.list_roles(&team_id)?;
    Ok(Json(roles))
}

/// Remove a role from a team (only removes the assignment, doesn't delete the role)
pub async fn remove_role_from_team(
    State(state): State<Arc<AppState>>,
    Path((team_id, role_id)): Path<(String, String)>,
) -> ApiResponse<bool> {
    let removed = state.teams_state.team_service.remove_role_from_team(&team_id, &role_id)?;
    Ok(Json(removed))
}

/// List all roles across all teams
pub async fn list_all_roles(
    State(state): State<Arc<AppState>>,
) -> ApiResponse<Vec<TeamRole>> {
    let roles = state.teams_state.team_service.list_all_roles()?;
    Ok(Json(roles))
}

/// Create a role in a team
pub async fn create_role(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<String>,
    Json(request): Json<CreateRoleRequest>,
) -> ApiResponse<TeamRole> {
    let role = state.teams_state.team_service.create_role(&team_id, request)?;
    Ok(Json(role))
}

/// Get a role by ID
pub async fn get_role(
    State(state): State<Arc<AppState>>,
    Path(role_id): Path<String>,
) -> ApiResponse<RoleWithSkills> {
    let role = state.teams_state.team_service.get_role_with_skills(&role_id)?;
    Ok(Json(role))
}

/// Update a role
pub async fn update_role(
    State(state): State<Arc<AppState>>,
    Path(role_id): Path<String>,
    Json(request): Json<UpdateRoleRequest>,
) -> ApiResponse<TeamRole> {
    let role = state.teams_state.team_service.update_role(&role_id, request)?;
    Ok(Json(role))
}

/// Delete a role
pub async fn delete_role(
    State(state): State<Arc<AppState>>,
    Path(role_id): Path<String>,
) -> ApiResponse<bool> {
    let deleted = state.teams_state.team_service.delete_role(&role_id)?;
    Ok(Json(deleted))
}

/// Assign a role to a team (change its team_id)
pub async fn assign_role_to_team(
    State(state): State<Arc<AppState>>,
    Path(role_id): Path<String>,
    Json(request): Json<crate::models::team::AssignRoleToTeamRequest>,
) -> ApiResponse<TeamRole> {
    let role = state.teams_state.team_service.assign_role_to_team(&role_id, &request.team_id)?;
    Ok(Json(role))
}

// Skill endpoints

/// Assign a skill to a role
pub async fn assign_skill(
    State(state): State<Arc<AppState>>,
    Path((role_id, skill_id)): Path<(String, String)>,
    Json(request): Json<AssignSkillRequest>,
) -> ApiResponse<crate::models::team::RoleSkill> {
    // skill_id comes from path, but we still parse request for priority
    let priority = request.priority.or(Some(SkillPriority::Medium));
    let skill = state.teams_state.team_service.assign_skill(&role_id, &skill_id, priority)?;
    Ok(Json(skill))
}

/// Remove a skill from a role
pub async fn remove_skill(
    State(state): State<Arc<AppState>>,
    Path((role_id, skill_id)): Path<(String, String)>,
) -> ApiResponse<bool> {
    let removed = state.teams_state.team_service.remove_skill(&role_id, &skill_id)?;
    Ok(Json(removed))
}

/// Get skills assigned to a role
pub async fn get_role_skills(
    State(state): State<Arc<AppState>>,
    Path(role_id): Path<String>,
) -> ApiResponse<Vec<crate::models::team::RoleSkill>> {
    let skills = state.teams_state.team_service.get_role_skills(&role_id)?;
    Ok(Json(skills))
}

// Message endpoints

/// Get messages for a team
pub async fn get_team_messages(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<String>,
    Query(params): Query<MessageQueryParams>,
) -> ApiResponse<Vec<TeamMessage>> {
    let messages = state.teams_state.team_service.get_team_messages(&team_id, params.limit)?;
    Ok(Json(messages))
}

#[derive(Debug, serde::Deserialize)]
pub struct MessageQueryParams {
    pub limit: Option<usize>,
}

// Execution endpoint

/// Execute a task across a team (with memory integration)
pub async fn execute_team_task(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<String>,
    Json(request): Json<ExecuteTeamTaskRequest>,
) -> ApiResponse<crate::models::team::ExecuteTeamTaskResponse> {
    tracing::info!("[Route] execute_team_task 被调用，team_id: {}", team_id);

    // 1. 尝试匹配角色触发关键词
    if let Some(matched_role_id) = find_role_by_trigger(&state, &team_id, &request.task).await {
        tracing::info!("[Route] 匹配到角色: {}，使用 execute_role_task", matched_role_id);

        // 构建 ExecuteRoleTaskRequest
        let role_task_request = crate::models::team::ExecuteRoleTaskRequest {
            role_id: matched_role_id.clone(),
            task: request.task.clone(),
            context: request.context.clone(),
        };

        // 调用 execute_role_task
        let role_response = execute_role_task_with_context(
            State(state.clone()),
            Json(role_task_request),
        ).await?;

        // 返回 ExecuteTeamTaskResponse 格式
        return Ok(Json(crate::models::team::ExecuteTeamTaskResponse {
            success: role_response.success,
            team_id: team_id.clone(),
            messages: vec![],
            final_output: role_response.response.clone(),
            error: role_response.error.clone(),
        }));
    }

    // 2. 没有匹配到角色，执行原有的团队任务逻辑
    // 搜索相关记忆
    let memory_context = search_and_build_context(&state.memory_state, &team_id, &request.task).await;

    // 创建增强的请求（包含记忆上下文）
    let enhanced_request = ExecuteTeamTaskRequest {
        team_id: request.team_id.clone(),
        task: request.task.clone(),
        context: {
            let mut ctx = request.context.clone();
            if !memory_context.is_empty() {
                ctx.insert("memory_context".to_string(), memory_context);
            }
            ctx
        },
    };

    // Execute task
    let response = state.teams_state.agent_team_service.execute_team_task(enhanced_request).await?;

    // Store conversation to memory (async, non-blocking)
    let memory_state = state.memory_state.clone();
    let team_id_clone = team_id.clone();
    let user_message = request.task.clone();
    let assistant_reply = response.final_output.clone();

    tokio::spawn(async move {
        // Store user message
        if let Err(e) = store_to_memory(&memory_state, &team_id_clone, "user", &user_message).await {
            tracing::warn!("[Memory] Failed to store user message: {}", e);
        }
        // Store assistant reply
        if !assistant_reply.is_empty() {
            if let Err(e) = store_to_memory(&memory_state, &team_id_clone, "assistant", &assistant_reply).await {
                tracing::warn!("[Memory] Failed to store assistant reply: {}", e);
            }
        }
    });

    Ok(Json(response))
}

/// 根据触发关键词查找匹配的角色
async fn find_role_by_trigger(
    state: &Arc<AppState>,
    team_id: &str,
    task: &str,
) -> Option<String> {
    // 获取团队的所有角色
    let roles = state.teams_state.team_service.list_roles(team_id).ok()?;

    // 任务文本转小写用于匹配
    let task_lower = task.to_lowercase();

    for role in roles {
        for keyword in &role.trigger_keywords {
            if task_lower.contains(&keyword.to_lowercase()) {
                tracing::info!("[Route] 角色 '{}' 匹配关键词 '{}'", role.name, keyword);
                return Some(role.id);
            }
        }
    }

    None
}

/// execute_role_task 的内部版本，支持传入已构建的请求
async fn execute_role_task_with_context(
    State(state): State<Arc<AppState>>,
    Json(request): Json<crate::models::team::ExecuteRoleTaskRequest>,
) -> ApiResponse<crate::models::team::ExecuteRoleTaskResponse> {
    println!("[DEBUG] execute_role_task_with_context CALLED, role_id: {}", request.role_id);
    tracing::info!("[Route] execute_role_task_with_context 被调用，role_id: {}", request.role_id);

    // 1. 获取 role 信息以确定 team_id
    println!("[DEBUG-1] Getting role_with_skills for role_id: {}", request.role_id);
    tracing::info!("[Route-1] Getting role_with_skills...");
    let role_with_skills = state.teams_state.team_service.get_role_with_skills(&request.role_id)
        .map_err(|e| AppError::from(crate::services::team_service::TeamServiceError::RoleNotFound(e.to_string())))?;
    println!("[DEBUG-2] Got role_with_skills, role name: {}", role_with_skills.role.name);
    tracing::info!("[Route-2] Got role_with_skills");

    let role_team_id = role_with_skills.role.team_id.clone().unwrap_or_default();
    tracing::info!("[Route] role_team_id: {}", role_team_id);

    // 2. 搜索相关记忆
    println!("[DEBUG-3] Calling search_and_build_context...");
    tracing::info!("[Route-3] Calling search_and_build_context...");
    let memory_context = search_and_build_context(&state.memory_state, &role_team_id, &request.task).await;
    println!("[DEBUG-4] search_and_build_context returned, memory_context len: {}", memory_context.len());
    tracing::info!("[Memory] role_id: {}, task: {}, memory_context length: {}", request.role_id, request.task, memory_context.len());
    tracing::debug!("[Memory] memory_context content (first 500 chars): {}", &memory_context[..memory_context.len().min(500)]);

    // 3. 创建增强的请求（包含记忆上下文）
    let enhanced_request = crate::models::team::ExecuteRoleTaskRequest {
        role_id: request.role_id.clone(),
        task: request.task.clone(),
        context: {
            let mut ctx = request.context.clone();
            if !memory_context.is_empty() {
                ctx.insert("memory_context".to_string(), memory_context);
            }
            ctx
        },
    };

    // 4. Execute task
    println!("[DEBUG-5] About to call agent_team_service.execute_role_task... enhanced_request.role_id={}", enhanced_request.role_id);
    tracing::info!("[Route-5] Calling agent_team_service.execute_role_task...");
    let response = state
        .teams_state
        .agent_team_service
        .execute_role_task(enhanced_request)
        .await?;
    println!("[DEBUG-6] agent_team_service.execute_role_task returned");
    tracing::info!("[Route-6] agent_team_service.execute_role_task returned");

    // 5. Store conversation to memory (async, non-blocking)
    let memory_state = state.memory_state.clone();
    let team_id_clone = role_team_id.clone();
    let user_message = request.task.clone();
    let assistant_reply = response.response.clone();

    tokio::spawn(async move {
        // Store user message
        if let Err(e) = store_to_memory(&memory_state, &team_id_clone, "user", &user_message).await {
            tracing::warn!("[Memory] Failed to store user message: {}", e);
        }
        // Store assistant reply (现在 response 是真实的 AI 回复)
        if !assistant_reply.is_empty() {
            if let Err(e) = store_to_memory(&memory_state, &team_id_clone, "assistant", &assistant_reply).await {
                tracing::warn!("[Memory] Failed to store assistant reply: {}", e);
            }
        }
    });

    Ok(Json(response))
}

/// 搜索记忆并构建上下文字符串
async fn search_and_build_context(
    memory_state: &Arc<crate::routes::memory::MemoryState>,
    team_id: &str,
    query: &str,
) -> String {
    // 如果查询太短（< 2个字符），跳过记忆搜索
    if query.trim().len() < 2 {
        return String::new();
    }

    // 确保索引已初始化
    if memory_state.search.get_index_stats(team_id).is_none() {
        if let Err(e) = memory_state.search.init_team_index(team_id) {
            tracing::warn!("[Memory] 初始化索引失败: {}", e);
            return String::new();
        }
    }

    // 搜索相关记忆
    let search_request = SearchRequest {
        team_id: Some(team_id.to_string()),
        query: query.to_string(),
        top_k: Some(5),
        vector_weight: None,
        keyword_weight: None,
        session_id: None,
    };

    match memory_state.search.search(&search_request) {
        Ok(results) if !results.results.is_empty() => {
            let mut context = String::from("\n\n## 相关历史记忆 (Relevant Memory)\n");
            context.push_str("以下是你之前与用户对话的相关记忆：\n\n");
            for (i, result) in results.results.iter().enumerate() {
                context.push_str(&format!("{}. {}\n", i + 1, result.content));
            }
            context.push_str("\n请结合以上记忆来回答用户的问题。\n");
            tracing::info!("[Memory] 找到 {} 条相关记忆", results.results.len());
            context
        }
        Ok(_) => {
            tracing::debug!("[Memory] 未找到相关记忆");
            String::new()
        }
        Err(e) => {
            tracing::warn!("[Memory] 搜索失败: {}", e);
            String::new()
        }
    }
}

/// 存储消息到记忆
async fn store_to_memory(
    memory_state: &Arc<crate::routes::memory::MemoryState>,
    team_id: &str,
    role: &str,
    content: &str,
) -> Result<(), String> {
    let transcript = Transcript::new(
        team_id,
        "system", // user_id - 可以改进
        if role == "user" { MessageRole::User } else { MessageRole::Assistant },
        content,
    );

    memory_state.store.store_transcript(&transcript)
        .map_err(|e| e.to_string())?;

    let chunk = MemoryChunk::from_transcript(&transcript, content.to_string(), 0);
    memory_state.store.store_chunk(&chunk)
        .map_err(|e| e.to_string())?;

    // 索引
    let metadata = serde_json::json!({});
    memory_state.search.index_chunk(team_id, &chunk.id, &chunk.content, metadata)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Execute a task for a single role (with its assigned skills)
pub async fn execute_role_task(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ExecuteRoleTaskRequest>,
) -> ApiResponse<ExecuteRoleTaskResponse> {
    tracing::info!("[Route] execute_role_task 被调用，role_id: {}", request.role_id);

    // 1. 获取 role 信息以确定 team_id
    let role_with_skills = state.teams_state.team_service.get_role_with_skills(&request.role_id)
        .map_err(|e| AppError::from(crate::services::team_service::TeamServiceError::RoleNotFound(e.to_string())))?;
    let team_id = role_with_skills.role.team_id.clone().unwrap_or_default();

    // 2. 搜索相关记忆
    let memory_context = search_and_build_context(&state.memory_state, &team_id, &request.task).await;

    // 3. 创建增强的请求（包含记忆上下文）
    let enhanced_request = ExecuteRoleTaskRequest {
        role_id: request.role_id.clone(),
        task: request.task.clone(),
        context: {
            let mut ctx = request.context.clone();
            if !memory_context.is_empty() {
                ctx.insert("memory_context".to_string(), memory_context);
            }
            ctx
        },
    };

    // 4. Execute task
    let response = state
        .teams_state
        .agent_team_service
        .execute_role_task(enhanced_request)
        .await?;

    // 5. Store conversation to memory (async, non-blocking)
    let memory_state = state.memory_state.clone();
    let team_id_clone = team_id.clone();
    let user_message = request.task.clone();
    let assistant_reply = response.response.clone();

    tokio::spawn(async move {
        // Store user message
        if let Err(e) = store_to_memory(&memory_state, &team_id_clone, "user", &user_message).await {
            tracing::warn!("[Memory] Failed to store user message: {}", e);
        }
        // Store assistant reply
        if !assistant_reply.is_empty() && assistant_reply != "Message received, processing in background..." {
            if let Err(e) = store_to_memory(&memory_state, &team_id_clone, "assistant", &assistant_reply).await {
                tracing::warn!("[Memory] Failed to store assistant reply: {}", e);
            }
        }
    });

    Ok(Json(response))
}

// Telegram endpoints

/// Configure Telegram for a role
pub async fn configure_telegram(
    State(state): State<Arc<AppState>>,
    Path(role_id): Path<String>,
    Json(request): Json<TelegramConfigRequest>,
) -> ApiResponse<TelegramBotConfig> {
    let config = state
        .teams_state
        .team_service
        .configure_telegram(
            &role_id,
            request.bot_token,
            request.chat_id,
            request.notifications_enabled,
            request.conversation_enabled,
        )?;
    Ok(Json(config))
}

/// Get Telegram configuration for a role
pub async fn get_telegram_config(
    State(state): State<Arc<AppState>>,
    Path(role_id): Path<String>,
) -> ApiResponse<TelegramBotConfig> {
    let config = state.teams_state.team_service.get_telegram_config(&role_id)?;
    Ok(Json(config))
}

/// Enable or disable Telegram for a role
pub async fn enable_telegram(
    State(state): State<Arc<AppState>>,
    Path((role_id, enabled)): Path<(String, bool)>,
) -> ApiResponse<TelegramBotConfig> {
    let config = state.teams_state.team_service.enable_telegram(&role_id, enabled)?;

    // Start or stop polling based on enabled state
    let telegram_service = &state.teams_state.telegram_service;
    if enabled {
        telegram_service.start_polling(role_id.clone(), config.bot_token.clone());
    } else {
        telegram_service.stop_polling(&role_id);
    }

    Ok(Json(config))
}

/// Delete Telegram configuration for a role
pub async fn delete_telegram_config(
    State(state): State<Arc<AppState>>,
    Path(role_id): Path<String>,
) -> ApiResponse<bool> {
    // Stop polling first
    state.teams_state.telegram_service.stop_polling(&role_id);
    let deleted = state.teams_state.team_service.delete_telegram_config(&role_id)?;
    Ok(Json(deleted))
}

// Team-level Telegram endpoints (delegates to first role's Telegram config)

/// Get Telegram configuration for a team (delegates to first role)
pub async fn get_team_telegram_config(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<String>,
) -> ApiResponse<crate::models::team::TelegramBotConfig> {
    // Get the first role of the team
    let roles = state.teams_state.team_service.list_roles(&team_id)
        .map_err(|e| AppError::from(e))?;

    let first_role = roles.into_iter().next()
        .ok_or_else(|| AppError { status: StatusCode::NOT_FOUND, message: format!("No roles found for team {}", team_id) })?;

    let config = state.teams_state.team_service.get_telegram_config(&first_role.id)
        .map_err(|e| AppError::from(e))?;
    Ok(Json(config))
}

/// Configure Telegram for a team (delegates to first role)
pub async fn configure_team_telegram(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<String>,
    Json(request): Json<crate::models::team::TelegramConfigRequest>,
) -> ApiResponse<crate::models::team::TelegramBotConfig> {
    // Get the first role of the team
    let roles = state.teams_state.team_service.list_roles(&team_id)
        .map_err(|e| AppError::from(e))?;

    let first_role = roles.into_iter().next()
        .ok_or_else(|| AppError { status: StatusCode::NOT_FOUND, message: format!("No roles found for team {}", team_id) })?;

    let config = state
        .teams_state
        .team_service
        .configure_telegram(
            &first_role.id,
            request.bot_token,
            request.chat_id,
            request.notifications_enabled,
            request.conversation_enabled,
        )
        .map_err(|e| AppError::from(e))?;
    Ok(Json(config))
}

/// Enable or disable Telegram for a team (delegates to first role)
pub async fn enable_team_telegram(
    State(state): State<Arc<AppState>>,
    Path((team_id, enabled)): Path<(String, bool)>,
) -> ApiResponse<crate::models::team::TelegramBotConfig> {
    // Get the first role of the team
    let roles = state.teams_state.team_service.list_roles(&team_id)
        .map_err(|e| AppError::from(e))?;

    let first_role = roles.into_iter().next()
        .ok_or_else(|| AppError { status: StatusCode::NOT_FOUND, message: format!("No roles found for team {}", team_id) })?;

    let config = state.teams_state.team_service.enable_telegram(&first_role.id, enabled)
        .map_err(|e| AppError::from(e))?;

    // Start or stop polling based on enabled state
    let telegram_service = &state.teams_state.telegram_service;
    if enabled {
        telegram_service.start_polling(first_role.id.clone(), config.bot_token.clone());
    } else {
        telegram_service.stop_polling(&first_role.id);
    }

    Ok(Json(config))
}

/// Send a test message via Telegram
pub async fn send_telegram_message(
    State(state): State<Arc<AppState>>,
    Path(role_id): Path<String>,
    Json(request): Json<crate::models::team::TelegramSendMessageRequest>,
) -> Result<StatusCode, AppError> {
    let config = state.teams_state.team_service.get_telegram_config(&role_id)
        .map_err(|e| AppError::from(e))?;

    state
        .teams_state
        .telegram_service
        .send_message(&config.bot_token, &request.chat_id, &request.text)
        .await
        .map_err(|e| AppError { status: StatusCode::BAD_REQUEST, message: e.to_string() })?;

    Ok(StatusCode::OK)
}
