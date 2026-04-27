//! 服务层

pub mod agent_team_service;
pub mod ai_provider_repository;
pub mod ai_provider_service;
pub mod api_key_repository;
pub mod claude_cli;
pub mod claude_terminal;
pub mod events;
pub mod execution_bridge;
pub mod execution_service;
pub mod file_skill_repository;
pub mod group_chat_repository;
pub mod group_chat_service;
pub mod issue_repository;
pub mod plugin_service;
pub mod project_repository;
pub mod project_service;
pub mod pty_task_watcher;
pub mod session_repository;
pub mod session_service;
pub mod skill_repository;
pub mod skill_service;
pub mod team_evolution;
pub mod team_repository;
pub mod team_service;
pub mod telegram_service;
pub mod test_generator;
pub mod workflow_repository;
pub mod workflow_service;
pub mod workspace_repository;
pub mod workspace_service;

pub use agent_team_service::{AgentTeamService, AgentTeamServiceError};
pub use ai_provider_repository::{
    AIProvider, APIFormat, MappingType, ModelMapping, ProviderPreset, ProviderRepository,
    ProviderRepositoryError, SqliteProviderRepository,
};
pub use ai_provider_service::{
    ConnectionTestResult, ProviderService, ProviderServiceError, SharedProviderService,
};
pub use api_key_repository::{ApiKeyRepository, ApiKeyRepositoryError, SqliteApiKeyRepository};
pub use claude_terminal::{ClaudeTerminalManager, TerminalSessionInfo};
pub use events::{ExecutionEvent, ExecutionStatus};
pub use execution_service::ExecutionService;
pub use file_skill_repository::{FileSkillRepository, FileSkillRepositoryError, SkillFileInfo};
pub use group_chat_repository::{
    GroupChatRepository, GroupChatRepositoryError, SqliteGroupChatRepository,
};
pub use group_chat_service::{GroupChatService, GroupChatServiceError, SharedGroupChatService};
pub use issue_repository::SqliteIssueRepository;
pub use plugin_service::{PluginInfo, PluginService};
pub use project_repository::{
    ProjectRepository, RepositoryError as ProjectRepositoryError, SqliteProjectRepository,
};
pub use project_service::{ProjectError, ProjectService};
pub use session_repository::{RepositoryError, SessionRepository, SqliteSessionRepository};
pub use session_service::{Session, SessionService, SessionStatus};
pub use skill_repository::{SkillRepository, SkillRepositoryError, SqliteSkillRepository};
pub use skill_service::{
    ExecuteSkillRequest, ExecuteSkillResponse, SearchSkillsRequest, SkillDetail, SkillService,
    SkillStats, SkillSummary,
};
pub use team_repository::{SqliteTeamRepository, TeamRepository, TeamRepositoryError};
pub use team_service::{RoleWithSkills, TeamService, TeamServiceError, TeamWithRoles};
pub use telegram_service::{InboundTelegramMessage, TelegramError, TelegramService};
pub use test_generator::{
    GenerateTestsRequest, GenerateTestsResponse, Language, TestFramework, TestGenError,
    TestGenerator,
};
pub use workflow_repository::{
    SharedWorkflowRepository, SqliteWorkflowRepository, WorkflowRepository,
};
pub use workflow_service::WorkflowService;
pub use workspace_repository::{
    RepositoryError as WorkspaceRepositoryError, SqliteWorkspaceRepository, Workspace,
    WorkspaceRepository,
};
pub use workspace_service::WorkspaceService;

// Re-export PtyManager from nx_session
pub use nx_session::pty::PtyManager;

// Wisdom service exports (re-exported from crate root)
pub use crate::wisdom::{
    CategorySummary, CreateWisdomRequest, QueryWisdomRequest, SqliteWisdomStore, WisdomCategory,
    WisdomEntry, WisdomResponse, WisdomService,
};
pub type SharedWisdomService = std::sync::Arc<WisdomService>;
