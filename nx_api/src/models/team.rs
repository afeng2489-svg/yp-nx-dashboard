//! Team models
//!
//! Data models for multi-agent team collaboration.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Team role skill priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillPriority {
    Critical,
    High,
    Medium,
    Low,
}

impl Default for SkillPriority {
    fn default() -> Self {
        SkillPriority::Medium
    }
}

impl SkillPriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            SkillPriority::Critical => "critical",
            SkillPriority::High => "high",
            SkillPriority::Medium => "medium",
            SkillPriority::Low => "low",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "critical" => SkillPriority::Critical,
            "high" => SkillPriority::High,
            "medium" => SkillPriority::Medium,
            "low" => SkillPriority::Low,
            _ => SkillPriority::Medium,
        }
    }
}

/// Team entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Team {
    pub fn new(name: String, description: String) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            description,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Team role entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamRole {
    pub id: String,
    pub team_id: Option<String>,  // Optional: role can exist without a team
    pub name: String,
    pub description: String,
    pub model_config: ModelConfig,
    pub system_prompt: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TeamRole {
    pub fn new(
        team_id: Option<String>,
        name: String,
        description: String,
        model_config: ModelConfig,
        system_prompt: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            team_id,
            name,
            description,
            model_config,
            system_prompt,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Model configuration for a role
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub model_id: String,
    pub provider: String,
    pub max_tokens: usize,
    pub temperature: f32,
    pub stop_sequences: Vec<String>,
    pub extra_params: HashMap<String, String>,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            model_id: "claude-sonnet-4-5".to_string(),
            provider: "anthropic".to_string(),
            max_tokens: 4096,
            temperature: 0.7,
            stop_sequences: vec![],
            extra_params: HashMap::new(),
        }
    }
}

/// Role skill assignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleSkill {
    pub role_id: String,
    pub skill_id: String,
    pub priority: SkillPriority,
}

/// Message type for team conversations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
    User,
    Assistant,
    System,
    Notification,
}

impl MessageType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageType::User => "user",
            MessageType::Assistant => "assistant",
            MessageType::System => "system",
            MessageType::Notification => "notification",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "user" => MessageType::User,
            "assistant" => MessageType::Assistant,
            "system" => MessageType::System,
            "notification" => MessageType::Notification,
            _ => MessageType::User,
        }
    }
}

/// Team message entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMessage {
    pub id: String,
    pub team_id: String,
    pub role_id: Option<String>,
    pub content: String,
    pub message_type: MessageType,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
}

impl TeamMessage {
    pub fn new(
        team_id: String,
        role_id: Option<String>,
        content: String,
        message_type: MessageType,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            team_id,
            role_id,
            content,
            message_type,
            metadata: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    pub fn user_message(team_id: String, content: String) -> Self {
        Self::new(team_id, None, content, MessageType::User)
    }

    pub fn assistant_message(team_id: String, role_id: String, content: String) -> Self {
        Self::new(team_id, Some(role_id), content, MessageType::Assistant)
    }

    pub fn system_message(team_id: String, content: String) -> Self {
        Self::new(team_id, None, content, MessageType::System)
    }

    pub fn notification(team_id: String, content: String) -> Self {
        Self::new(team_id, None, content, MessageType::Notification)
    }
}

/// Telegram bot configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramBotConfig {
    pub id: String,
    pub role_id: String,
    pub bot_token: String,
    pub chat_id: Option<String>,
    pub enabled: bool,
    pub notifications_enabled: bool,
    pub conversation_enabled: bool,
}

impl TelegramBotConfig {
    pub fn new(role_id: String, bot_token: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role_id,
            bot_token,
            chat_id: None,
            enabled: false,
            notifications_enabled: true,
            conversation_enabled: false,
        }
    }
}

/// Telegram update from long polling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramUpdate {
    pub update_id: i64,
    pub chat_id: i64,
    pub text: Option<String>,
    pub chat_type: String,
}

/// Team execution request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteTeamTaskRequest {
    pub team_id: String,
    pub task: String,
    pub context: HashMap<String, String>,
}

/// Team execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteTeamTaskResponse {
    pub success: bool,
    pub team_id: String,
    pub messages: Vec<TeamMessage>,
    pub final_output: String,
    pub error: Option<String>,
}

/// Role execution request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteRoleTaskRequest {
    pub role_id: String,
    pub task: String,
    pub context: HashMap<String, String>,
}

/// Role execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteRoleTaskResponse {
    pub success: bool,
    pub role_id: String,
    pub response: String,
    pub error: Option<String>,
}

/// Create team request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTeamRequest {
    pub name: String,
    pub description: String,
}

/// Update team request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTeamRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

/// Create role request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRoleRequest {
    pub name: String,
    pub description: String,
    pub model_config: Option<ModelConfig>,
    pub system_prompt: String,
}

/// Update role request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRoleRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub model_config: Option<ModelConfig>,
    pub system_prompt: Option<String>,
}

/// Assign skill request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignSkillRequest {
    pub skill_id: Option<String>,  // Optional - skill_id comes from URL path
    pub priority: Option<SkillPriority>,
}

/// Assign role to team request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignRoleToTeamRequest {
    pub team_id: String,
}

/// Telegram config request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfigRequest {
    pub bot_token: String,
    pub chat_id: Option<String>,
    pub notifications_enabled: Option<bool>,
    pub conversation_enabled: Option<bool>,
}

/// Telegram send message request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramSendMessageRequest {
    pub chat_id: String,
    pub text: String,
}

/// Role with skills (for detailed view)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleWithSkills {
    pub role: TeamRole,
    pub skills: Vec<String>,
}

/// Team with roles (for detailed view)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamWithRoles {
    pub team: Team,
    pub roles: Vec<RoleWithSkills>,
}