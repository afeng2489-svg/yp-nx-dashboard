//! A2UI Module for NexusFlow API
//!
//! Agent-to-User Interactive Communication module.
//! Provides real-time messaging, session management, and user interaction capabilities.

pub mod message;
pub mod session;

pub use message::{A2UIMessage, U2AMessage, InteractiveMessage, UserResponseRequest, MessagesResponse, InformLevel};
pub use session::{A2UISession, A2UISessionManager, A2UISessionEvent, SessionState};

use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::broadcast;

use crate::ws::WebSocketHandler;
use super::error::ApiResult;

/// A2UI service for managing interactive communication
pub struct A2UIService {
    /// Session manager
    session_manager: Arc<A2UISessionManager>,
    /// WebSocket handler for real-time communication
    ws_handler: Arc<RwLock<Option<WebSocketHandler>>>,
}

impl A2UIService {
    /// Create a new A2UI service
    pub fn new() -> Self {
        Self {
            session_manager: Arc::new(A2UISessionManager::new()),
            ws_handler: Arc::new(RwLock::new(None)),
        }
    }

    /// Get the session manager
    pub fn session_manager(&self) -> &A2UISessionManager {
        &self.session_manager
    }

    /// Set the WebSocket handler
    pub fn set_ws_handler(&self, handler: WebSocketHandler) {
        *self.ws_handler.write() = Some(handler);
    }

    /// Create or get a session for an execution
    pub fn get_or_create_session(&self, execution_id: &str) -> A2UISession {
        self.session_manager.get_or_create_session(execution_id)
    }

    /// Get a session by ID
    pub fn get_session(&self, session_id: &str) -> Option<A2UISession> {
        self.session_manager.get_session(session_id)
    }

    /// Add an interactive message to a session
    pub fn add_message(&self, session_id: &str, message: InteractiveMessage) -> ApiResult<()> {
        if message.content.is_ask() || message.content.is_confirm() || message.content.is_select() {
            self.session_manager
                .add_pending_message(session_id, message)
                .ok_or_else(|| crate::error::ApiError::SessionNotFound(format!("Session not found: {}", session_id)))?;
        } else {
            self.session_manager
                .add_message(session_id, message)
                .ok_or_else(|| crate::error::ApiError::SessionNotFound(format!("Session not found: {}", session_id)))?;
        }
        Ok(())
    }

    /// Send a user response to a pending message
    pub fn respond(&self, session_id: &str, message_id: &str, response: U2AMessage) -> ApiResult<InteractiveMessage> {
        self.session_manager
            .respond(session_id, message_id, response)
            .ok_or_else(|| crate::error::ApiError::MessageNotFound("Message not found or already responded".to_string()))
    }

    /// Get pending messages for a session
    pub fn get_pending_messages(&self, session_id: &str) -> Vec<InteractiveMessage> {
        self.session_manager.get_pending_messages(session_id)
    }

    /// Get all messages for a session
    pub fn get_all_messages(&self, session_id: &str) -> Vec<InteractiveMessage> {
        self.session_manager
            .get_session(session_id)
            .map(|s| s.messages().to_vec())
            .unwrap_or_default()
    }

    /// End a session
    pub fn end_session(&self, session_id: &str) -> ApiResult<()> {
        self.session_manager
            .end_session(session_id)
            .ok_or_else(|| crate::error::ApiError::SessionNotFound(format!("Session not found: {}", session_id)))
    }

    /// Subscribe to session events
    pub fn subscribe(&self, session_id: &str) -> Option<broadcast::Receiver<A2UISessionEvent>> {
        self.session_manager.subscribe(session_id)
    }
}

impl Default for A2UIService {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait for A2UIMessage to check message types
pub trait A2UIMessageExt {
    fn is_ask(&self) -> bool;
    fn is_confirm(&self) -> bool;
    fn is_select(&self) -> bool;
    fn is_inform(&self) -> bool;
}

impl A2UIMessageExt for A2UIMessage {
    fn is_ask(&self) -> bool {
        matches!(self, A2UIMessage::Ask { .. })
    }

    fn is_confirm(&self) -> bool {
        matches!(self, A2UIMessage::Confirm { .. })
    }

    fn is_select(&self) -> bool {
        matches!(self, A2UIMessage::Select { .. })
    }

    fn is_inform(&self) -> bool {
        matches!(self, A2UIMessage::Inform { .. })
    }
}