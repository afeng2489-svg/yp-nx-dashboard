//! AF-10b 工厂台附件：multipart 上传到工作区 `.nx/factory-attachments/`

use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    routing::post,
    Json, Router,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::routes::AppState;
use crate::services::workspace_service::WorkspaceServiceError;

const MAX_BYTES: usize = 10 * 1024 * 1024;
const TEXT_EXCERPT_LIMIT: usize = 8000;

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/api/v1/factory/attachments", post(upload_attachment))
}

#[derive(serde::Serialize)]
pub struct UploadResponse {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<AttachmentData>,
}

#[derive(serde::Serialize)]
struct AttachmentData {
    path: String,
    relative_path: String,
    filename: String,
    size: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    text_excerpt: Option<String>,
}

pub async fn upload_attachment(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, StatusCode> {
    let mut workspace_id: Option<String> = None;
    let mut filename = String::from("attachment.bin");
    let mut bytes: Vec<u8> = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
    {
        match field.name().unwrap_or("") {
            "workspace_id" => {
                workspace_id = Some(
                    field
                        .text()
                        .await
                        .map_err(|_| StatusCode::BAD_REQUEST)?
                        .trim()
                        .to_string(),
                );
            }
            "file" => {
                if let Some(name) = field.file_name() {
                    filename = sanitize_filename(name);
                }
                bytes = field
                    .bytes()
                    .await
                    .map_err(|_| StatusCode::BAD_REQUEST)?
                    .to_vec();
            }
            _ => {}
        }
    }

    if bytes.is_empty() {
        return Ok(Json(UploadResponse {
            ok: false,
            error: Some("未收到文件".to_string()),
            data: None,
        }));
    }
    if bytes.len() > MAX_BYTES {
        return Ok(Json(UploadResponse {
            ok: false,
            error: Some(format!("文件超过 {}MB 限制", MAX_BYTES / 1024 / 1024)),
            data: None,
        }));
    }

    let root = match resolve_workspace_root(&state, workspace_id.as_deref()) {
        Ok(r) => r,
        Err(msg) => {
            return Ok(Json(UploadResponse {
                ok: false,
                error: Some(msg),
                data: None,
            }));
        }
    };

    let stored_name = format!("{}_{}", Uuid::new_v4(), filename);
    let relative_path = format!(".nx/factory-attachments/{stored_name}");

    let attach_dir = std::path::Path::new(&root).join(".nx/factory-attachments");
    if let Err(e) = std::fs::create_dir_all(&attach_dir) {
        return Ok(Json(UploadResponse {
            ok: false,
            error: Some(format!("创建附件目录失败: {e}")),
            data: None,
        }));
    }

    let full_path = match state
        .workspace_service
        .resolve_relative_path(&root, &relative_path)
    {
        Ok(p) => p,
        Err(e) => {
            return Ok(Json(UploadResponse {
                ok: false,
                error: Some(workspace_err_msg(&e)),
                data: None,
            }));
        }
    };

    if let Err(e) = std::fs::write(&full_path, &bytes) {
        return Ok(Json(UploadResponse {
            ok: false,
            error: Some(format!("写入附件失败: {e}")),
            data: None,
        }));
    }

    let text_excerpt = text_excerpt_if_applicable(&filename, &bytes);
    let path = full_path.to_string_lossy().to_string();

    Ok(Json(UploadResponse {
        ok: true,
        error: None,
        data: Some(AttachmentData {
            path,
            relative_path,
            filename,
            size: bytes.len(),
            text_excerpt,
        }),
    }))
}

fn resolve_workspace_root(state: &AppState, workspace_id: Option<&str>) -> Result<String, String> {
    if let Some(wid) = workspace_id {
        if let Ok(Some(ws)) = state.workspace_service.get_workspace(wid) {
            if let Some(root) = ws.root_path.filter(|r| !r.trim().is_empty()) {
                return Ok(root);
            }
        }
    }
    state
        .current_workspace_path
        .read()
        .clone()
        .filter(|r| !r.trim().is_empty())
        .ok_or_else(|| "请先在顶栏选择工作区".to_string())
}

fn sanitize_filename(name: &str) -> String {
    let base = std::path::Path::new(name)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("attachment.bin");
    base.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn text_excerpt_if_applicable(filename: &str, bytes: &[u8]) -> Option<String> {
    let lower = filename.to_lowercase();
    let text_ext = [
        ".txt", ".md", ".json", ".yaml", ".yml", ".toml", ".csv", ".ts", ".tsx", ".js", ".jsx",
        ".rs", ".py", ".go", ".sql", ".html", ".css", ".xml",
    ];
    if !text_ext.iter().any(|ext| lower.ends_with(ext)) {
        return None;
    }
    let text = String::from_utf8_lossy(bytes);
    if text.trim().is_empty() {
        return None;
    }
    let excerpt = if text.len() > TEXT_EXCERPT_LIMIT {
        format!("{}…", &text[..TEXT_EXCERPT_LIMIT])
    } else {
        text.to_string()
    };
    Some(excerpt)
}

fn workspace_err_msg(e: &WorkspaceServiceError) -> String {
    match e {
        WorkspaceServiceError::NotFound(id) => format!("工作区不存在: {id}"),
        WorkspaceServiceError::FileError(msg) => msg.clone(),
        other => other.to_string(),
    }
}
