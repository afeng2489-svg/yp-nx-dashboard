use axum::{
    extract::{Path, State},
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::AppState;
use crate::services::sprint_service::{SprintCard, SprintEvent};

pub fn sprint_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_sprints))
        .route("/", post(upsert_sprint))
        .route("/:id/status", put(update_status))
        .route("/:id/events", get(list_events))
        .route("/:id/events", post(add_event))
}

async fn list_sprints(State(state): State<Arc<AppState>>) -> Result<Json<Vec<SprintCard>>, String> {
    state.sprint_service.list().map(Json).map_err(|e| e.to_string())
}

async fn upsert_sprint(
    State(state): State<Arc<AppState>>,
    Json(card): Json<SprintCard>,
) -> Result<Json<SprintCard>, String> {
    state.sprint_service.upsert(&card).map_err(|e| e.to_string())?;
    Ok(Json(card))
}

#[derive(Deserialize)]
struct StatusBody {
    status: String,
}

async fn update_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<StatusBody>,
) -> Result<Json<serde_json::Value>, String> {
    state.sprint_service.update_status(&id, &body.status).map_err(|e| e.to_string())?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn list_events(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<SprintEvent>>, String> {
    state.sprint_service.events_for(&id).map(Json).map_err(|e| e.to_string())
}

#[derive(Deserialize, Serialize)]
struct AddEventBody {
    event_type: String,
    detail: Option<String>,
}

async fn add_event(
    State(state): State<Arc<AppState>>,
    Path(sprint_id): Path<String>,
    Json(body): Json<AddEventBody>,
) -> Result<Json<serde_json::Value>, String> {
    let event = SprintEvent {
        id: uuid::Uuid::new_v4().to_string(),
        sprint_id,
        event_type: body.event_type,
        detail: body.detail,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    state.sprint_service.record_event(&event).map_err(|e| e.to_string())?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
