//! 测试生成路由

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

use crate::routes::AppState;
use crate::services::{GenerateTestsRequest, Language, TestFramework};

/// 生成测试
pub async fn generate_tests(
    State(state): State<Arc<AppState>>,
    Json(request): Json<GenerateTestsRequest>,
) -> Result<Json<crate::services::GenerateTestsResponse>, (StatusCode, String)> {
    tracing::info!("生成测试: language={:?}", request.language);

    state
        .test_generator
        .generate_tests(request)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("测试生成失败: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })
}

/// 生成单元测试
pub async fn generate_unit_tests(
    State(state): State<Arc<AppState>>,
    Json(request): Json<UnitTestRequest>,
) -> Result<Json<crate::services::GenerateTestsResponse>, (StatusCode, String)> {
    tracing::info!("生成单元测试: language={:?}", request.language);

    state
        .test_generator
        .generate_unit_tests(&request.source_code, request.language)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("单元测试生成失败: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })
}

/// 生成集成测试
pub async fn generate_integration_tests(
    State(state): State<Arc<AppState>>,
    Json(request): Json<IntegrationTestRequest>,
) -> Result<Json<crate::services::GenerateTestsResponse>, (StatusCode, String)> {
    tracing::info!("生成集成测试: language={:?}", request.language);

    state
        .test_generator
        .generate_integration_tests(&request.source_code, request.language)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("集成测试生成失败: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })
}

// ============ 请求类型 ============

#[derive(Debug, serde::Deserialize)]
pub struct UnitTestRequest {
    pub source_code: String,
    pub language: Language,
}

#[derive(Debug, serde::Deserialize)]
pub struct IntegrationTestRequest {
    pub source_code: String,
    pub language: Language,
}
