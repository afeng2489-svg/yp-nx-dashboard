//! A2UI Module for NexusFlow API
//!
//! Agent-to-User Interactive Communication module.
//! Provides real-time messaging, session management, and user interaction capabilities.

pub mod message;
pub mod session;

pub use message::{
    A2UIMessage, InformLevel, InteractiveMessage, MessagesResponse, U2AMessage, UserResponseRequest,
};
pub use session::{A2UISession, A2UISessionEvent, A2UISessionManager, SessionState};

use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::broadcast;

use super::error::ApiResult;
use crate::services::session_message_store::{PersistedMessage, SessionMessageStore};
use crate::ws::WebSocketHandler;

/// A2UI service for managing interactive communication
pub struct A2UIService {
    session_manager: Arc<A2UISessionManager>,
    ws_handler: Arc<RwLock<Option<WebSocketHandler>>>,
    msg_store: Option<Arc<SessionMessageStore>>,
}

impl A2UIService {
    pub fn new() -> Self {
        Self {
            session_manager: Arc::new(A2UISessionManager::new()),
            ws_handler: Arc::new(RwLock::new(None)),
            msg_store: None,
        }
    }

    pub fn with_store(mut self, store: Arc<SessionMessageStore>) -> Self {
        self.msg_store = Some(store);
        self
    }

    pub fn msg_store(&self) -> Option<&Arc<SessionMessageStore>> {
        self.msg_store.as_ref()
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
        let is_pending = message.content.is_ask() || message.content.is_confirm() || message.content.is_select();
        if is_pending {
            self.session_manager
                .add_pending_message(session_id, message.clone())
                .ok_or_else(|| crate::error::ApiError::SessionNotFound(format!("Session not found: {}", session_id)))?;
        } else {
            self.session_manager
                .add_message(session_id, message.clone())
                .ok_or_else(|| crate::error::ApiError::SessionNotFound(format!("Session not found: {}", session_id)))?;
        }
        if let Some(store) = &self.msg_store {
            let pm = PersistedMessage {
                id: message.id.clone(),
                session_id: session_id.to_string(),
                execution_id: Some(message.execution_id.clone()),
                role: "agent".to_string(),
                content_json: serde_json::to_string(&message.content).unwrap_or_default(),
                pending: is_pending,
                responded: false,
                created_at: message.timestamp.to_rfc3339(),
            };
            let _ = store.insert(&pm);
        }
        Ok(())
    }

    /// Send a user response to a pending message
    pub fn respond(
        &self,
        session_id: &str,
        message_id: &str,
        response: U2AMessage,
    ) -> ApiResult<InteractiveMessage> {
        let msg = self.session_manager
            .respond(session_id, message_id, response)
            .ok_or_else(|| {
                crate::error::ApiError::MessageNotFound(
                    "Message not found or already responded".to_string(),
                )
            })?;
        if let Some(store) = &self.msg_store {
            let _ = store.mark_responded(message_id);
        }
        Ok(msg)
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
        self.session_manager.end_session(session_id).ok_or_else(|| {
            crate::error::ApiError::SessionNotFound(format!("Session not found: {}", session_id))
        })
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
