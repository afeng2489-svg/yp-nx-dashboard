//! 服务层

pub mod workflow_repository;
pub mod workflow_service;
pub mod execution_service;
pub mod execution_bridge;
pub mod events;
pub mod session_service;
pub mod session_repository;
pub mod workspace_repository;
pub mod workspace_service;
pub mod test_generator;
pub mod plugin_service;
pub mod skill_service;
pub mod skill_repository;
pub mod file_skill_repository;
pub mod team_repository;
pub mod team_service;
pub mod telegram_service;
pub mod agent_team_service;
pub mod api_key_repository;
pub mod ai_provider_repository;
pub mod ai_provider_service;
pub mod project_repository;
pub mod project_service;
pub mod claude_cli;
pub mod group_chat_repository;
pub mod group_chat_service;

pub use workflow_repository::{SqliteWorkflowRepository, SharedWorkflowRepository, WorkflowRepository};
pub use workflow_service::WorkflowService;
pub use execution_service::ExecutionService;
pub use events::{ExecutionEvent, ExecutionStatus};
pub use session_service::{SessionService, SessionStatus, Session};
pub use session_repository::{SessionRepository, SqliteSessionRepository, RepositoryError};
pub use workspace_repository::{WorkspaceRepository, SqliteWorkspaceRepository, RepositoryError as WorkspaceRepositoryError, Workspace};
pub use workspace_service::WorkspaceService;
pub use test_generator::{TestGenerator, TestFramework, Language, GenerateTestsRequest, GenerateTestsResponse, TestGenError};
pub use plugin_service::{PluginService, PluginInfo};
pub use skill_service::{SkillService, SkillSummary, SkillDetail, SkillStats, ExecuteSkillRequest, ExecuteSkillResponse, SearchSkillsRequest};
pub use skill_repository::{SkillRepository, SqliteSkillRepository, SkillRepositoryError};
pub use file_skill_repository::{FileSkillRepository, FileSkillRepositoryError, SkillFileInfo};
pub use team_repository::{TeamRepository, SqliteTeamRepository, TeamRepositoryError};
pub use team_service::{TeamService, TeamServiceError, TeamWithRoles, RoleWithSkills};
pub use telegram_service::{TelegramService, TelegramError, InboundTelegramMessage};
pub use agent_team_service::{AgentTeamService, AgentTeamServiceError};
pub use api_key_repository::{ApiKeyRepository, SqliteApiKeyRepository, ApiKeyRepositoryError};
pub use ai_provider_repository::{
    AIProvider, APIFormat, MappingType, ModelMapping, ProviderPreset,
    ProviderRepository, SqliteProviderRepository, ProviderRepositoryError,
};
pub use ai_provider_service::{ProviderService, ProviderServiceError, SharedProviderService, ConnectionTestResult};
pub use project_repository::{ProjectRepository, SqliteProjectRepository, RepositoryError as ProjectRepositoryError};
pub use project_service::{ProjectService, ProjectError};
pub use group_chat_repository::{GroupChatRepository, GroupChatRepositoryError, SqliteGroupChatRepository};
pub use group_chat_service::{GroupChatService, GroupChatServiceError, SharedGroupChatService};

// Re-export PtyManager from nx_session
pub use nx_session::pty::PtyManager;

// Wisdom service exports (re-exported from crate root)
pub use crate::wisdom::{WisdomService, SqliteWisdomStore, WisdomCategory, WisdomEntry, CreateWisdomRequest, QueryWisdomRequest, WisdomResponse, CategorySummary};
pub type SharedWisdomService = std::sync::Arc<WisdomService>;