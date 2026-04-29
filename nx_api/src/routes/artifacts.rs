//! 产物 API 路由
//!
//! 前端"产物面板"的数据源：
//! 1. GET /api/v1/executions/:id/artifacts — 列出所有产物（可选 ?stage= 过滤）
//! 2. GET /api/v1/executions/:id/artifacts/summary — 按 stage 汇总变更统计
//! 3. GET /api/v1/executions/:id/artifacts/file — 按路径查询最新记录

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::routes::AppState;

#[derive(Deserialize)]
pub struct ArtifactsQuery {
    pub stage: Option<String>,
}

#[derive(Deserialize)]
pub struct FileQuery {
    pub path: String,
}

#[derive(serde::Serialize)]
pub struct ArtifactSummary {
    pub stage_name: Option<String>,
    pub added: usize,
    pub modified: usize,
    pub deleted: usize,
}

/// GET /api/v1/executions/:id/artifacts
pub async fn list_artifacts(
    State(state): State<Arc<AppState>>,
    Path(execution_id): Path<String>,
    Query(query): Query<ArtifactsQuery>,
) -> Json<serde_json::Value> {
    let Some(repo) = &state.artifact_repo else {
        return Json(serde_json::json!({ "ok": false, "error": "产物仓库未启用" }));
    };

    match repo.list_by_execution(&execution_id) {
        Ok(records) => {
            let filtered: Vec<_> = if let Some(ref stage) = query.stage {
                records
                    .into_iter()
                    .filter(|r| r.stage_name.as_deref() == Some(stage.as_str()))
                    .collect()
            } else {
                records
            };
            Json(serde_json::json!({ "ok": true, "data": filtered }))
        }
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
    }
}

/// GET /api/v1/executions/:id/artifacts/summary
pub async fn artifacts_summary(
    State(state): State<Arc<AppState>>,
    Path(execution_id): Path<String>,
) -> Json<serde_json::Value> {
    let Some(repo) = &state.artifact_repo else {
        return Json(serde_json::json!({ "ok": false, "error": "产物仓库未启用" }));
    };

    match repo.list_by_execution(&execution_id) {
        Ok(records) => {
            let mut grouped: BTreeMap<Option<String>, (usize, usize, usize)> = BTreeMap::new();
            for r in &records {
                let entry = grouped.entry(r.stage_name.clone()).or_default();
                match r.change_type.as_str() {
                    "added" => entry.0 += 1,
                    "modified" => entry.1 += 1,
                    "deleted" => entry.2 += 1,
                    _ => {}
                }
            }
            let summaries: Vec<ArtifactSummary> = grouped
                .into_iter()
                .map(|(stage, (added, modified, deleted))| ArtifactSummary {
                    stage_name: stage,
                    added,
                    modified,
                    deleted,
                })
                .collect();
            Json(serde_json::json!({ "ok": true, "data": summaries }))
        }
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
    }
}

/// GET /api/v1/executions/:id/artifacts/file?path=...
pub async fn get_artifact_by_path(
    State(state): State<Arc<AppState>>,
    Path(execution_id): Path<String>,
    Query(query): Query<FileQuery>,
) -> Json<serde_json::Value> {
    let Some(repo) = &state.artifact_repo else {
        return Json(serde_json::json!({ "ok": false, "error": "产物仓库未启用" }));
    };

    match repo.find_by_path(&execution_id, &query.path) {
        Ok(Some(record)) => Json(serde_json::json!({ "ok": true, "data": record })),
        Ok(None) => Json(serde_json::json!({ "ok": true, "data": null })),
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
    }
}
