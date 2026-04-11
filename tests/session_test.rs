//! Session Integration Tests
//!
//! Tests for session management, lifecycle, and state transitions.

use std::sync::Arc;
use nx_api::services::SessionService;
use nx_api::services::session_repository::SqliteSessionRepository;
use nx_api::services::session_service::{Session, SessionStatus};

#[tokio::test]
async fn test_session_state_transitions() {
    let repo = SqliteSessionRepository::in_memory().unwrap();
    let service = SessionService::new(Arc::new(repo));

    // Create session - starts in Pending
    let session = service.create_session("workflow-1".to_string()).await.unwrap();
    assert_eq!(session.status, SessionStatus::Pending);
    assert!(session.resume_key.is_some());

    let session_id = session.id.clone();

    // Transition to Active
    service.update_status(&session_id, SessionStatus::Active).await.unwrap();
    let found = service.get_session(&session_id).await.unwrap().unwrap();
    assert_eq!(found.status, SessionStatus::Active);

    // Transition to Idle
    service.update_status(&session_id, SessionStatus::Idle).await.unwrap();
    let found = service.get_session(&session_id).await.unwrap().unwrap();
    assert_eq!(found.status, SessionStatus::Idle);

    // Transition to Paused
    service.update_status(&session_id, SessionStatus::Paused).await.unwrap();
    let found = service.get_session(&session_id).await.unwrap().unwrap();
    assert_eq!(found.status, SessionStatus::Paused);

    // Transition to Terminated
    service.update_status(&session_id, SessionStatus::Terminated).await.unwrap();
    let found = service.get_session(&session_id).await.unwrap().unwrap();
    assert_eq!(found.status, SessionStatus::Terminated);
}

#[tokio::test]
async fn test_session_pause_and_resume() {
    let repo = SqliteSessionRepository::in_memory().unwrap();
    let service = SessionService::new(Arc::new(repo));

    let session = service.create_session("workflow-1".to_string()).await.unwrap();
    let session_id = session.id.clone();

    // Activate then pause
    service.update_status(&session_id, SessionStatus::Active).await.unwrap();
    service.update_status(&session_id, SessionStatus::Paused).await.unwrap();

    let found = service.get_session(&session_id).await.unwrap().unwrap();
    assert_eq!(found.status, SessionStatus::Paused);
    assert!(found.can_resume());

    // Resume
    service.update_status(&session_id, SessionStatus::Active).await.unwrap();
    let found = service.get_session(&session_id).await.unwrap().unwrap();
    assert_eq!(found.status, SessionStatus::Active);
}

#[tokio::test]
async fn test_session_list_all() {
    let repo = SqliteSessionRepository::in_memory().unwrap();
    let service = SessionService::new(Arc::new(repo));

    // Create multiple sessions
    service.create_session("workflow-1".to_string()).await.unwrap();
    service.create_session("workflow-2".to_string()).await.unwrap();
    service.create_session("workflow-3".to_string()).await.unwrap();

    let sessions = service.list_sessions().await.unwrap();
    assert_eq!(sessions.len(), 3);
}

#[tokio::test]
async fn test_session_delete() {
    let repo = SqliteSessionRepository::in_memory().unwrap();
    let service = SessionService::new(Arc::new(repo));

    let session = service.create_session("workflow-1".to_string()).await.unwrap();
    let session_id = session.id.clone();

    // Delete
    let deleted = service.delete_session(&session_id).await.unwrap();
    assert!(deleted);

    // Verify deleted
    let found = service.get_session(&session_id).await.unwrap();
    assert!(found.is_none());

    // List should be empty
    let sessions = service.list_sessions().await.unwrap();
    assert!(sessions.is_empty());
}

#[tokio::test]
async fn test_session_delete_nonexistent() {
    let repo = SqliteSessionRepository::in_memory().unwrap();
    let service = SessionService::new(Arc::new(repo));

    let deleted = service.delete_session("non-existent-id").await.unwrap();
    assert!(!deleted);
}

#[test]
fn test_session_model_state_machine() {
    let mut session = Session::new("workflow-1".to_string());

    // Initial state
    assert_eq!(session.status, SessionStatus::Pending);
    assert!(!session.can_resume());

    // Activate
    session.activate();
    assert_eq!(session.status, SessionStatus::Active);

    // Idle
    session.idle();
    assert_eq!(session.status, SessionStatus::Idle);

    // Pause
    session.pause();
    assert_eq!(session.status, SessionStatus::Paused);
    assert!(session.can_resume());

    // Resume
    session.resume();
    assert_eq!(session.status, SessionStatus::Active);

    // Can only resume from Paused
    session.pause();
    session.resume();
    session.pause();
    // Try to resume again when already Active
    session.resume();
    assert_eq!(session.status, SessionStatus::Active); // Still Active, not double-resumed

    // Terminate
    session.terminate();
    assert_eq!(session.status, SessionStatus::Terminated);
}

#[test]
fn test_session_display_trait() {
    assert_eq!(SessionStatus::Pending.to_string(), "pending");
    assert_eq!(SessionStatus::Active.to_string(), "active");
    assert_eq!(SessionStatus::Idle.to_string(), "idle");
    assert_eq!(SessionStatus::Paused.to_string(), "paused");
    assert_eq!(SessionStatus::Terminated.to_string(), "terminated");
}

#[tokio::test]
async fn test_session_find_by_resume_key() {
    let repo = SqliteSessionRepository::in_memory().unwrap();
    let service = SessionService::new(Arc::new(repo));

    let session = service.create_session("workflow-1".to_string()).await.unwrap();
    let resume_key = session.resume_key.clone().unwrap();

    // Find by resume key
    let found = service.get_session_by_resume_key(&resume_key).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, session.id);
}

#[tokio::test]
async fn test_session_find_by_invalid_resume_key() {
    let repo = SqliteSessionRepository::in_memory().unwrap();
    let service = SessionService::new(Arc::new(repo));

    service.create_session("workflow-1".to_string()).await.unwrap();

    // Find by non-existent resume key
    let found = service.get_session_by_resume_key("invalid-key").await.unwrap();
    assert!(found.is_none());
}
