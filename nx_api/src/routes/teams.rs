//! Team routes
//!
//! REST API endpoints for team management.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use nx_memory::{MemoryChunk, MessageRole, SearchRequest, Transcript};

use crate::models::team::{
    AssignSkillRequest, CreateRoleRequest, CreateTeamRequest, ExecuteRoleTaskRequest,
    ExecuteRoleTaskResponse, ExecuteTeamTaskRequest, SkillPriority, Team, TeamMessage, TeamRole,
    TelegramBotConfig, TelegramConfigRequest, UpdateRoleRequest, UpdateTeamRequest,
};

/// Bot status for a single team member
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct MemberBotStatus {
    pub role_id: String,
    pub role_name: String,
    pub bot_config: Option<TelegramBotConfig>,
    pub is_polling: bool,
}

/// Configure a single member's bot (used in batch request)
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct MemberBotConfigItem {
    pub role_id: String,
    pub bot_token: String,
    pub chat_id: Option<String>,
    pub notifications_enabled: Option<bool>,
    pub conversation_enabled: Option<bool>,
}
use crate::routes::AppState;
use crate::services::team_service::{RoleWithSkills, TeamWithRoles};

/// API response type
type ApiResponse<T> = Result<Json<T>, AppError>;

/// Application error (returns envelope format)
#[derive(Debug)]
pub struct AppError {
    status: StatusCode,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let body = serde_json::json!({ "ok": false, "error": self.message });
        (self.status, Json(body)).into_response()
    }
}

impl From<crate::services::team_service::TeamServiceError> for AppError {
    fn from(err: crate::services::team_service::TeamServiceError) -> Self {
        match err {
            crate::services::team_service::TeamServiceError::TeamNotFound(id) => AppError {
                status: StatusCode::NOT_FOUND,
                message: format!("Team not found: {}", id),
            },
            crate::services::team_service::TeamServiceError::RoleNotFound(id) => AppError {
                status: StatusCode::NOT_FOUND,
                message: format!("Role not found: {}", id),
            },
            crate::services::team_service::TeamServiceError::TelegramConfigNotFound(role_id) => {
                AppError {
                    status: StatusCode::NOT_FOUND,
                    message: format!("Telegram config not found for role: {}", role_id),
                }
            }
            _ => AppError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: err.to_string(),
            },
        }
    }
}

impl From<crate::services::agent_team_service::AgentTeamServiceError> for AppError {
    fn from(err: crate::services::agent_team_service::AgentTeamServiceError) -> Self {
        AppError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: err.to_string(),
        }
    }
}

// Team endpoints

/// List all teams
pub async fn list_teams(State(state): State<Arc<AppState>>) -> ApiResponse<Vec<Team>> {
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
    let team = state
        .teams_state
        .team_service
        .get_team_with_roles(&team_id)?;
    Ok(Json(team))
}

/// Update a team
pub async fn update_team(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<String>,
    Json(request): Json<UpdateTeamRequest>,
) -> ApiResponse<Team> {
    let team = state
        .teams_state
        .team_service
        .update_team(&team_id, request)?;
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
    let removed = state
        .teams_state
        .team_service
        .remove_role_from_team(&team_id, &role_id)?;
    Ok(Json(removed))
}

/// List all roles across all teams
pub async fn list_all_roles(State(state): State<Arc<AppState>>) -> ApiResponse<Vec<TeamRole>> {
    let roles = state.teams_state.team_service.list_all_roles()?;
    Ok(Json(roles))
}

/// Create a role in a team
pub async fn create_role(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<String>,
    Json(request): Json<CreateRoleRequest>,
) -> ApiResponse<TeamRole> {
    let role = state
        .teams_state
        .team_service
        .create_role(&team_id, request)?;
    Ok(Json(role))
}

/// Get a role by ID
pub async fn get_role(
    State(state): State<Arc<AppState>>,
    Path(role_id): Path<String>,
) -> ApiResponse<RoleWithSkills> {
    let role = state
        .teams_state
        .team_service
        .get_role_with_skills(&role_id)?;
    Ok(Json(role))
}

/// Update a role
pub async fn update_role(
    State(state): State<Arc<AppState>>,
    Path(role_id): Path<String>,
    Json(request): Json<UpdateRoleRequest>,
) -> ApiResponse<TeamRole> {
    let role = state
        .teams_state
        .team_service
        .update_role(&role_id, request)?;
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
    let role = state
        .teams_state
        .team_service
        .assign_role_to_team(&role_id, &request.team_id)?;
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
    let skill = state
        .teams_state
        .team_service
        .assign_skill(&role_id, &skill_id, priority)?;
    Ok(Json(skill))
}

/// Remove a skill from a role
pub async fn remove_skill(
    State(state): State<Arc<AppState>>,
    Path((role_id, skill_id)): Path<(String, String)>,
) -> ApiResponse<bool> {
    let removed = state
        .teams_state
        .team_service
        .remove_skill(&role_id, &skill_id)?;
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
    let messages = state
        .teams_state
        .team_service
        .get_team_messages(&team_id, params.limit)?;
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

    // 生成 execution_id 用于异步追踪
    let execution_id = uuid::Uuid::new_v4().to_string();
    let event_tx = state.agent_execution_manager.event_sender();
    let cancel_token = tokio_util::sync::CancellationToken::new();
    state
        .agent_execution_manager
        .register_cancel_token(&execution_id, cancel_token.clone());

    // ── PTY-first dispatch: resolve target role_id(s) ──────────────────────
    // trigger 匹配到多个角色 → 并行派发（如"前后端一起开发"）
    // trigger 未匹配 → 单个兜底角色（如"你好"不应所有角色都回复）
    let matched_role_ids = find_roles_by_trigger(&state, &team_id, &request.task).await;
    let target_role_ids = if !matched_role_ids.is_empty() {
        matched_role_ids
    } else {
        // 先尝试已有 PTY session 的角色，否则取团队第一个角色
        let sessions = state
            .claude_terminal_manager
            .list_sessions_for_team(&team_id);
        let from_session = sessions.first().and_then(|s| s.info.role_id.clone());
        if let Some(rid) = from_session {
            vec![rid]
        } else {
            // 无已有 session — 使用团队第一个角色，try_pty_dispatch_pub 会自动创建 session
            match state.teams_state.team_service.get_team_with_roles(&team_id) {
                Ok(twr) => twr
                    .roles
                    .first()
                    .map(|r| vec![r.role.id.clone()])
                    .unwrap_or_default(),
                Err(_) => vec![],
            }
        }
    };

    // task_summary 提前计算
    let task_summary = if request.task.len() > 80 {
        format!("{}...", &request.task[..80])
    } else {
        request.task.clone()
    };

    // ── 并行派发到所有匹配角色 ──
    // PTY 路径不经过 execute_team_task，需要在此保存用户消息
    if !target_role_ids.is_empty() {
        let user_msg = TeamMessage::user_message(team_id.clone(), request.task.clone());
        if let Err(e) = state.teams_state.team_service.add_message(user_msg) {
            tracing::warn!("[Route] 保存用户消息失败: {}", e);
        }
        let working_dir = state.current_workspace_path.read().clone();
        let mut primary_execution_id = execution_id.clone();
        let mut all_execution_ids = Vec::new();

        for (idx, role_id) in target_role_ids.iter().enumerate() {
            let exec_id = if idx == 0 {
                execution_id.clone()
            } else {
                uuid::Uuid::new_v4().to_string()
            };

            let ct = if idx == 0 {
                cancel_token.clone()
            } else {
                let ct = tokio_util::sync::CancellationToken::new();
                state
                    .agent_execution_manager
                    .register_cancel_token(&exec_id, ct.clone());
                ct
            };

            match try_pty_dispatch_pub(
                &state,
                &team_id,
                role_id,
                &request.task,
                &exec_id,
                working_dir.as_deref(),
                event_tx.clone(),
                ct,
                None,
            ) {
                Ok(session_id) => {
                    tracing::info!(
                        "[Route] PTY dispatch 成功, role: {}, session: {}, execution_id: {}",
                        role_id,
                        session_id,
                        exec_id
                    );
                    let _ =
                        event_tx.send(crate::ws::agent_execution::AgentExecutionEvent::Started {
                            execution_id: exec_id.clone(),
                            agent_role: "team".to_string(),
                            task_summary: task_summary.clone(),
                            role_id: Some(role_id.clone()),
                            session_id: Some(session_id),
                        });
                    all_execution_ids.push(exec_id);
                }
                Err(e) => {
                    tracing::warn!(
                        "[Route] PTY dispatch 失败 for role {}: {}, fallback 到原有路径",
                        role_id,
                        e
                    );
                }
            }
        }

        if !all_execution_ids.is_empty() {
            primary_execution_id = all_execution_ids[0].clone();

            // ── 后台监听所有 PTY 执行完成，保存 AI 回复到 team_messages ──
            for (role_idx, exec_id) in all_execution_ids.iter().enumerate() {
                let team_svc = state.teams_state.team_service.clone();
                let tid = team_id.clone();
                let rid = target_role_ids[role_idx].clone();
                let eid = exec_id.clone();
                let mut rx = event_tx.subscribe();
                tokio::spawn(async move {
                    let deadline = tokio::time::sleep(std::time::Duration::from_secs(1800));
                    tokio::pin!(deadline);
                    loop {
                        tokio::select! {
                            _ = &mut deadline => break,
                            evt = rx.recv() => {
                                match evt {
                                    Ok(crate::ws::agent_execution::AgentExecutionEvent::Completed { execution_id, result, .. })
                                        if execution_id == eid && !result.is_empty() =>
                                    {
                                        let msg = TeamMessage::assistant_message(tid, rid, result);
                                        if let Err(e) = team_svc.add_message(msg) {
                                            tracing::warn!("[Route] PTY完成保存AI回复失败: {}", e);
                                        }
                                        break;
                                    }
                                    Ok(crate::ws::agent_execution::AgentExecutionEvent::Failed { execution_id, .. })
                                        if execution_id == eid => break,
                                    Ok(crate::ws::agent_execution::AgentExecutionEvent::Cancelled { execution_id })
                                        if execution_id == eid => break,
                                    Err(_) => break,
                                    _ => {}
                                }
                            }
                        }
                    }
                });
            }

            // 构建 sessions 映射，让前端同步所有并行 PTY session 到 terminalSessions store
            let sessions_map: Vec<serde_json::Value> = all_execution_ids.iter().enumerate()
                .filter_map(|(i, eid)| {
                    let rid = target_role_ids.get(i)?;
                    let sid = state.claude_terminal_manager.list_sessions_for_team(&team_id)
                        .into_iter()
                        .find(|s| s.info.role_id.as_deref() == Some(rid))?;
                    Some(serde_json::json!({"role_id": rid, "session_id": sid.info.session_id, "execution_id": eid}))
                })
                .collect();

            return Ok(Json(crate::models::team::ExecuteTeamTaskResponse {
                success: true,
                team_id: team_id.clone(),
                messages: vec![],
                final_output: serde_json::json!({
                    "execution_id": primary_execution_id,
                    "status": "processing",
                    "parallel_count": all_execution_ids.len(),
                    "sessions": sessions_map,
                })
                .to_string(),
                error: None,
            }));
        }
    }

    // Fallback Started 事件（无 PTY session）
    let fallback_role_id = target_role_ids.first().cloned();
    let _ = event_tx.send(crate::ws::agent_execution::AgentExecutionEvent::Started {
        execution_id: execution_id.clone(),
        agent_role: "team".to_string(),
        task_summary,
        role_id: fallback_role_id.clone(),
        session_id: None,
    });

    // ── Fallback: 原有执行路径 ───────────────────────────────────────────
    // 没有匹配到 PTY session 或 PTY dispatch 失败，使用原有异步执行逻辑
    let memory_context =
        search_and_build_context(&state.memory_state, &team_id, &request.task).await;

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
        auto_confirm: request.auto_confirm,
    };

    // 后台异步执行团队任务
    let state_bg = state.clone();
    let team_id_bg = team_id.clone();
    let target_role_id_bg = fallback_role_id.clone();
    let user_message = request.task.clone();
    let exec_id = execution_id.clone();
    let tx = event_tx.clone();
    let manager = state.agent_execution_manager.clone();
    let auto_confirm = request.auto_confirm;

    // 注册确认响应等待器
    let confirm_rx = manager.register_confirmation(&exec_id);

    tokio::spawn(async move {
        let start = std::time::Instant::now();
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
        interval.tick().await;

        let task_future = state_bg.teams_state.agent_team_service.execute_team_task(
            enhanced_request,
            Some((tx.clone(), exec_id.clone())),
            Some(confirm_rx),
            auto_confirm,
        );
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
            Ok(response) => {
                // Store conversation to memory
                let memory_state = state_bg.memory_state.clone();
                let assistant_reply = response.final_output.clone();
                let team_id_for_memory = team_id_bg.clone();
                tokio::spawn(async move {
                    if let Err(e) = store_raw_transcript(
                        &memory_state,
                        &team_id_for_memory,
                        "user",
                        &user_message,
                    )
                    .await
                    {
                        tracing::warn!("[Memory] Failed to store user transcript: {}", e);
                    }
                    if !assistant_reply.is_empty() {
                        if let Err(e) = store_raw_transcript(
                            &memory_state,
                            &team_id_for_memory,
                            "assistant",
                            &assistant_reply,
                        )
                        .await
                        {
                            tracing::warn!("[Memory] Failed to store assistant transcript: {}", e);
                        }
                        if let Err(e) = store_structured_memory(
                            &memory_state,
                            &team_id_for_memory,
                            &user_message,
                            &assistant_reply,
                        )
                        .await
                        {
                            tracing::warn!("[Memory] Structured storage failed: {}", e);
                            let _ = store_to_memory(
                                &memory_state,
                                &team_id_for_memory,
                                "assistant",
                                &assistant_reply,
                            )
                            .await;
                        }
                    }
                });

                // AI 回复由 execute_team_task 内部保存，此处不重复保存

                let event = crate::ws::agent_execution::AgentExecutionEvent::Completed {
                    execution_id: exec_id.clone(),
                    result: response.final_output,
                    duration_ms: start.elapsed().as_millis() as u64,
                };
                manager.cache_terminal_event(event.clone());
                let _ = tx.send(event);
            }
            Err(e) => {
                let event = crate::ws::agent_execution::AgentExecutionEvent::Failed {
                    execution_id: exec_id.clone(),
                    error: format!("{}", e),
                };
                manager.cache_terminal_event(event.clone());
                let _ = tx.send(event);
            }
        }
        manager.remove_execution(&exec_id);
    });

    // 立即返回 execution_id
    Ok(Json(crate::models::team::ExecuteTeamTaskResponse {
        success: true,
        team_id: team_id.clone(),
        messages: vec![],
        final_output: format!(
            "{{\"execution_id\":\"{}\",\"status\":\"processing\"}}",
            execution_id
        ),
        error: None,
    }))
}

/// Try to dispatch a task to an existing or auto-created PTY session.
/// Returns Ok(session_id) if dispatch succeeded, Err with reason if fallback needed.
/// Public wrapper used by pipeline dispatch and other modules.
/// 智能处理 claude 启动时的对话框，处理完后 dispatch 任务
///
/// 监听 PTY 输出 15 秒，识别两种 dialog：
/// - `Bypass Permissions`：默认选中 "No, exit"，需要发 ↓ + Enter 选 Yes
/// - `trust this folder`：默认选中 "Yes, I trust"，直接发 Enter 即可
///
/// 看到输入框就绪标志或 15 秒超时后，发任务文本。
fn handle_startup_dialogs_and_dispatch(
    session: std::sync::Arc<crate::services::claude_terminal::ClaudeTerminalSession>,
    prompt: String,
) {
    use std::time::{Duration, Instant};
    use tokio::sync::broadcast::error::TryRecvError;

    let mut rx = session.subscribe_output();
    let mut buffer = String::new();
    let start = Instant::now();
    let dialog_timeout = Duration::from_secs(15);
    let mut handled_bypass = false;
    let mut handled_trust = false;

    loop {
        if start.elapsed() > dialog_timeout {
            tracing::info!("[StartupDialog] 超时未发现 dialog 或就绪标志，直接 dispatch");
            break;
        }

        match rx.try_recv() {
            Ok(bytes) => {
                let chunk = String::from_utf8_lossy(&bytes);
                buffer.push_str(&chunk);
                let clean = crate::services::agent_team_service::strip_ansi(&buffer);

                // 1. Bypass Permissions dialog —— 默认 "1. No, exit"，需要 ↓ + Enter 选 Yes
                if !handled_bypass && clean.contains("Bypass Permissions") {
                    tracing::info!("[StartupDialog] 检测到 Bypass Permissions dialog，选 Yes");
                    std::thread::sleep(Duration::from_millis(300));
                    // ↓ 键: ESC [ B
                    session.send_input(vec![0x1B, 0x5B, 0x42]);
                    std::thread::sleep(Duration::from_millis(200));
                    session.send_enter();
                    handled_bypass = true;
                    buffer.clear();
                    std::thread::sleep(Duration::from_millis(800));
                    continue;
                }

                // 2. workspace trust dialog —— 默认 "1. Yes, I trust"，直接 Enter
                if !handled_trust && clean.contains("trust this folder") {
                    tracing::info!("[StartupDialog] 检测到 trust folder dialog，发 Enter 确认");
                    std::thread::sleep(Duration::from_millis(300));
                    session.send_enter();
                    handled_trust = true;
                    buffer.clear();
                    std::thread::sleep(Duration::from_millis(800));
                    continue;
                }

                // 3. 输入框就绪：claude TUI prompt 区域显示 ">"
                // 至少等 3 秒（避免 banner 里的 "> " 被误判）
                if (handled_bypass || handled_trust || start.elapsed() > Duration::from_secs(3))
                    && (clean.contains("\n> ") || clean.contains("│ >"))
                {
                    tracing::info!("[StartupDialog] 检测到输入框就绪，开始 dispatch");
                    break;
                }
            }
            Err(TryRecvError::Empty) => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(TryRecvError::Closed) => {
                tracing::warn!("[StartupDialog] PTY 通道已关闭，放弃 dispatch");
                return;
            }
            Err(TryRecvError::Lagged(_)) => {
                // 落后了无所谓，继续
            }
        }
    }

    // 短暂延迟让 claude 完成最后的渲染，然后 dispatch
    std::thread::sleep(Duration::from_millis(500));
    session.dispatch_task(&prompt);
}

pub fn try_pty_dispatch_pub(
    state: &Arc<AppState>,
    team_id: &str,
    role_id: &str,
    task: &str,
    execution_id: &str,
    working_dir: Option<&str>,
    event_tx: tokio::sync::broadcast::Sender<crate::ws::agent_execution::AgentExecutionEvent>,
    cancel_token: tokio_util::sync::CancellationToken,
    pipeline_step_id: Option<&str>,
) -> Result<String, String> {
    // Write checkpoint for crash-resume (R1)
    if let Some(rs) = &state.resume_service {
        let project_id = state
            .project_service
            .list_projects_by_team(team_id)
            .ok()
            .and_then(|projects| projects.into_iter().next())
            .map(|p| p.id)
            .unwrap_or_default();
        if !project_id.is_empty() {
            if let Err(e) =
                rs.create_checkpoint(execution_id, &project_id, pipeline_step_id, role_id, task)
            {
                tracing::warn!("[Checkpoint] 创建检查点失败: {}", e);
            } else {
                tracing::info!("[Checkpoint] 已创建检查点, execution_id: {}", execution_id);
            }
        }
    }
    // Get or create a PTY session for this role
    let session = state
        .claude_terminal_manager
        .get_or_create_session(team_id, role_id, working_dir, 80, 24)
        .map_err(|e| format!("Failed to get or create terminal session: {}", e))?;
    let session_id = session.info.session_id.clone();

    // Build the full prompt using the shared function
    let team_context = {
        // Build minimal team context from available roles
        let teams_state = &state.teams_state;
        match teams_state.team_service.get_team_with_roles(team_id) {
            Ok(twr) => {
                crate::services::agent_team_service::AgentTeamService::build_team_context_pub(
                    &twr.team, &twr.roles,
                )
            }
            Err(_) => String::new(),
        }
    };

    let memory_context = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(search_and_build_context(
            &state.memory_state,
            team_id,
            task,
        ))
    });
    let full_prompt = crate::services::agent_team_service::build_team_prompt(
        &team_context,
        &memory_context,
        task,
        working_dir,
    );

    // 智能 dialog 处理：在后台 thread 里监听 PTY 输出，检测启动 dialog（trust folder / bypass
    // permissions），按 dialog 类型选择正确的按键，等 claude 输入框就绪后再 dispatch_task。
    // 这样不会因为 claude 启动慢、dialog 出现时机不可预测而卡住。
    let session_for_dialog = session.clone();
    let prompt_clone = full_prompt.clone();
    std::thread::spawn(move || {
        handle_startup_dialogs_and_dispatch(session_for_dialog, prompt_clone);
    });

    // Start the PTY task watcher in a background task
    let session_clone = session.clone();
    let exec_id = execution_id.to_string();
    let manager = state.agent_execution_manager.clone();
    let rs = state.resume_service.clone();

    tokio::spawn(async move {
        crate::services::pty_task_watcher::watch_pty_task(
            exec_id,
            session_clone,
            event_tx,
            cancel_token,
            manager,
            rs,
        )
        .await;
    });

    Ok(session_id)
}

/// 根据触发关键词查找所有匹配的角色
async fn find_roles_by_trigger(state: &Arc<AppState>, team_id: &str, task: &str) -> Vec<String> {
    let Ok(roles) = state.teams_state.team_service.list_roles(team_id) else {
        return vec![];
    };

    let task_lower = task.to_lowercase();
    let mut matched = Vec::new();

    for role in roles {
        for keyword in &role.trigger_keywords {
            if task_lower.contains(&keyword.to_lowercase()) {
                tracing::info!("[Route] 角色 '{}' 匹配关键词 '{}'", role.name, keyword);
                if !matched.contains(&role.id) {
                    matched.push(role.id);
                }
                break;
            }
        }
    }

    matched
}

/// execute_role_task 的内部版本，支持传入已构建的请求和可选的流式发送器
async fn execute_role_task_with_context(
    State(state): State<Arc<AppState>>,
    Json(request): Json<crate::models::team::ExecuteRoleTaskRequest>,
    stream_tx: Option<(
        tokio::sync::broadcast::Sender<crate::ws::agent_execution::AgentExecutionEvent>,
        String,
    )>,
) -> ApiResponse<crate::models::team::ExecuteRoleTaskResponse> {
    tracing::info!(
        "[Route] execute_role_task_with_context called, role_id: {}",
        request.role_id
    );

    // 1. 获取 role 信息以确定 team_id
    let role_with_skills = state
        .teams_state
        .team_service
        .get_role_with_skills(&request.role_id)
        .map_err(|e| {
            AppError::from(
                crate::services::team_service::TeamServiceError::RoleNotFound(e.to_string()),
            )
        })?;

    let role_team_id = role_with_skills.role.team_id.clone().unwrap_or_default();
    tracing::info!("[Route] role_team_id: {}", role_team_id);

    // 2. 搜索相关记忆
    let memory_context =
        search_and_build_context(&state.memory_state, &role_team_id, &request.task).await;
    tracing::info!(
        "[Memory] role_id: {}, memory_context length: {}",
        request.role_id,
        memory_context.len()
    );

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
    let response = state
        .teams_state
        .agent_team_service
        .execute_role_task(enhanced_request, stream_tx)
        .await?;

    // 5. Store conversation to memory (async, non-blocking)
    let memory_state = state.memory_state.clone();
    let team_id_clone = role_team_id.clone();
    let user_message = request.task.clone();
    let assistant_reply = response.response.clone();

    tokio::spawn(async move {
        if let Err(e) =
            store_raw_transcript(&memory_state, &team_id_clone, "user", &user_message).await
        {
            tracing::warn!("[Memory] Failed to store user transcript: {}", e);
        }
        if !assistant_reply.is_empty() {
            if let Err(e) =
                store_raw_transcript(&memory_state, &team_id_clone, "assistant", &assistant_reply)
                    .await
            {
                tracing::warn!("[Memory] Failed to store assistant transcript: {}", e);
            }
            if let Err(e) = store_structured_memory(
                &memory_state,
                &team_id_clone,
                &user_message,
                &assistant_reply,
            )
            .await
            {
                tracing::warn!("[Memory] Structured storage failed, falling back: {}", e);
                let _ =
                    store_to_memory(&memory_state, &team_id_clone, "assistant", &assistant_reply)
                        .await;
            }
        }
    });

    Ok(Json(response))
}

/// 查询扩展缓存（query -> (expanded, timestamp)）
fn query_cache() -> &'static Mutex<HashMap<String, (String, std::time::Instant)>> {
    static CACHE: OnceLock<Mutex<HashMap<String, (String, std::time::Instant)>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

const QUERY_CACHE_TTL_SECS: u64 = 300; // 5 分钟

/// 带缓存的查询扩展
async fn expand_query_cached(query: &str) -> String {
    // 检查缓存
    if let Ok(cache) = query_cache().lock() {
        if let Some((expanded, ts)) = cache.get(query) {
            if ts.elapsed().as_secs() < QUERY_CACHE_TTL_SECS {
                return expanded.clone();
            }
        }
    }

    // 通过 Claude CLI 扩展
    match crate::services::claude_cli::expand_query_for_search(query).await {
        Ok(expanded) => {
            if let Ok(mut cache) = query_cache().lock() {
                cache.insert(
                    query.to_string(),
                    (expanded.clone(), std::time::Instant::now()),
                );
                // 驱逐过期条目
                cache.retain(|_, (_, ts)| ts.elapsed().as_secs() < QUERY_CACHE_TTL_SECS * 2);
            }
            expanded
        }
        Err(e) => {
            tracing::warn!("[Memory] Query expansion failed: {}, using raw query", e);
            query.to_string()
        }
    }
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

    // 查询扩展：提升 BM25 召回率
    let expanded_query = expand_query_cached(query).await;
    tracing::debug!(
        "[Memory] Query expanded: '{}' -> '{}'",
        query,
        expanded_query.chars().take(200).collect::<String>()
    );

    // 搜索相关记忆
    let search_request = SearchRequest {
        team_id: Some(team_id.to_string()),
        query: expanded_query,
        top_k: Some(5),
        vector_weight: None,
        keyword_weight: None,
        session_id: None,
    };

    match memory_state.search.search(&search_request, None) {
        Ok(results) if !results.results.is_empty() => {
            // 过滤低相关性结果
            let meaningful: Vec<_> = results.results.iter().filter(|r| r.score >= 0.5).collect();

            if meaningful.is_empty() {
                tracing::debug!("[Memory] 所有结果低于相关性阈值");
                return String::new();
            }

            let mut context = String::from("\n\n## Relevant Conversation History\n\n");

            for result in &meaningful {
                // 尝试解析结构化记忆（新格式）
                if let Ok(structured) = serde_json::from_str::<serde_json::Value>(&result.content) {
                    if let (Some(topic), Some(problem), Some(solution)) = (
                        structured.get("topic").and_then(|v| v.as_str()),
                        structured.get("problem").and_then(|v| v.as_str()),
                        structured.get("solution").and_then(|v| v.as_str()),
                    ) {
                        let timestamp = structured
                            .get("timestamp")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        let keywords = structured
                            .get("keywords")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            })
                            .unwrap_or_default();
                        context.push_str(&format!(
                            "**[{}] Topic: {}**\n- Problem: {}\n- Solution: {}\n- Keywords: {}\n\n",
                            timestamp, topic, problem, solution, keywords,
                        ));
                        continue;
                    }
                }

                // Fallback：旧格式（原始文本）
                let role = result.metadata.user_name.as_deref().unwrap_or("unknown");
                context.push_str(&format!(
                    "**[{}] {}:** {}\n\n",
                    result.created_at.format("%Y-%m-%d %H:%M"),
                    role,
                    result.content,
                ));
            }

            context.push_str(
                "Use the above conversation history to provide contextually relevant responses.\n",
            );
            tracing::info!("[Memory] 注入 {} 条相关记忆", meaningful.len());
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
        if role == "user" {
            MessageRole::User
        } else {
            MessageRole::Assistant
        },
        content,
    );

    memory_state
        .store
        .store_transcript(&transcript)
        .map_err(|e| e.to_string())?;

    let chunk = MemoryChunk::from_transcript(&transcript, content.to_string(), 0);
    memory_state
        .store
        .store_chunk(&chunk)
        .map_err(|e| e.to_string())?;

    // 索引
    let metadata = serde_json::json!({
        "transcript_id": transcript.id,
        "role": role,
        "created_at": transcript.created_at.to_rfc3339(),
    });
    memory_state
        .search
        .index_chunk(team_id, &chunk.id, &chunk.content, metadata)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// 存储结构化记忆摘要（使用 Claude CLI 进行摘要）
async fn store_structured_memory(
    memory_state: &Arc<crate::routes::memory::MemoryState>,
    team_id: &str,
    user_message: &str,
    assistant_reply: &str,
) -> Result<(), String> {
    use crate::services::claude_cli::summarize_for_memory;

    let structured = summarize_for_memory(user_message, assistant_reply).await?;

    // 创建 transcript（存储摘要）
    let transcript = Transcript::new(
        team_id,
        "system",
        MessageRole::Assistant,
        &structured.summary,
    );

    memory_state
        .store
        .store_transcript(&transcript)
        .map_err(|e| e.to_string())?;

    // 存储完整结构化 JSON 为 chunk content（上下文注入时使用）
    let chunk_content = serde_json::to_string(&structured).map_err(|e| e.to_string())?;
    let chunk = MemoryChunk::from_transcript(&transcript, chunk_content, 0);
    memory_state
        .store
        .store_chunk(&chunk)
        .map_err(|e| e.to_string())?;

    // BM25 索引用关键词丰富的拼接文本（提升召回率）
    let searchable_content = format!(
        "{} {} {} {}",
        structured.topic,
        structured.problem,
        structured.solution,
        structured.keywords.join(" "),
    );

    let metadata = serde_json::json!({
        "transcript_id": transcript.id,
        "role": structured.role,
        "topic": structured.topic,
        "created_at": transcript.created_at.to_rfc3339(),
        "keywords": structured.keywords,
    });

    memory_state
        .search
        .index_chunk(team_id, &chunk.id, &searchable_content, metadata)
        .await
        .map_err(|e| e.to_string())?;

    tracing::info!(
        "[Memory] Stored structured memory: topic='{}', keywords={:?}",
        structured.topic,
        structured.keywords
    );

    Ok(())
}

/// 存储原始对话记录（仅入库，不索引，用于审计追踪）
async fn store_raw_transcript(
    memory_state: &Arc<crate::routes::memory::MemoryState>,
    team_id: &str,
    role: &str,
    content: &str,
) -> Result<(), String> {
    let transcript = Transcript::new(
        team_id,
        "system",
        if role == "user" {
            MessageRole::User
        } else {
            MessageRole::Assistant
        },
        content,
    );
    memory_state
        .store
        .store_transcript(&transcript)
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Execute a task for a single role (with its assigned skills)
pub async fn execute_role_task(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ExecuteRoleTaskRequest>,
) -> ApiResponse<ExecuteRoleTaskResponse> {
    tracing::info!(
        "[Route] execute_role_task 被调用，role_id: {}",
        request.role_id
    );

    // 1. 获取 role 信息以确定 team_id
    let role_with_skills = state
        .teams_state
        .team_service
        .get_role_with_skills(&request.role_id)
        .map_err(|e| {
            AppError::from(
                crate::services::team_service::TeamServiceError::RoleNotFound(e.to_string()),
            )
        })?;
    let team_id = role_with_skills.role.team_id.clone().unwrap_or_default();

    // 2. 搜索相关记忆
    let memory_context =
        search_and_build_context(&state.memory_state, &team_id, &request.task).await;

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
        .execute_role_task(enhanced_request, None)
        .await?;

    // 5. Store conversation to memory (async, non-blocking)
    let memory_state = state.memory_state.clone();
    let team_id_clone = team_id.clone();
    let user_message = request.task.clone();
    let assistant_reply = response.response.clone();

    tokio::spawn(async move {
        if let Err(e) =
            store_raw_transcript(&memory_state, &team_id_clone, "user", &user_message).await
        {
            tracing::warn!("[Memory] Failed to store user transcript: {}", e);
        }
        if !assistant_reply.is_empty()
            && assistant_reply != "Message received, processing in background..."
        {
            if let Err(e) =
                store_raw_transcript(&memory_state, &team_id_clone, "assistant", &assistant_reply)
                    .await
            {
                tracing::warn!("[Memory] Failed to store assistant transcript: {}", e);
            }
            if let Err(e) = store_structured_memory(
                &memory_state,
                &team_id_clone,
                &user_message,
                &assistant_reply,
            )
            .await
            {
                tracing::warn!("[Memory] Structured storage failed, falling back: {}", e);
                let _ =
                    store_to_memory(&memory_state, &team_id_clone, "assistant", &assistant_reply)
                        .await;
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
    let config = state.teams_state.team_service.configure_telegram(
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
    let config = state
        .teams_state
        .team_service
        .get_telegram_config(&role_id)?;
    Ok(Json(config))
}

/// Enable or disable Telegram for a role
pub async fn enable_telegram(
    State(state): State<Arc<AppState>>,
    Path((role_id, enabled)): Path<(String, bool)>,
) -> ApiResponse<TelegramBotConfig> {
    let config = state
        .teams_state
        .team_service
        .enable_telegram(&role_id, enabled)?;

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
    let deleted = state
        .teams_state
        .team_service
        .delete_telegram_config(&role_id)?;
    Ok(Json(deleted))
}

// Team-level Telegram endpoints (delegates to first role's Telegram config)

/// Get Telegram configuration for a team (delegates to first role)
pub async fn get_team_telegram_config(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<String>,
) -> ApiResponse<crate::models::team::TelegramBotConfig> {
    // Get the first role of the team
    let roles = state.teams_state.team_service.list_roles(&team_id)?;

    let first_role = roles.into_iter().next().ok_or_else(|| AppError {
        status: StatusCode::NOT_FOUND,
        message: format!("No roles found for team {}", team_id),
    })?;

    let config = state
        .teams_state
        .team_service
        .get_telegram_config(&first_role.id)?;
    Ok(Json(config))
}

/// Configure Telegram for a team (delegates to first role)
pub async fn configure_team_telegram(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<String>,
    Json(request): Json<crate::models::team::TelegramConfigRequest>,
) -> ApiResponse<crate::models::team::TelegramBotConfig> {
    // Get the first role of the team
    let roles = state.teams_state.team_service.list_roles(&team_id)?;

    let first_role = roles.into_iter().next().ok_or_else(|| AppError {
        status: StatusCode::NOT_FOUND,
        message: format!("No roles found for team {}", team_id),
    })?;

    let config = state.teams_state.team_service.configure_telegram(
        &first_role.id,
        request.bot_token,
        request.chat_id,
        request.notifications_enabled,
        request.conversation_enabled,
    )?;
    Ok(Json(config))
}

/// Enable or disable Telegram for a team (delegates to first role)
pub async fn enable_team_telegram(
    State(state): State<Arc<AppState>>,
    Path((team_id, enabled)): Path<(String, bool)>,
) -> ApiResponse<crate::models::team::TelegramBotConfig> {
    // Get the first role of the team
    let roles = state.teams_state.team_service.list_roles(&team_id)?;

    let first_role = roles.into_iter().next().ok_or_else(|| AppError {
        status: StatusCode::NOT_FOUND,
        message: format!("No roles found for team {}", team_id),
    })?;

    let config = state
        .teams_state
        .team_service
        .enable_telegram(&first_role.id, enabled)?;

    // Start or stop polling based on enabled state
    let telegram_service = &state.teams_state.telegram_service;
    if enabled {
        telegram_service.start_polling(first_role.id.clone(), config.bot_token.clone());
    } else {
        telegram_service.stop_polling(&first_role.id);
    }

    Ok(Json(config))
}

/// Get bot status for all members of a team
pub async fn get_team_member_bots(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<String>,
) -> ApiResponse<Vec<MemberBotStatus>> {
    let roles = state.teams_state.team_service.list_roles(&team_id)?;

    let statuses = roles
        .into_iter()
        .map(|role| {
            let bot_config = state
                .teams_state
                .team_service
                .get_telegram_config(&role.id)
                .ok();
            let is_polling = state.teams_state.telegram_service.is_polling(&role.id);
            MemberBotStatus {
                role_id: role.id.clone(),
                role_name: role.name.clone(),
                bot_config,
                is_polling,
            }
        })
        .collect();

    Ok(Json(statuses))
}

/// Configure bot for a specific member in a team
pub async fn configure_member_bot(
    State(state): State<Arc<AppState>>,
    Path((team_id, role_id)): Path<(String, String)>,
    Json(request): Json<MemberBotConfigItem>,
) -> ApiResponse<MemberBotStatus> {
    // Verify role belongs to this team
    let roles = state.teams_state.team_service.list_roles(&team_id)?;
    if !roles.iter().any(|r| r.id == role_id) {
        return Err(AppError {
            status: StatusCode::NOT_FOUND,
            message: format!("Role {} not found in team {}", role_id, team_id),
        });
    }

    let bot_config = state.teams_state.team_service.configure_telegram(
        &role_id,
        request.bot_token,
        request.chat_id,
        request.notifications_enabled,
        request.conversation_enabled,
    )?;

    let role = roles.into_iter().find(|r| r.id == role_id).unwrap();
    let is_polling = state.teams_state.telegram_service.is_polling(&role_id);

    Ok(Json(MemberBotStatus {
        role_id,
        role_name: role.name,
        bot_config: Some(bot_config),
        is_polling,
    }))
}

/// Toggle bot polling for all members in a team
pub async fn toggle_all_member_bots(
    State(state): State<Arc<AppState>>,
    Path((team_id, enabled)): Path<(String, bool)>,
) -> ApiResponse<Vec<MemberBotStatus>> {
    let roles = state.teams_state.team_service.list_roles(&team_id)?;

    let mut statuses = Vec::new();
    for role in &roles {
        let config_result = state
            .teams_state
            .team_service
            .enable_telegram(&role.id, enabled);
        let bot_config = match config_result {
            Ok(cfg) => {
                if enabled {
                    state
                        .teams_state
                        .telegram_service
                        .start_polling(role.id.clone(), cfg.bot_token.clone());
                } else {
                    state.teams_state.telegram_service.stop_polling(&role.id);
                }
                Some(cfg)
            }
            Err(_) => {
                // Role has no bot configured — skip silently
                None
            }
        };

        let is_polling = state.teams_state.telegram_service.is_polling(&role.id);
        statuses.push(MemberBotStatus {
            role_id: role.id.clone(),
            role_name: role.name.clone(),
            bot_config,
            is_polling,
        });
    }

    Ok(Json(statuses))
}

/// Send a test message via Telegram
pub async fn send_telegram_message(
    State(state): State<Arc<AppState>>,
    Path(role_id): Path<String>,
    Json(request): Json<crate::models::team::TelegramSendMessageRequest>,
) -> Result<StatusCode, AppError> {
    let config = state
        .teams_state
        .team_service
        .get_telegram_config(&role_id)?;

    state
        .teams_state
        .telegram_service
        .send_message(&config.bot_token, &request.chat_id, &request.text)
        .await
        .map_err(|e| AppError {
            status: StatusCode::BAD_REQUEST,
            message: e.to_string(),
        })?;

    Ok(StatusCode::OK)
}
