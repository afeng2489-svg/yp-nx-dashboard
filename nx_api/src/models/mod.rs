//! Data models

pub mod team;
pub mod project;
pub mod skill;
pub mod group_chat;
pub mod issue;

pub use team::{
    CreateRoleRequest, CreateTeamRequest, ExecuteRoleTaskRequest, ExecuteRoleTaskResponse,
    ExecuteTeamTaskRequest, ExecuteTeamTaskResponse, ModelConfig, RoleSkill, RoleWithSkills,
    SkillPriority, Team, TeamMessage, TeamRole, TeamWithRoles, TelegramBotConfig,
    TelegramConfigRequest, TelegramSendMessageRequest, TelegramUpdate, MessageType,
};

pub use project::{
    Project, ProjectStatus, ProjectWithTeam, CreateProjectRequest, UpdateProjectRequest,
    ExecuteProjectRequest, ExecuteProjectResponse, ProjectMessage,
};

pub use skill::{
    SkillCategory, SkillDetail, SkillMetadata, SkillParameter, SkillRecord, SkillSummary,
    CreateSkillRequest, UpdateSkillRequest,
};

pub use group_chat::{
    ConsensusStrategy, CreateGroupSessionRequest, DiscussionTurnInfo, GroupConclusion,
    GroupMessage, GroupParticipant, GroupSession, GroupSessionDetail, GroupStatus,
    SendMessageRequest, SpeakingStrategy, StartDiscussionRequest, ToolCall,
    UpdateGroupSessionRequest, ConcludeDiscussionRequest, GetMessagesRequest,
};