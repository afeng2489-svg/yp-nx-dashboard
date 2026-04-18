//! Group Chat Models
//!
//! Data models for multi-agent group discussion.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Group discussion status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupStatus {
    Pending,     // 等待开始
    Active,      // 讨论中
    Concluded,   // 已结束
    Cancelled,   // 已取消
}

impl Default for GroupStatus {
    fn default() -> Self {
        GroupStatus::Pending
    }
}

impl GroupStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            GroupStatus::Pending => "pending",
            GroupStatus::Active => "active",
            GroupStatus::Concluded => "concluded",
            GroupStatus::Cancelled => "cancelled",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "active" => GroupStatus::Active,
            "concluded" => GroupStatus::Concluded,
            "cancelled" => GroupStatus::Cancelled,
            _ => GroupStatus::Pending,
        }
    }
}

/// Speaking strategy for group discussion
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpeakingStrategy {
    Free,      // 自由发言
    RoundRobin, // 轮询发言
    Moderator,  // 主持人模式
    Debate,     // 辩论模式
}

impl Default for SpeakingStrategy {
    fn default() -> Self {
        SpeakingStrategy::Free
    }
}

impl SpeakingStrategy {
    pub fn as_str(&self) -> &'static str {
        match self {
            SpeakingStrategy::Free => "free",
            SpeakingStrategy::RoundRobin => "round_robin",
            SpeakingStrategy::Moderator => "moderator",
            SpeakingStrategy::Debate => "debate",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "round_robin" => SpeakingStrategy::RoundRobin,
            "moderator" => SpeakingStrategy::Moderator,
            "debate" => SpeakingStrategy::Debate,
            _ => SpeakingStrategy::Free,
        }
    }
}

/// Consensus strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsensusStrategy {
    Majority,  // 多数同意
    Unanimous, // 全员同意
    Score,     // 评分共识
}

impl Default for ConsensusStrategy {
    fn default() -> Self {
        ConsensusStrategy::Majority
    }
}

impl ConsensusStrategy {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConsensusStrategy::Majority => "majority",
            ConsensusStrategy::Unanimous => "unanimous",
            ConsensusStrategy::Score => "score",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "unanimous" => ConsensusStrategy::Unanimous,
            "score" => ConsensusStrategy::Score,
            _ => ConsensusStrategy::Majority,
        }
    }
}

/// Group session entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupSession {
    pub id: String,
    pub team_id: String,
    pub name: String,
    pub topic: String,
    pub status: GroupStatus,
    pub speaking_strategy: SpeakingStrategy,
    pub consensus_strategy: ConsensusStrategy,
    pub moderator_role_id: Option<String>,  // 主持人角色ID
    pub max_turns: u32,                     // 最大轮次
    pub current_turn: u32,                  // 当前轮次
    pub turn_policy: String,                // 轮次策略，如 "all" 或 "one_per_role"
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl GroupSession {
    pub fn new(
        team_id: String,
        name: String,
        topic: String,
        speaking_strategy: SpeakingStrategy,
        consensus_strategy: ConsensusStrategy,
        moderator_role_id: Option<String>,
        max_turns: u32,
        turn_policy: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            team_id,
            name,
            topic,
            status: GroupStatus::Pending,
            speaking_strategy,
            consensus_strategy,
            moderator_role_id,
            max_turns,
            current_turn: 0,
            turn_policy,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Group message entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMessage {
    pub id: String,
    pub session_id: String,
    pub role_id: String,
    pub role_name: String,
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub reply_to: Option<String>,
    pub turn_number: u32,
    pub created_at: DateTime<Utc>,
}

impl GroupMessage {
    pub fn new(
        session_id: String,
        role_id: String,
        role_name: String,
        content: String,
        tool_calls: Vec<ToolCall>,
        reply_to: Option<String>,
        turn_number: u32,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_id,
            role_id,
            role_name,
            content,
            tool_calls,
            reply_to,
            turn_number,
            created_at: Utc::now(),
        }
    }
}

/// Tool call from Claude CLI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub input: HashMap<String, serde_json::Value>,
}

/// Group conclusion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupConclusion {
    pub id: String,
    pub session_id: String,
    pub content: String,
    pub consensus_level: f32,  // 0.0 - 1.0
    pub participant_scores: HashMap<String, f32>,  // role_id -> score
    pub agreed_by: Vec<String>,  // 同意的角色ID列表
    pub created_at: DateTime<Utc>,
}

impl GroupConclusion {
    pub fn new(
        session_id: String,
        content: String,
        consensus_level: f32,
        participant_scores: HashMap<String, f32>,
        agreed_by: Vec<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_id,
            content,
            consensus_level,
            participant_scores,
            agreed_by,
            created_at: Utc::now(),
        }
    }
}

/// Participant in a group discussion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupParticipant {
    pub role_id: String,
    pub role_name: String,
    pub joined_at: DateTime<Utc>,
    pub last_spoke_at: Option<DateTime<Utc>>,
    pub message_count: u32,
}

// ============== Request/Response Types ==============

/// Create group session request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGroupSessionRequest {
    pub team_id: String,
    pub name: String,
    pub topic: String,
    pub speaking_strategy: Option<SpeakingStrategy>,
    pub consensus_strategy: Option<ConsensusStrategy>,
    pub moderator_role_id: Option<String>,
    pub max_turns: Option<u32>,
    pub turn_policy: Option<String>,
}

/// Update group session request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateGroupSessionRequest {
    pub name: Option<String>,
    pub topic: Option<String>,
    pub speaking_strategy: Option<SpeakingStrategy>,
    pub consensus_strategy: Option<ConsensusStrategy>,
    pub moderator_role_id: Option<String>,
    pub max_turns: Option<u32>,
}

/// Send message request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub role_id: String,
    pub content: String,
    pub reply_to: Option<String>,
}

/// Start discussion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartDiscussionRequest {
    pub participant_role_ids: Vec<String>,
}

/// Conclude discussion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcludeDiscussionRequest {
    pub force: Option<bool>,  // 强制结束
}

/// Get session messages request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetMessagesRequest {
    pub limit: Option<u32>,
    pub before: Option<String>,  // 消息ID，用于分页
}

// ============== Response Types ==============

/// Group session with participants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupSessionDetail {
    #[serde(flatten)]
    pub session: GroupSession,
    pub participants: Vec<GroupParticipant>,
    pub message_count: u32,
    pub conclusion: Option<GroupConclusion>,
}

/// Discussion turn info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscussionTurnInfo {
    pub current_turn: u32,
    pub max_turns: u32,
    pub next_speaker_role_id: Option<String>,
    pub next_speaker_role_name: Option<String>,
    pub speaking_order: Vec<String>,  // 角色ID列表
}

/// Next speaker info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextSpeakerInfo {
    pub role_id: String,
    pub role_name: String,
}
