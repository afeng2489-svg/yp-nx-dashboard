//! 知识库 API 路由

use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::AppState;
use crate::services::knowledge::kb_search::SearchResult as KbSearchResult;

type ApiResponse<T> = Result<Json<T>, KnowledgeApiError>;

/// 创建知识库路由
pub fn knowledge_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(create_knowledge_base))
        .route("/", get(list_knowledge_bases))
        .route("/:id", delete(delete_knowledge_base))
        .route("/upload", post(upload_document))
        .route("/:kb_id/documents", get(list_documents))
        .route("/:kb_id/documents/:doc_id", delete(delete_document))
        .route("/search", post(search_knowledge_base))
}

// ── 请求类型 ──

#[derive(Debug, Deserialize)]
pub struct CreateKnowledgeBaseRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_provider")]
    pub embedding_provider: String,
    #[serde(default = "default_model")]
    pub embedding_model: String,
    #[serde(default = "default_dimension")]
    pub embedding_dimension: i64,
    #[serde(default = "default_chunk_size")]
    pub chunk_size: i64,
}

fn default_provider() -> String {
    "openai".to_string()
}
fn default_model() -> String {
    "text-embedding-3-small".to_string()
}
fn default_dimension() -> i64 {
    1536
}
fn default_chunk_size() -> i64 {
    500
}

#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub kb_id: String,
    pub query: String,
    #[serde(default = "default_top_k")]
    pub top_k: usize,
    #[serde(default = "default_threshold")]
    pub threshold: f32,
}

fn default_top_k() -> usize {
    5
}
fn default_threshold() -> f32 {
    0.5
}

// ── 响应类型 ──

#[derive(Debug, Serialize)]
pub struct KbSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub embedding_provider: String,
    pub embedding_model: String,
    pub document_count: i64,
    pub chunk_count: i64,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct DocumentSummary {
    pub id: String,
    pub filename: String,
    pub content_type: String,
    pub file_size: i64,
    pub chunk_count: i64,
    pub status: String,
    pub error: Option<String>,
    pub created_at: String,
}

// ── 错误类型 ──

#[derive(Debug)]
pub struct KnowledgeApiError {
    pub status: StatusCode,
    pub message: String,
}

impl IntoResponse for KnowledgeApiError {
    fn into_response(self) -> Response {
        let body = Json(serde_json::json!({
            "error": self.message
        }));
        (self.status, body).into_response()
    }
}

impl From<crate::services::knowledge::KnowledgeError> for KnowledgeApiError {
    fn from(err: crate::services::knowledge::KnowledgeError) -> Self {
        match err {
            crate::services::knowledge::KnowledgeError::NotFound(id) => KnowledgeApiError {
                status: StatusCode::NOT_FOUND,
                message: format!("知识库不存在: {}", id),
            },
            crate::services::knowledge::KnowledgeError::EmptyDocument => KnowledgeApiError {
                status: StatusCode::BAD_REQUEST,
                message: "文档为空".to_string(),
            },
            _ => KnowledgeApiError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: err.to_string(),
            },
        }
    }
}

// ── Handlers ──

/// POST /api/v1/knowledge-bases — 创建知识库
pub async fn create_knowledge_base(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateKnowledgeBaseRequest>,
) -> ApiResponse<serde_json::Value> {
    let kb = state
        .knowledge_service
        .create_knowledge_base(
            req.name,
            req.description,
            req.embedding_provider,
            req.embedding_model,
            req.embedding_dimension,
            req.chunk_size,
        )
        .map_err(KnowledgeApiError::from)?;
    Ok(Json(serde_json::json!({
        "id": kb.id,
        "name": kb.name,
    })))
}

/// GET /api/v1/knowledge-bases — 列出知识库
pub async fn list_knowledge_bases(
    State(state): State<Arc<AppState>>,
) -> ApiResponse<Vec<KbSummary>> {
    let kbs = state
        .knowledge_service
        .list_knowledge_bases()
        .map_err(KnowledgeApiError::from)?;

    let summaries: Vec<KbSummary> = kbs
        .into_iter()
        .map(|kb| KbSummary {
            id: kb.id,
            name: kb.name,
            description: kb.description,
            embedding_provider: kb.embedding_provider,
            embedding_model: kb.embedding_model,
            document_count: 0,
            chunk_count: 0,
            created_at: kb.created_at,
        })
        .collect();

    Ok(Json(summaries))
}

/// DELETE /api/v1/knowledge-bases/:id — 删除知识库
pub async fn delete_knowledge_base(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ApiResponse<serde_json::Value> {
    state
        .knowledge_service
        .delete_knowledge_base(&id)
        .map_err(KnowledgeApiError::from)?;
    Ok(Json(serde_json::json!({"ok": true})))
}

/// POST /api/v1/knowledge-bases/upload — 上传文档（multipart）
/// kb_id 从 multipart form field "kb_id" 传入
pub async fn upload_document(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> ApiResponse<DocumentSummary> {
    let (kb_id, filename, content) =
        extract_multipart_upload(&mut multipart)
            .await
            .map_err(|e| KnowledgeApiError {
                status: StatusCode::BAD_REQUEST,
                message: e,
            })?;

    let doc = state
        .knowledge_service
        .upload_document(&kb_id, filename, content)
        .await
        .map_err(KnowledgeApiError::from)?;

    Ok(Json(DocumentSummary {
        id: doc.id,
        filename: doc.filename,
        content_type: doc.content_type,
        file_size: doc.file_size,
        chunk_count: doc.chunk_count,
        status: doc.status,
        error: doc.error,
        created_at: doc.created_at,
    }))
}

/// GET /api/v1/knowledge-bases/:kb_id/documents — 列出文档
pub async fn list_documents(
    State(state): State<Arc<AppState>>,
    Path(kb_id): Path<String>,
) -> ApiResponse<Vec<DocumentSummary>> {
    let docs = state
        .knowledge_service
        .list_documents(&kb_id)
        .map_err(KnowledgeApiError::from)?;

    let summaries: Vec<DocumentSummary> = docs
        .into_iter()
        .map(|d| DocumentSummary {
            id: d.id,
            filename: d.filename,
            content_type: d.content_type,
            file_size: d.file_size,
            chunk_count: d.chunk_count,
            status: d.status,
            error: d.error,
            created_at: d.created_at,
        })
        .collect();

    Ok(Json(summaries))
}

/// DELETE /api/v1/knowledge-bases/:kb_id/documents/:doc_id — 删除文档
pub async fn delete_document(
    State(state): State<Arc<AppState>>,
    Path((_kb_id, doc_id)): Path<(String, String)>,
) -> ApiResponse<serde_json::Value> {
    state
        .knowledge_service
        .delete_document(&doc_id)
        .map_err(KnowledgeApiError::from)?;
    Ok(Json(serde_json::json!({"ok": true})))
}

/// POST /api/v1/knowledge-bases/search — 检索
pub async fn search_knowledge_base(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SearchRequest>,
) -> ApiResponse<Vec<KbSearchResult>> {
    let results = state
        .knowledge_service
        .search(&req.kb_id, &req.query, req.top_k, req.threshold)
        .await
        .map_err(KnowledgeApiError::from)?;
    Ok(Json(results))
}

// ── 辅助 ──

/// 从 multipart 提取 kb_id + filename + content
async fn extract_multipart_upload(
    multipart: &mut Multipart,
) -> Result<(String, String, String), String> {
    let mut kb_id = String::new();
    let mut filename = String::from("upload.txt");
    let mut content = String::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| format!("Multipart error: {}", e))?
    {
        let name = field.name().unwrap_or("").to_string();
        let fname = field.file_name().map(|s| s.to_string());

        match name.as_str() {
            "kb_id" => {
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|e| format!("读取 kb_id 失败: {}", e))?;
                kb_id = String::from_utf8_lossy(&bytes).to_string();
            }
            "file" | "content" => {
                if let Some(f) = fname {
                    filename = f;
                }
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|e| format!("读取文件失败: {}", e))?;
                content = String::from_utf8_lossy(&bytes).to_string();
            }
            _ => {}
        }
    }

    if kb_id.is_empty() {
        return Err("缺少 kb_id 字段".to_string());
    }

    Ok((kb_id, filename, content))
}
