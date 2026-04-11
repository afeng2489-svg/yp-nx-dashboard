//! A2UI API Routes
//!
//! Handles interactive communication between agents and users.

use axum::{
    extract::{Path, State, WebSocketUpgrade},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use futures_util::StreamExt;

use crate::a2ui::{
    A2UIService, A2UISession, InteractiveMessage, UserResponseRequest, MessagesResponse,
    A2UIMessage, U2AMessage, A2UISessionEvent, A2UIMessageExt,
};
use crate::error::{ApiError, ApiResult};
use super::AppState;

/// Application state for A2UI
pub struct A2UIState {
    pub service: Arc<A2UIService>,
}

/// Create A2UI router
pub fn create_a2ui_router(service: Arc<A2UIService>) -> Router {
    let state = A2UIState { service };

    Router::new()
        .route("/api/v1/a2ui/sessions/:id/messages", get(get_messages))
        .route("/api/v1/a2ui/sessions/:id/respond", post(respond_to_message))
        .route("/api/v1/a2ui/sessions", get(list_sessions))
        .route("/api/v1/a2ui/sessions/:id", get(get_session))
        .route("/ws/a2ui/:execution_id", get(a2ui_ws_handler))
        .with_state(state)
}

/// GET /api/v1/a2ui/sessions/:id/messages
/// Get all messages for a session, including pending ones
async fn get_messages(
    State(state): State<A2UIState>,
    Path(session_id): Path<String>,
) -> ApiResult<Json<MessagesResponse>> {
    let messages = state.service.get_all_messages(&session_id);
    let pending_count = messages.iter().filter(|m| m.pending).count();

    Ok(Json(MessagesResponse {
        messages,
        pending_count,
    }))
}

/// POST /api/v1/a2ui/sessions/:id/respond
/// User responds to a pending message
async fn respond_to_message(
    State(state): State<A2UIState>,
    Path(session_id): Path<String>,
    Json(request): Json<UserResponseRequest>,
) -> ApiResult<Json<InteractiveMessage>> {
    let message = state
        .service
        .respond(&session_id, &request.message_id, request.response)?;

    Ok(Json(message))
}

/// GET /api/v1/a2ui/sessions
/// List all active sessions
async fn list_sessions(State(state): State<A2UIState>) -> ApiResult<Json<Vec<A2UISession>>> {
    let sessions = state.service.session_manager().list_sessions();
    Ok(Json(sessions))
}

/// GET /api/v1/a2ui/sessions/:id
/// Get a specific session
async fn get_session(
    State(state): State<A2UIState>,
    Path(session_id): Path<String>,
) -> ApiResult<Json<A2UISession>> {
    let session = state
        .service
        .get_session(&session_id)
        .ok_or_else(|| ApiError::SessionNotFound(session_id))?;

    Ok(Json(session))
}

/// WebSocket handler for real-time A2UI
async fn a2ui_ws_handler(
    State(state): State<A2UIState>,
    Path(execution_id): Path<String>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let service = state.service.clone();

    ws.on_upgrade(async move |socket| {
        let (sender, receiver) = socket.split();
        let service = service.clone();

        // Get or create session for this execution
        let session = service.get_or_create_session(&execution_id);
        let session_id = session.id.clone();

        // Subscribe to session events
        let mut event_rx = service.subscribe(&session_id).unwrap_or_else(|| {
            // Create a dummy receiver if subscription fails
            let (_tx, rx) = broadcast::channel(100);
            rx
        });

        // Spawn task to handle outgoing messages
        let send_handle = tokio::spawn(async move {
            let mut stream = BroadcastStream::new(event_rx);
            while let Some(Ok(event)) = stream.next().await {
                let msg = match &event {
                    A2UISessionEvent::NewMessage(msg) => {
                        serde_json::json!({
                            "type": "message",
                            "data": msg
                        })
                    }
                    A2UISessionEvent::PendingMessage(msg) => {
                        serde_json::json!({
                            "type": "pending",
                            "data": msg
                        })
                    }
                    A2UISessionEvent::UserResponse { message_id, response } => {
                        serde_json::json!({
                            "type": "response",
                            "message_id": message_id,
                            "response": response
                        })
                    }
                    A2UISessionEvent::SessionEnded { execution_id } => {
                        serde_json::json!({
                            "type": "ended",
                            "execution_id": execution_id
                        })
                    }
                };

                if sender.send(axum::extract::ws::Message::Text(msg.to_string())).is_err() {
                    break;
                }
            }
        });

        // Handle incoming messages (user responses via WebSocket)
        let recv_handle = tokio::spawn(async move {
            let mut receiver = receiver;
            while let Some(Ok(msg)) = receiver.next().await {
                if let axum::extract::ws::Message::Text(text) = msg {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Some(response_type) = parsed.get("type").and_then(|t| t.as_str()) {
                            match response_type {
                                "respond" => {
                                    if let (Some(message_id), Some(response)) = (
                                        parsed.get("message_id").and_then(|v| v.as_str()),
                                        parsed.get("response").and_then(|v| {
                                            serde_json::from_value::<U2AMessage>(v.clone()).ok()
                                        }),
                                    ) {
                                        let _ = service.respond(&session_id, message_id, response);
                                    }
                                }
                                "ping" => {
                                    // Heartbeat/ping - respond with pong
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        });

        // Wait for both handles to complete
        let _ = tokio::join!(send_handle, recv_handle);
    })
}