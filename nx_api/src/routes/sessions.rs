//! 会话路由

use std::sync::Arc;
use axum::{
    extract::{Path, State},
    Json,
};
use futures_util::{SinkExt, StreamExt};

use crate::services::session_repository::RepositoryError;
use super::AppState;

/// 列出会话
pub async fn list_sessions(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<SessionSummary>>, (axum::http::StatusCode, String)> {
    let sessions = state.session_service.list_sessions().await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let summaries: Vec<SessionSummary> = sessions.into_iter().map(|s| SessionSummary {
        id: s.id,
        workflow_id: s.workflow_id,
        workflow_name: String::new(),
        status: s.status.to_string(),
        resume_key: s.resume_key,
        created_at: s.created_at.to_rfc3339(),
        updated_at: s.updated_at.to_rfc3339(),
    }).collect();

    Ok(Json(summaries))
}

/// 创建会话
pub async fn create_session(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateSessionRequest>,
) -> Result<Json<SessionResponse>, (axum::http::StatusCode, String)> {
    let session = state.session_service.create_session(payload.workflow_id).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tracing::info!("创建新会话: {}", session.id);

    Ok(Json(SessionResponse {
        id: session.id,
        workflow_id: session.workflow_id,
        status: session.status.to_string(),
        resume_key: session.resume_key,
        created_at: session.created_at.to_rfc3339(),
        updated_at: session.updated_at.to_rfc3339(),
    }))
}

/// 获取会话
pub async fn get_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<SessionResponse>, (axum::http::StatusCode, String)> {
    let session = state.session_service.get_session(&id).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match session {
        Some(s) => Ok(Json(SessionResponse {
            id: s.id,
            workflow_id: s.workflow_id,
            status: s.status.to_string(),
            resume_key: s.resume_key,
            created_at: s.created_at.to_rfc3339(),
            updated_at: s.updated_at.to_rfc3339(),
        })),
        None => Err((axum::http::StatusCode::NOT_FOUND, format!("会话 {} 不存在", id))),
    }
}

/// 删除会话
pub async fn delete_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<DeleteSessionResponse>, (axum::http::StatusCode, String)> {
    let deleted = state.session_service.delete_session(&id).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if deleted {
        tracing::info!("删除会话: {}", id);
        Ok(Json(DeleteSessionResponse {
            success: true,
            message: format!("会话 {} 已删除", id),
        }))
    } else {
        Err((axum::http::StatusCode::NOT_FOUND, format!("会话 {} 不存在", id)))
    }
}

/// 恢复会话
pub async fn resume_session(
    State(state): State<Arc<AppState>>,
    Path(resume_key): Path<String>,
) -> Result<Json<SessionResponse>, (axum::http::StatusCode, String)> {
    let session = state.session_service.resume_session(&resume_key).await
        .map_err(|e| (axum::http::StatusCode::NOT_FOUND, format!("无法恢复会话: {}", e)))?;

    tracing::info!("恢复会话: {}", session.id);

    Ok(Json(SessionResponse {
        id: session.id,
        workflow_id: session.workflow_id,
        status: session.status.to_string(),
        resume_key: session.resume_key,
        created_at: session.created_at.to_rfc3339(),
        updated_at: session.updated_at.to_rfc3339(),
    }))
}

/// 暂停会话
pub async fn pause_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<SessionResponse>, (axum::http::StatusCode, String)> {
    let session = state.session_service.pause_session(&id).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("暂停会话失败: {}", e)))?;

    tracing::info!("暂停会话: {}", id);

    Ok(Json(SessionResponse {
        id: session.id,
        workflow_id: session.workflow_id,
        status: session.status.to_string(),
        resume_key: session.resume_key,
        created_at: session.created_at.to_rfc3339(),
        updated_at: session.updated_at.to_rfc3339(),
    }))
}

/// 激活会话
pub async fn activate_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<SessionResponse>, (axum::http::StatusCode, String)> {
    let session = state.session_service.activate_session(&id).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("激活会话失败: {}", e)))?;

    tracing::info!("激活会话: {}", id);

    Ok(Json(SessionResponse {
        id: session.id,
        workflow_id: session.workflow_id,
        status: session.status.to_string(),
        resume_key: session.resume_key,
        created_at: session.created_at.to_rfc3339(),
        updated_at: session.updated_at.to_rfc3339(),
    }))
}

/// 同步会话状态
pub async fn sync_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<SessionResponse>, (axum::http::StatusCode, String)> {
    let session = state.session_service.sync_session(&id).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("同步会话失败: {}", e)))?;

    Ok(Json(SessionResponse {
        id: session.id,
        workflow_id: session.workflow_id,
        status: session.status.to_string(),
        resume_key: session.resume_key,
        created_at: session.created_at.to_rfc3339(),
        updated_at: session.updated_at.to_rfc3339(),
    }))
}

/// 会话 WebSocket
pub async fn session_ws(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<String>,
    ws: axum::extract::WebSocketUpgrade,
) -> impl axum::response::IntoResponse {
    tracing::info!("会话 WebSocket 连接: {}", id);

    ws.on_upgrade(|socket: axum::extract::ws::WebSocket| async move {
        use axum::extract::ws::Message;
        let (send, mut receive) = socket.split();

        tokio::spawn(async move {
            while let Some(msg) = receive.next().await {
                if let Ok(Message::Text(text)) = msg {
                    tracing::debug!("收到会话消息: {}", text);
                }
            }
        });
    })
}

// ============ 请求/响应类型 ============

#[derive(Debug, serde::Deserialize)]
pub struct CreateSessionRequest {
    pub workflow_id: String,
    #[serde(default)]
    pub variables: Option<serde_json::Value>,
}

#[derive(Debug, serde::Serialize)]
pub struct SessionResponse {
    pub id: String,
    pub workflow_id: String,
    pub status: String,
    pub resume_key: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, serde::Serialize)]
pub struct SessionSummary {
    pub id: String,
    pub workflow_id: String,
    pub workflow_name: String,
    pub status: String,
    pub resume_key: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, serde::Serialize)]
pub struct DeleteSessionResponse {
    pub success: bool,
    pub message: String,
}