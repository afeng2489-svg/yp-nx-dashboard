//! A2UI Message Types
//!
//! Defines the interactive message types for Agent-to-User communication.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Agent-to-User message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum A2UIMessage {
    /// Agent asks user a question
    Ask {
        question: String,
        context: Option<String>,
    },
    /// Agent informs user
    Inform { message: String, level: InformLevel },
    /// Agent requests confirmation
    Confirm {
        prompt: String,
        details: Option<String>,
    },
    /// User selection request
    Select {
        prompt: String,
        options: Vec<String>,
    },
}

/// Inform message levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InformLevel {
    Info,
    Warning,
    Error,
    Success,
}

impl Default for InformLevel {
    fn default() -> Self {
        Self::Info
    }
}

/// User-to-Agent message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum U2AMessage {
    /// User responds with text
    Response(String),
    /// User selects an option by index
    Select(usize),
    /// User confirms
    Confirm,
    /// User cancels
    Cancel,
}

/// Interactive message envelope for API communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractiveMessage {
    /// Unique message ID
    pub id: String,
    /// Session ID this message belongs to
    pub session_id: String,
    /// Execution ID this message is associated with
    pub execution_id: String,
    /// The actual message content
    pub content: A2UIMessage,
    /// Source agent ID
    pub source: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Whether the message is pending user response
    pub pending: bool,
    /// User response if provided
    pub response: Option<U2AMessage>,
    /// Response timestamp if provided
    pub responded_at: Option<DateTime<Utc>>,
}

impl InteractiveMessage {
    /// Create a new Ask message
    pub fn ask(
        session_id: impl Into<String>,
        execution_id: impl Into<String>,
        source: impl Into<String>,
        question: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: session_id.into(),
            execution_id: execution_id.into(),
            content: A2UIMessage::Ask {
                question: question.into(),
                context: None,
            },
            source: source.into(),
            timestamp: Utc::now(),
            pending: true,
            response: None,
            responded_at: None,
        }
    }

    /// Create a new Inform message
    pub fn inform(
        session_id: impl Into<String>,
        execution_id: impl Into<String>,
        source: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: session_id.into(),
            execution_id: execution_id.into(),
            content: A2UIMessage::Inform {
                message: message.into(),
                level: InformLevel::Info,
            },
            source: source.into(),
            timestamp: Utc::now(),
            pending: false,
            response: None,
            responded_at: None,
        }
    }

    /// Create a new Confirm message
    pub fn confirm(
        session_id: impl Into<String>,
        execution_id: impl Into<String>,
        source: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: session_id.into(),
            execution_id: execution_id.into(),
            content: A2UIMessage::Confirm {
                prompt: prompt.into(),
                details: None,
            },
            source: source.into(),
            timestamp: Utc::now(),
            pending: true,
            response: None,
            responded_at: None,
        }
    }

    /// Create a new Select message
    pub fn select(
        session_id: impl Into<String>,
        execution_id: impl Into<String>,
        source: impl Into<String>,
        prompt: impl Into<String>,
        options: Vec<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: session_id.into(),
            execution_id: execution_id.into(),
            content: A2UIMessage::Select {
                prompt: prompt.into(),
                options,
            },
            source: source.into(),
            timestamp: Utc::now(),
            pending: true,
            response: None,
            responded_at: None,
        }
    }

    /// Add response to the message
    pub fn with_response(mut self, response: U2AMessage) -> Self {
        self.pending = false;
        self.response = Some(response);
        self.responded_at = Some(Utc::now());
        self
    }
}

/// API request for user response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponseRequest {
    /// Message ID to respond to
    pub message_id: String,
    /// User's response
    pub response: U2AMessage,
}

/// API response for messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagesResponse {
    pub messages: Vec<InteractiveMessage>,
    pub pending_count: usize,
}
