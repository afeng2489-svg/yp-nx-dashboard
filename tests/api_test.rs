//! API Integration Tests
//!
//! Tests for API endpoints and service interactions.

use std::sync::Arc;

// Re-export types from nx_api for testing
use nx_api::services::{ExecutionService, SessionService};
use nx_api::services::session_repository::SqliteSessionRepository;
use nx_api::services::session_service::{Session, SessionStatus};

#[tokio::test]
async fn test_session_lifecycle() {
    // Create in-memory repository
    let repo = SqliteSessionRepository::in_memory().unwrap();
    let service = SessionService::new(Arc::new(repo));

    // Create session
    let session = service.create_session("workflow-1".to_string()).await.unwrap();
    let session_id = session.id.clone();
    assert_eq!(session.status, SessionStatus::Pending);

    // Get session
    let found = service.get_session(&session_id).await.unwrap().unwrap();
    assert_eq!(found.id, session_id);

    // Update to Active
    service.update_status(&session_id, SessionStatus::Active).await.unwrap();
    let found = service.get_session(&session_id).await.unwrap().unwrap();
    assert_eq!(found.status, SessionStatus::Active);

    // Update to Paused
    service.update_status(&session_id, SessionStatus::Paused).await.unwrap();
    let found = service.get_session(&session_id).await.unwrap().unwrap();
    assert_eq!(found.status, SessionStatus::Paused);

    // List sessions
    let sessions = service.list_sessions().await.unwrap();
    assert_eq!(sessions.len(), 1);

    // Delete session
    let deleted = service.delete_session(&session_id).await.unwrap();
    assert!(deleted);

    let found = service.get_session(&session_id).await.unwrap();
    assert!(found.is_none());
}

#[tokio::test]
async fn test_session_resume_key() {
    let repo = SqliteSessionRepository::in_memory().unwrap();
    let service = SessionService::new(Arc::new(repo));

    let session = service.create_session("workflow-1".to_string()).await.unwrap();
    let resume_key = session.resume_key.clone().unwrap();

    // Get by resume key
    let found = service.get_session_by_resume_key(&resume_key).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, session.id);
}

#[tokio::test]
async fn test_session_cannot_resume_when_not_paused() {
    let mut session = Session::new("workflow-1".to_string());
    assert!(!session.can_resume());

    session.activate();
    assert!(!session.can_resume());

    session.pause();
    assert!(session.can_resume());

    session.resume();
    assert!(!session.can_resume()); // Already resumed, can't resume again
}

#[test]
fn test_execution_service_events() {
    let service = ExecutionService::new();
    let mut rx = service.subscribe();

    // Start execution
    let execution = service.start_execution("workflow-1".to_string(), serde_json::json!({}));
    let exec_id = execution.id.clone();

    // Should receive Started event
    let event = rx.try_recv();
    assert!(event.is_ok());

    // Update status
    service.update_status(&exec_id, ExecutionStatus::Completed);

    // Should receive StatusChanged event
    let event = rx.try_recv();
    assert!(event.is_ok());
}

#[test]
fn test_execution_service_multiple_executions() {
    let service = ExecutionService::new();

    let exec1 = service.start_execution("workflow-1".to_string(), serde_json::json!({}));
    let exec2 = service.start_execution("workflow-2".to_string(), serde_json::json!({}));

    let all = service.get_all_executions();
    assert_eq!(all.len(), 2);

    // Cancel one
    service.cancel_execution(&exec1.id);
    let remaining = service.get_all_executions();
    assert_eq!(remaining.len(), 2); // Still in list, just cancelled

    let found = service.get_execution(&exec1.id).unwrap();
    assert_eq!(found.status, ExecutionStatus::Cancelled);
}

#[test]
fn test_execution_service_stage_tracking() {
    let service = ExecutionService::new();
    let execution = service.start_execution("workflow-1".to_string(), serde_json::json!({}));

    // Add stage outputs
    service.add_stage_output(&execution.id, "stage-1".to_string(), serde_json::json!({"result": 1}));
    service.add_stage_output(&execution.id, "stage-2".to_string(), serde_json::json!({"result": 2}));

    let found = service.get_execution(&execution.id).unwrap();
    assert_eq!(found.stage_results.len(), 2);
    assert_eq!(found.stage_results[0].stage_name, "stage-1");
    assert_eq!(found.stage_results[1].stage_name, "stage-2");
}

#[test]
fn test_execution_service_broadcast() {
    use tokio::sync::broadcast;

    let service = ExecutionService::new();
    let mut rx1 = service.subscribe();
    let mut rx2 = service.subscribe();

    service.start_execution("workflow-1".to_string(), serde_json::json!({}));

    // Both receivers should get the event
    let timeout = std::time::Duration::from_millis(100);
    assert!(rx1.recv_timeout(timeout).is_ok());
    assert!(rx2.recv_timeout(timeout).is_ok());
}
