//! A2UI Session Management
//!
//! Manages interactive sessions for Agent-to-User communication.

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::Serialize;
use std::collections::HashMap;
use tokio::sync::broadcast;

use super::message::{A2UIMessage, InteractiveMessage, U2AMessage};

/// Session state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SessionState {
    /// Session is active and waiting for input
    Waiting,
    /// Session is active and processing
    Processing,
    /// Session has received response and can continue
    Responded,
    /// Session has ended
    Ended,
}

/// Interactive session for A2UI
#[derive(Debug, Clone, Serialize)]
pub struct A2UISession {
    /// Unique session ID
    pub id: String,
    /// Associated execution ID
    pub execution_id: String,
    /// Session state
    pub state: SessionState,
    /// Pending messages waiting for user response
    pending_messages: Vec<String>,
    /// All messages in the session
    messages: Vec<InteractiveMessage>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
}

impl A2UISession {
    /// Create a new session
    pub fn new(execution_id: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            execution_id: execution_id.into(),
            state: SessionState::Waiting,
            pending_messages: Vec::new(),
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Add a message to the session
    pub fn add_message(&mut self, message: InteractiveMessage) {
        if message.pending {
            self.pending_messages.push(message.id.clone());
            self.state = SessionState::Waiting;
        } else {
            self.state = SessionState::Processing;
        }
        self.messages.push(message);
        self.updated_at = Utc::now();
    }

    /// Add a pending message
    pub fn add_pending(&mut self, message: InteractiveMessage) {
        self.pending_messages.push(message.id.clone());
        self.state = SessionState::Waiting;
        self.messages.push(message);
        self.updated_at = Utc::now();
    }

    /// Get pending message IDs
    pub fn pending_ids(&self) -> &[String] {
        &self.pending_messages
    }

    /// Get all messages
    pub fn messages(&self) -> &[InteractiveMessage] {
        &self.messages
    }

    /// Get pending messages
    pub fn get_pending_messages(&self) -> Vec<&InteractiveMessage> {
        self.messages.iter().filter(|m| m.pending).collect()
    }

    /// Check if session has pending messages
    pub fn has_pending(&self) -> bool {
        !self.pending_messages.is_empty()
    }

    /// Handle user response
    pub fn respond(
        &mut self,
        message_id: &str,
        response: U2AMessage,
    ) -> Option<&InteractiveMessage> {
        // Find and update the pending message
        for msg in &mut self.messages {
            if msg.id == message_id && msg.pending {
                msg.pending = false;
                msg.response = Some(response);
                msg.responded_at = Some(Utc::now());
                self.pending_messages.retain(|id| id != message_id);
                self.state = SessionState::Responded;
                self.updated_at = Utc::now();
                return Some(msg);
            }
        }
        None
    }

    /// Get a message by ID
    pub fn get_message(&self, message_id: &str) -> Option<&InteractiveMessage> {
        self.messages.iter().find(|m| m.id == message_id)
    }

    /// End the session
    pub fn end(&mut self) {
        self.state = SessionState::Ended;
        self.updated_at = Utc::now();
    }
}

/// Session manager for handling multiple sessions
pub struct A2UISessionManager {
    /// Active sessions
    sessions: RwLock<HashMap<String, A2UISession>>,
    /// Broadcast channels for real-time updates
    broadcasters: RwLock<HashMap<String, broadcast::Sender<A2UISessionEvent>>>,
}

impl A2UISessionManager {
    /// Create a new session manager
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            broadcasters: RwLock::new(HashMap::new()),
        }
    }

    /// Create a new session for an execution
    pub fn create_session(&self, execution_id: impl Into<String>) -> A2UISession {
        let session = A2UISession::new(execution_id);
        let session_id = session.id.clone();

        // Create broadcast channel for this session
        let (tx, _) = broadcast::channel(100);
        {
            let mut broadcasters = self.broadcasters.write();
            broadcasters.insert(session_id.clone(), tx);
        }

        // Store the session
        {
            let mut sessions = self.sessions.write();
            sessions.insert(session_id.clone(), session.clone());
        }

        session
    }

    /// Get a session by ID
    pub fn get_session(&self, session_id: &str) -> Option<A2UISession> {
        let sessions = self.sessions.read();
        sessions.get(session_id).cloned()
    }

    /// Get or create session for execution
    pub fn get_or_create_session(&self, execution_id: &str) -> A2UISession {
        // First check if a session exists for this execution
        {
            let sessions = self.sessions.read();
            for session in sessions.values() {
                if session.execution_id == execution_id && session.state != SessionState::Ended {
                    return session.clone();
                }
            }
        }

        // Create new session
        self.create_session(execution_id)
    }

    /// Add a message to a session
    pub fn add_message(&self, session_id: &str, message: InteractiveMessage) -> Option<()> {
        let mut sessions = self.sessions.write();
        if let Some(session) = sessions.get_mut(session_id) {
            session.add_message(message.clone());
            drop(sessions);

            // Broadcast the new message
            self.broadcast(session_id, A2UISessionEvent::NewMessage(message));
            Some(())
        } else {
            None
        }
    }

    /// Add a pending message to a session
    pub fn add_pending_message(&self, session_id: &str, message: InteractiveMessage) -> Option<()> {
        let mut sessions = self.sessions.write();
        if let Some(session) = sessions.get_mut(session_id) {
            session.add_pending(message.clone());
            drop(sessions);

            // Broadcast the pending message
            self.broadcast(session_id, A2UISessionEvent::PendingMessage(message));
            Some(())
        } else {
            None
        }
    }

    /// Get pending messages for a session
    pub fn get_pending_messages(&self, session_id: &str) -> Vec<InteractiveMessage> {
        let sessions = self.sessions.read();
        sessions
            .get(session_id)
            .map(|s| s.get_pending_messages().into_iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Handle user response
    pub fn respond(
        &self,
        session_id: &str,
        message_id: &str,
        response: U2AMessage,
    ) -> Option<InteractiveMessage> {
        let mut sessions = self.sessions.write();
        if let Some(session) = sessions.get_mut(session_id) {
            if let Some(message) = session.respond(message_id, response.clone()) {
                let msg_clone = message.clone();
                drop(sessions);

                // Broadcast the response
                self.broadcast(
                    session_id,
                    A2UISessionEvent::UserResponse {
                        message_id: message_id.to_string(),
                        response,
                    },
                );
                return Some(msg_clone);
            }
        }
        None
    }

    /// End a session
    pub fn end_session(&self, session_id: &str) -> Option<()> {
        let mut sessions = self.sessions.write();
        if let Some(session) = sessions.get_mut(session_id) {
            session.end();
            let execution_id = session.execution_id.clone();
            drop(sessions);

            // Broadcast session end
            self.broadcast(session_id, A2UISessionEvent::SessionEnded { execution_id });
            Some(())
        } else {
            None
        }
    }

    /// Subscribe to session events
    pub fn subscribe(&self, session_id: &str) -> Option<broadcast::Receiver<A2UISessionEvent>> {
        let broadcasters = self.broadcasters.read();
        broadcasters.get(session_id).map(|tx| tx.subscribe())
    }

    /// Broadcast an event
    fn broadcast(&self, session_id: &str, event: A2UISessionEvent) {
        let broadcasters = self.broadcasters.read();
        if let Some(tx) = broadcasters.get(session_id) {
            let _ = tx.send(event);
        }
    }

    /// List all active sessions
    pub fn list_sessions(&self) -> Vec<A2UISession> {
        let sessions = self.sessions.read();
        sessions
            .values()
            .filter(|s| s.state != SessionState::Ended)
            .cloned()
            .collect()
    }
}

impl Default for A2UISessionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Session events for real-time communication
#[derive(Debug, Clone)]
pub enum A2UISessionEvent {
    /// New message added
    NewMessage(InteractiveMessage),
    /// New pending message waiting for response
    PendingMessage(InteractiveMessage),
    /// User responded to a message
    UserResponse {
        message_id: String,
        response: U2AMessage,
    },
    /// Session ended
    SessionEnded { execution_id: String },
}

impl A2UISessionEvent {
    /// Check if this event indicates the session is waiting for user input
    pub fn is_waiting_for_input(&self) -> bool {
        matches!(self, Self::PendingMessage(_))
    }
}
