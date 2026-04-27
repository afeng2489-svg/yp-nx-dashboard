//! Wisdom API routes
//!
//! REST endpoints for wisdom management.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::AppState;
use crate::wisdom::{
    CategorySummary, CreateWisdomRequest, QueryWisdomRequest, WisdomCategory, WisdomEntry,
    WisdomError, WisdomResponse, WisdomService,
};

/// 应用错误类型
#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    BadRequest(String),
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::Internal(msg) => {
                tracing::error!("Wisdom internal error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
        };

        let body = serde_json::json!({
            "error": message
        });

        (status, Json(body)).into_response()
    }
}

impl From<WisdomError> for AppError {
    fn from(err: WisdomError) -> Self {
        match err {
            WisdomError::NotFound(id) => AppError::NotFound(id),
            WisdomError::InvalidRequest(msg) => AppError::BadRequest(msg),
            WisdomError::Storage(msg) => AppError::Internal(msg.to_string()),
        }
    }
}

/// GET /api/v1/wisdom
/// Query wisdom entries with optional filters
pub async fn list_wisdom(
    State(state): State<Arc<AppState>>,
    Query(params): Query<WisdomQueryParams>,
) -> Result<Json<WisdomResponse>, AppError> {
    let request = params.into_request();
    let response = state.wisdom_service.query(&request)?;
    Ok(Json(response))
}

/// POST /api/v1/wisdom
/// Create a new wisdom entry
pub async fn create_wisdom(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateWisdomRequest>,
) -> Result<Json<WisdomEntry>, AppError> {
    let entry = state.wisdom_service.add(request)?;
    Ok(Json(entry))
}

/// GET /api/v1/wisdom/:id
/// Get a specific wisdom entry
pub async fn get_wisdom(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<WisdomEntry>, AppError> {
    let entry = state
        .wisdom_service
        .get(&id)?
        .ok_or_else(|| AppError::NotFound(format!("Wisdom entry not found: {}", id)))?;
    Ok(Json(entry))
}

/// DELETE /api/v1/wisdom/:id
/// Delete a wisdom entry
pub async fn delete_wisdom(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<DeleteResponse>, AppError> {
    let deleted = state.wisdom_service.delete(&id)?;
    Ok(Json(DeleteResponse { deleted }))
}

/// GET /api/v1/wisdom/categories
/// Get category summaries
pub async fn list_categories(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<CategorySummary>>, AppError> {
    let categories = state.wisdom_service.categories()?;
    Ok(Json(categories))
}

/// GET /api/v1/wisdom/categories/:category
/// Get wisdom entries by category
pub async fn get_by_category(
    State(state): State<Arc<AppState>>,
    Path(category): Path<String>,
) -> Result<Json<Vec<WisdomEntry>>, AppError> {
    let category = WisdomCategory::from_str(&category)
        .ok_or_else(|| AppError::BadRequest(format!("Invalid category: {}", category)))?;
    let entries = state.wisdom_service.by_category(category)?;
    Ok(Json(entries))
}

/// GET /api/v1/wisdom/search?q=
/// Search wisdom entries
pub async fn search_wisdom(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchParams>,
) -> Result<Json<SearchResponse>, AppError> {
    let limit = params.limit.unwrap_or(20).min(100);
    let entries = state.wisdom_service.search(&params.q, limit)?;
    Ok(Json(SearchResponse {
        entries,
        query: params.q,
    }))
}

/// Query parameters for listing wisdom
#[derive(Debug, Deserialize)]
pub struct WisdomQueryParams {
    pub category: Option<String>,
    pub tags: Option<String>,
    pub query: Option<String>,
    pub min_confidence: Option<f32>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl WisdomQueryParams {
    pub fn into_request(self) -> QueryWisdomRequest {
        let tags = self
            .tags
            .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default();

        let category = self.category.and_then(|c| WisdomCategory::from_str(&c));

        QueryWisdomRequest {
            category,
            tags,
            query: self.query,
            min_confidence: self.min_confidence,
            limit: self.limit,
            offset: self.offset,
        }
    }
}

/// Query parameters for search
#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: String,
    pub limit: Option<usize>,
}

/// Search response
#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub entries: Vec<WisdomEntry>,
    pub query: String,
}

/// Delete response
#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub deleted: bool,
}
