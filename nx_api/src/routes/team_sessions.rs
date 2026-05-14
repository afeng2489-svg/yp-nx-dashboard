//! 团队会话路由 — TeamSession 数据查询 API
//!
//! 独立的子路由器，读取 nexus-orchestrator SessionStore 的 SQLite 数据，
//! 通过标准 ApiResponse 信封暴露给前端。

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get},
    Json, Router,
};
use nexus_orchestrator::session::ChainResult;
use nexus_orchestrator::session_store::{SessionStore, SessionSummary};
use std::path::PathBuf;
use std::sync::Arc;

use crate::response::{ok, ApiErrorResponse, ApiOk};

/// 团队会话应用状态
#[derive(Clone)]
struct TeamSessionState {
    store: Arc<SessionStore>,
}

/// 解析 team_sessions.db 路径（与 CLI 保持一致：~/.nx/team_sessions.db）
fn resolve_db_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let path = PathBuf::from(home).join(".nx").join("team_sessions.db");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    path
}

/// 创建团队会话路由器（独立于 AppState，使用默认路径 ~/.nx/team_sessions.db）
pub fn create_router() -> Router {
    let path = resolve_db_path();
    let store = SessionStore::open(&path).expect("无法打开团队会话数据库");
    create_router_with_store(store)
}

/// 使用已有 SessionStore 创建路由器（测试用）
pub fn create_router_with_store(store: SessionStore) -> Router {
    let state = TeamSessionState {
        store: Arc::new(store),
    };

    Router::new()
        .route("/", get(list_team_sessions))
        .route("/:id", get(get_team_session))
        .route("/:id", delete(delete_team_session))
        .with_state(state)
}

/// GET /api/v1/team-sessions
async fn list_team_sessions(
    State(state): State<TeamSessionState>,
) -> Result<ApiOk<Vec<SessionSummary>>, ApiErrorResponse> {
    state
        .store
        .list(50)
        .map(ok)
        .map_err(|e| ApiErrorResponse::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// DELETE /api/v1/team-sessions/:id
async fn delete_team_session(
    State(state): State<TeamSessionState>,
    Path(id): Path<String>,
) -> Result<ApiOk<serde_json::Value>, ApiErrorResponse> {
    match state.store.delete(&id) {
        Ok(true) => Ok(ok(serde_json::json!({"deleted": id}))),
        Ok(false) => Err(ApiErrorResponse::new(
            StatusCode::NOT_FOUND,
            format!("团队会话 {} 不存在", id),
        )),
        Err(e) => Err(ApiErrorResponse::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            e.to_string(),
        )),
    }
}

/// GET /api/v1/team-sessions/:id
async fn get_team_session(
    State(state): State<TeamSessionState>,
    Path(id): Path<String>,
) -> Result<ApiOk<ChainResult>, ApiErrorResponse> {
    match state.store.get(&id) {
        Ok(Some(result)) => Ok(ok(result)),
        Ok(None) => Err(ApiErrorResponse::new(
            StatusCode::NOT_FOUND,
            format!("团队会话 {} 不存在", id),
        )),
        Err(e) => Err(ApiErrorResponse::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            e.to_string(),
        )),
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::Path;
    use nexus_orchestrator::session::{AgentResult, ChainResult};
    use nexus_orchestrator::team::{AgentId, AgentRole};
    use uuid::Uuid;

    // ── Fixtures ─────────────────────────────────────────────────────

    fn make_result_with_task(task: &str, agent_count: usize) -> ChainResult {
        let mut agents = Vec::new();
        for i in 0..agent_count {
            agents.push(AgentResult {
                agent_id: AgentId::new(),
                role: match i % 4 {
                    0 => AgentRole::Architect,
                    1 => AgentRole::Developer,
                    2 => AgentRole::Reviewer,
                    _ => AgentRole::Tester,
                },
                agent_name: format!("agent-{}", i),
                text: format!("result from agent {}", i),
                duration_ms: (i as u64 + 1) * 1000,
                attempts: 1,
            });
        }
        let total = agents.iter().map(|a| a.duration_ms).sum();
        ChainResult {
            execution_id: Uuid::new_v4(),
            task: task.into(),
            agent_results: agents,
            total_duration_ms: total,
        }
    }

    fn make_default_result() -> ChainResult {
        make_result_with_task("default task", 2)
    }

    fn setup_state() -> TeamSessionState {
        let store = SessionStore::in_memory().expect("in-memory store");
        TeamSessionState {
            store: Arc::new(store),
        }
    }

    fn setup_state_with_data() -> (TeamSessionState, Vec<ChainResult>) {
        let state = setup_state();
        let results: Vec<ChainResult> = (0..3)
            .map(|i| make_result_with_task(&format!("task-{}", i), i + 1))
            .collect();
        for r in &results {
            state.store.save(r).unwrap();
        }
        (state, results)
    }

    // ── list_team_sessions ───────────────────────────────────────────

    #[tokio::test]
    async fn list_empty_when_no_sessions() {
        let state = setup_state();
        let result = list_team_sessions(State(state)).await.unwrap();
        let data = result.0.data.unwrap();
        assert!(data.is_empty());
    }

    #[tokio::test]
    async fn list_returns_saved_sessions() {
        let (state, results) = setup_state_with_data();
        let result = list_team_sessions(State(state)).await.unwrap();
        let data = result.0.data.unwrap();

        assert_eq!(data.len(), 3);
        // All tasks present (order may vary due to same-second timestamps)
        let tasks: Vec<&str> = data.iter().map(|s| s.task.as_str()).collect();
        assert!(tasks.contains(&"task-0"));
        assert!(tasks.contains(&"task-1"));
        assert!(tasks.contains(&"task-2"));
        assert!(data.iter().all(|s| s.duration_ms > 0));
        assert!(data.iter().all(|s| !s.created_at.is_empty()));
    }

    #[tokio::test]
    async fn list_returns_correct_agent_counts() {
        let (state, _) = setup_state_with_data();
        let result = list_team_sessions(State(state)).await.unwrap();
        let data = result.0.data.unwrap();

        let counts: Vec<usize> = data.iter().map(|s| s.agent_count).collect();
        assert!(counts.contains(&1));
        assert!(counts.contains(&2));
        assert!(counts.contains(&3));
    }

    #[tokio::test]
    async fn list_handles_large_number_of_sessions() {
        let state = setup_state();
        for i in 0..100 {
            let r = make_result_with_task(&format!("bulk-{}", i), 1);
            state.store.save(&r).unwrap();
        }
        let result = list_team_sessions(State(state)).await.unwrap();
        // API hardcodes limit=50
        assert_eq!(result.0.data.unwrap().len(), 50);
    }

    #[tokio::test]
    async fn list_status_field_is_populated() {
        let (state, _) = setup_state_with_data();
        let result = list_team_sessions(State(state)).await.unwrap();
        for s in result.0.data.unwrap() {
            assert_eq!(s.status, "completed");
        }
    }

    // ── get_team_session ─────────────────────────────────────────────

    #[tokio::test]
    async fn get_returns_session_by_id() {
        let (state, results) = setup_state_with_data();
        let id = results[0].execution_id.to_string();

        let result = get_team_session(State(state), Path(id)).await.unwrap();
        let data = result.0.data.unwrap();

        assert_eq!(data.task, "task-0");
        assert_eq!(data.agent_results.len(), 1);
        assert!(data.total_duration_ms > 0);
    }

    #[tokio::test]
    async fn get_returns_404_for_nonexistent_id() {
        let state = setup_state();
        let err = get_team_session(State(state), Path("nonexistent".into()))
            .await
            .unwrap_err();
        assert_eq!(err.status, StatusCode::NOT_FOUND);
        assert!(err.message.contains("不存在"));
    }

    #[tokio::test]
    async fn get_returns_404_for_empty_string_id() {
        let state = setup_state();
        let err = get_team_session(State(state), Path(String::new()))
            .await
            .unwrap_err();
        assert_eq!(err.status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn get_returns_detailed_agent_results() {
        let (state, results) = setup_state_with_data();
        let id = results[2].execution_id.to_string(); // task-2 has 3 agents

        let result = get_team_session(State(state), Path(id)).await.unwrap();
        let data = result.0.data.unwrap();

        assert_eq!(data.agent_results.len(), 3);
        assert_eq!(data.agent_results[0].role, AgentRole::Architect);
        assert_eq!(data.agent_results[1].role, AgentRole::Developer);
        assert_eq!(data.agent_results[2].role, AgentRole::Reviewer);
    }

    #[tokio::test]
    async fn get_returns_result_with_attempts_info() {
        let state = setup_state();
        let mut r = make_default_result();
        r.agent_results[0].attempts = 3;
        r.agent_results[0].text = "retried output".to_string();
        state.store.save(&r).unwrap();
        let id = r.execution_id.to_string();

        let result = get_team_session(State(state), Path(id)).await.unwrap();
        let agent = &result.0.data.unwrap().agent_results[0];
        assert_eq!(agent.attempts, 3);
        assert_eq!(agent.text, "retried output");
    }

    // ── delete_team_session ──────────────────────────────────────────

    #[tokio::test]
    async fn delete_existing_session() {
        let state = setup_state();
        let r = make_default_result();
        state.store.save(&r).unwrap();
        let id = r.execution_id.to_string();

        let result = delete_team_session(State(state), Path(id)).await.unwrap();
        let json = result.0.data.unwrap();
        assert_eq!(json["deleted"], r.execution_id.to_string());
    }

    #[tokio::test]
    async fn delete_nonexistent_returns_404() {
        let state = setup_state();
        let err = delete_team_session(State(state), Path("nonexistent".into()))
            .await
            .unwrap_err();
        assert_eq!(err.status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn delete_removes_session_from_list() {
        let state = setup_state();
        let r = make_default_result();
        state.store.save(&r).unwrap();
        let id = r.execution_id.to_string();

        // Delete
        delete_team_session(State(state.clone()), Path(id.clone()))
            .await
            .unwrap();

        // Verify list is now empty
        let list = list_team_sessions(State(state)).await.unwrap();
        assert!(list.0.data.unwrap().is_empty());
    }

    #[tokio::test]
    async fn delete_twice_returns_404_second_time() {
        let state = setup_state();
        let r = make_default_result();
        state.store.save(&r).unwrap();
        let id = r.execution_id.to_string();

        // First delete succeeds
        delete_team_session(State(state.clone()), Path(id.clone()))
            .await
            .unwrap();

        // Second delete returns 404
        let err = delete_team_session(State(state), Path(id))
            .await
            .unwrap_err();
        assert_eq!(err.status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn delete_empty_string_id_returns_404() {
        let state = setup_state();
        let err = delete_team_session(State(state), Path(String::new()))
            .await
            .unwrap_err();
        assert_eq!(err.status, StatusCode::NOT_FOUND);
    }
}
