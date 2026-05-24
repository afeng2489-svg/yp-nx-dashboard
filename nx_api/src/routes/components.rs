//! Project Component Catalog Routes
//!
//! CRUD API for project_components table with direct rusqlite access.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::AppState;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectComponent {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub component_type: String,
    pub file_path: String,
    pub description: Option<String>,
    pub props_json: Option<String>,
    pub source_code: Option<String>,
    pub is_platform_component: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateComponentRequest {
    pub name: String,
    pub component_type: String,
    pub file_path: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub props_json: Option<String>,
    #[serde(default)]
    pub source_code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListComponentsQuery {
    #[serde(rename = "type")]
    pub component_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchComponentsQuery {
    pub q: Option<String>,
}

fn component_from_row(row: &rusqlite::Row) -> rusqlite::Result<ProjectComponent> {
    Ok(ProjectComponent {
        id: row.get("id")?,
        project_id: row.get("project_id")?,
        name: row.get("name")?,
        component_type: row.get("component_type")?,
        file_path: row.get("file_path")?,
        description: row.get("description")?,
        props_json: row.get("props_json")?,
        source_code: row.get("source_code")?,
        is_platform_component: row.get::<_, i32>("is_platform_component")? != 0,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

/// GET /api/v1/projects/:project_id/components?type=component|hook|type|util
pub async fn list_components(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<String>,
    Query(query): Query<ListComponentsQuery>,
) -> Result<Json<Vec<ProjectComponent>>, (StatusCode, String)> {
    let conn = rusqlite::Connection::open(&state.db_path)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let (sql, params): (String, Vec<Box<dyn rusqlite::types::ToSql>>) =
        if let Some(ref ct) = query.component_type {
            (
                "SELECT * FROM project_components WHERE project_id = ?1 AND component_type = ?2 ORDER BY created_at DESC"
                    .to_string(),
                vec![Box::new(project_id), Box::new(ct.clone())],
            )
        } else {
            (
                "SELECT * FROM project_components WHERE project_id = ?1 ORDER BY created_at DESC"
                    .to_string(),
                vec![Box::new(project_id)],
            )
        };

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let rows = stmt
        .query_map(
            rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
            component_from_row,
        )
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut components = Vec::new();
    for row in rows {
        components.push(
            row.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
        );
    }
    Ok(Json(components))
}

/// POST /api/v1/projects/:project_id/components
pub async fn create_component(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<String>,
    Json(req): Json<CreateComponentRequest>,
) -> Result<(StatusCode, Json<ProjectComponent>), (StatusCode, String)> {
    let conn = rusqlite::Connection::open(&state.db_path)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO project_components (id, project_id, name, component_type, file_path, description, props_json, source_code, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        rusqlite::params![
            id,
            project_id,
            req.name,
            req.component_type,
            req.file_path,
            req.description,
            req.props_json,
            req.source_code,
            now,
            now,
        ],
    )
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("UNIQUE constraint") {
            (StatusCode::CONFLICT, format!("Component already exists: {}", msg))
        } else {
            (StatusCode::INTERNAL_SERVER_ERROR, msg)
        }
    })?;

    let component = conn
        .query_row(
            "SELECT * FROM project_components WHERE id = ?1",
            rusqlite::params![id],
            component_from_row,
        )
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(component)))
}

/// GET /api/v1/projects/:project_id/components/search?q=
pub async fn search_components(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<String>,
    Query(query): Query<SearchComponentsQuery>,
) -> Result<Json<Vec<ProjectComponent>>, (StatusCode, String)> {
    let conn = rusqlite::Connection::open(&state.db_path)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let q = query.q.unwrap_or_default();
    let pattern = format!("%{}%", q);

    let mut stmt = conn
        .prepare(
            "SELECT * FROM project_components WHERE project_id = ?1 AND (name LIKE ?2 OR description LIKE ?2) ORDER BY created_at DESC",
        )
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let rows = stmt
        .query_map(rusqlite::params![project_id, pattern], component_from_row)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut components = Vec::new();
    for row in rows {
        components.push(
            row.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
        );
    }
    Ok(Json(components))
}

/// GET /api/v1/projects/:project_id/components/:id
pub async fn get_component(
    State(state): State<Arc<AppState>>,
    Path((_project_id, id)): Path<(String, String)>,
) -> Result<Json<ProjectComponent>, (StatusCode, String)> {
    let conn = rusqlite::Connection::open(&state.db_path)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    conn.query_row(
        "SELECT * FROM project_components WHERE id = ?1",
        rusqlite::params![id],
        component_from_row,
    )
    .map(Json)
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            (StatusCode::NOT_FOUND, "Component not found".to_string())
        }
        e => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    })
}

/// DELETE /api/v1/projects/:project_id/components/:id
pub async fn delete_component(
    State(state): State<Arc<AppState>>,
    Path((_project_id, id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let conn = rusqlite::Connection::open(&state.db_path)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Check is_platform_component first
    let is_platform: bool = conn
        .query_row(
            "SELECT is_platform_component FROM project_components WHERE id = ?1",
            rusqlite::params![id],
            |row| row.get::<_, i32>(0),
        )
        .map(|v| v != 0)
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                (StatusCode::NOT_FOUND, "Component not found".to_string())
            }
            e => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        })?;

    if is_platform {
        return Err((
            StatusCode::FORBIDDEN,
            "Cannot delete platform components".to_string(),
        ));
    }

    conn.execute("DELETE FROM project_components WHERE id = ?1", rusqlite::params![id])
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}
