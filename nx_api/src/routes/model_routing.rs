//! 模型路由规则 CRUD API

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use std::sync::Arc;

use crate::routes::AppState;
use crate::services::model_router::RoutingRule;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/v1/ai/routing-rules",
            get(list_rules).post(create_rule),
        )
        .route(
            "/api/v1/ai/routing-rules/:id",
            put(update_rule).delete(delete_rule),
        )
        .route("/api/v1/ai/routing-rules/test", post(test_route))
}

async fn list_rules(State(state): State<Arc<AppState>>) -> Json<Vec<RoutingRule>> {
    Json(state.execution_service.get_routing_rules())
}

async fn create_rule(
    State(state): State<Arc<AppState>>,
    Json(rule): Json<RoutingRule>,
) -> (StatusCode, Json<RoutingRule>) {
    let mut rules = state.execution_service.get_routing_rules();
    rules.push(rule.clone());
    state.execution_service.set_routing_rules(rules);
    (StatusCode::CREATED, Json(rule))
}

async fn update_rule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(updated): Json<RoutingRule>,
) -> Result<Json<RoutingRule>, StatusCode> {
    let mut rules = state.execution_service.get_routing_rules();
    let pos = rules
        .iter()
        .position(|r| r.id == id)
        .ok_or(StatusCode::NOT_FOUND)?;
    rules[pos] = updated.clone();
    state.execution_service.set_routing_rules(rules);
    Ok(Json(updated))
}

async fn delete_rule(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> StatusCode {
    let mut rules = state.execution_service.get_routing_rules();
    let before = rules.len();
    rules.retain(|r| r.id != id);
    if rules.len() == before {
        return StatusCode::NOT_FOUND;
    }
    state.execution_service.set_routing_rules(rules);
    StatusCode::NO_CONTENT
}

#[derive(serde::Deserialize)]
struct TestRouteRequest {
    prompt: String,
    task_type: Option<String>,
}

#[derive(serde::Serialize)]
struct TestRouteResponse {
    model: Option<String>,
}

async fn test_route(
    State(state): State<Arc<AppState>>,
    Json(req): Json<TestRouteRequest>,
) -> Json<TestRouteResponse> {
    use crate::services::model_router::{ModelRouter, TaskContext};
    let rules = state.execution_service.get_routing_rules();
    let router = ModelRouter::new(rules);
    let ctx = TaskContext {
        prompt: &req.prompt,
        task_type: req.task_type.as_deref(),
    };
    Json(TestRouteResponse {
        model: router.route(&ctx),
    })
}
