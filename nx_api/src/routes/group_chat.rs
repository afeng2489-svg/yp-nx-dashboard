//! Group Chat Routes
//!
//! REST API endpoints for multi-agent group discussion.

use axum::{
    extract::{Path, Query, State},
    routing::{get, post, put, delete},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::error::ApiError;
use crate::models::group_chat::{
    ConcludeDiscussionRequest, CreateGroupSessionRequest, GetMessagesRequest,
    GroupConclusion, GroupMessage, GroupSession, GroupSessionDetail, DiscussionTurnInfo,
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
        vec![]  // Return empty if no team_id provided
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

    service
        .delete_session(&id)
        .await
        .map_err(ApiError::from)?;

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
        .send_message(&id, request)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(message))
}

/// Get next speaker
pub async fn get_next_speaker(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Option<(String, String)>>, ApiError> {
    let service = &state.group_chat_service;

    let next = service
        .get_next_speaker(&id)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(next))
}

/// Advance to next speaker
pub async fn advance_speaker(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<()>, ApiError> {
    let service = &state.group_chat_service;

    service
        .advance_speaker(&id)
        .await
        .map_err(ApiError::from)?;

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

/// Execute a role's turn (async — returns execution_id immediately)
pub async fn execute_role_turn(
    State(state): State<Arc<AppState>>,
    Path((id, role_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let execution_id = uuid::Uuid::new_v4().to_string();
    let event_tx = state.agent_execution_manager.event_sender();
    let cancel_token = tokio_util::sync::CancellationToken::new();
    state.agent_execution_manager.register_cancel_token(&execution_id, cancel_token.clone());

    // 发送 Started 事件
    let _ = event_tx.send(crate::ws::agent_execution::AgentExecutionEvent::Started {
        execution_id: execution_id.clone(),
        agent_role: role_id.clone(),
        task_summary: format!("Role turn: {}", role_id),
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
