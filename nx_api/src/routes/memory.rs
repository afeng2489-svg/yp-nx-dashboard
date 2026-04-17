//! Memory 路由 - 团队对话记忆 API
//!
//! API Endpoints:
//! - POST   /api/v1/teams/{team_id}/memories          存储记忆
//! - POST   /api/v1/teams/{team_id}/memories/search   搜索记忆
//! - GET    /api/v1/teams/{team_id}/memories/stats    获取统计
//! - DELETE /api/v1/teams/{team_id}/memories          清空记忆

use std::sync::Arc;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};

use crate::routes::AppState;
use nx_memory::{
    MemoryStore, MemorySearch, MemoryChunk, Transcript,
    SearchRequest, StoreRequest, EmbeddingProvider,
};

/// Memory 状态
pub struct MemoryState {
    pub store: Arc<MemoryStore>,
    pub search: Arc<MemorySearch>,
}

/// 创建 Memory 状态
pub fn create_memory_state(db_path: &str, embedding_provider: Option<Arc<dyn EmbeddingProvider>>) -> MemoryState {
    let store = Arc::new(
        MemoryStore::new(db_path).expect("Failed to create memory store")
    );

    let search = if let Some(provider) = embedding_provider {
        Arc::new(MemorySearch::with_embedding_provider(store.clone(), provider))
    } else {
        Arc::new(MemorySearch::new(store.clone()))
    };

    MemoryState { store, search }
}

// ─────────────────────────────────────────────────────────────────────────────
// API Handlers (use AppState)
// ─────────────────────────────────────────────────────────────────────────────

/// 存储记忆
pub async fn store_memory(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<String>,
    Json(request): Json<StoreRequest>,
) -> impl IntoResponse {
    let memory_state = &state.memory_state;

    // 1. 创建 Transcript
    let mut transcript = Transcript::new(&team_id, &request.user_id, request.role, &request.content);

    if let Some(session_id) = &request.session_id {
        transcript = transcript.with_session(session_id);
    }

    if let Some(user_name) = &request.user_name {
        transcript.metadata.user_name = Some(user_name.clone());
    }

    // 2. 存储 Transcript
    if let Err(e) = memory_state.store.store_transcript(&transcript) {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("存储失败: {}", e)));
    }

    // 3. 创建 MemoryChunk（直接使用完整内容，不分块）
    let chunk = MemoryChunk::from_transcript(&transcript, transcript.content.clone(), 0);

    // 4. 存储 Chunk
    if let Err(e) = memory_state.store.store_chunk(&chunk) {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("存储块失败: {}", e)));
    }

    // 5. 索引（生成向量）
    let mut metadata = serde_json::to_value(&transcript.metadata).unwrap_or_default();
    if let serde_json::Value::Object(ref mut map) = metadata {
        map.insert("transcript_id".to_string(), serde_json::Value::String(transcript.id.clone()));
        map.insert("created_at".to_string(), serde_json::Value::String(transcript.created_at.to_rfc3339()));
    }
    if let Err(e) = memory_state.search.index_chunk(&team_id, &chunk.id, &chunk.content, metadata).await {
        tracing::warn!("索引失败（可能无 embedding provider）: {}", e);
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "transcript_id": transcript.id,
        "chunk_id": chunk.id,
    })))
}

/// 搜索记忆
pub async fn search_memory(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<String>,
    Json(request): Json<SearchRequest>,
) -> impl IntoResponse {
    let memory_state = &state.memory_state;

    // 确保团队索引已初始化
    if memory_state.search.get_index_stats(&team_id).is_none() {
        if let Err(e) = memory_state.search.init_team_index(&team_id) {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("索引初始化失败: {}", e)));
        }
    }

    // 使用 URL 路径中的 team_id 创建新的搜索请求
    let search_request = SearchRequest {
        team_id: Some(team_id.clone()),
        ..request
    };

    // 生成查询向量（异步，用于向量重排序）
    let query_embedding = memory_state.search.embed_query(&search_request.query).await;

    match memory_state.search.search(&search_request, query_embedding.as_deref()) {
        Ok(response) => Ok(Json(response)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, format!("搜索失败: {}", e))),
    }
}

/// 获取统计信息
pub async fn get_stats(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<String>,
) -> impl IntoResponse {
    let memory_state = &state.memory_state;

    match memory_state.store.get_team_stats(&team_id) {
        Ok(stats) => Ok(Json(stats)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, format!("获取统计失败: {}", e))),
    }
}

/// 清空团队记忆
pub async fn clear_memory(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<String>,
) -> impl IntoResponse {
    let memory_state = &state.memory_state;

    // 清空存储
    if let Err(e) = memory_state.store.clear_team(&team_id) {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("清空失败: {}", e)));
    }

    // 清空索引
    memory_state.search.clear_team_index(&team_id);

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("团队 {} 的记忆已清空", team_id),
    })))
}
