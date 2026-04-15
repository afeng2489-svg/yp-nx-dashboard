//! Teams application state
//!
//! Shared state for team routes.

use std::sync::Arc;
use std::path::PathBuf;
use parking_lot::RwLock as ParkingRwLock;

use crate::config::ApiConfig;
use crate::services::agent_team_service::AgentTeamService;
use crate::services::team_repository::SqliteTeamRepository;
use crate::services::team_service::TeamService;
use crate::services::telegram_service::TelegramService;
use crate::services::ai_provider_service::ProviderService;

/// Teams application state
#[derive(Clone)]
pub struct TeamsAppState {
    pub team_service: TeamService,
    pub agent_team_service: AgentTeamService,
    pub telegram_service: TelegramService,
}

impl TeamsAppState {
    /// Create new teams state - called after AppState is created to pass real dependencies
    pub fn new(
        team_service: TeamService,
        telegram_service: TelegramService,
        ai_manager: Arc<nexus_ai::AIModelManager>,
        provider_service: Option<Arc<ProviderService>>,
        current_workspace_path: Arc<ParkingRwLock<Option<String>>>,
    ) -> Self {
        let agents_dir = std::env::var("AGENTS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                std::env::current_dir()
                    .unwrap_or_default()
                    .join(".claude/agents")
            });
        let skill_service = crate::services::SkillService::with_agents_dir(agents_dir)
            .unwrap_or_else(|_| crate::services::SkillService::default());

        let agent_team_service = if let Some(ps) = provider_service.clone() {
            AgentTeamService::with_provider_service(
                team_service.clone(),
                skill_service,
                telegram_service.clone(),
                ai_manager,
                ps,
                current_workspace_path.clone(),
            )
        } else {
            AgentTeamService::new(
                team_service.clone(),
                skill_service,
                telegram_service.clone(),
                ai_manager,
                current_workspace_path.clone(),
            )
        };

        // Start background workers (Telegram message handler)
        agent_team_service.start_workers();

        Self {
            team_service,
            agent_team_service,
            telegram_service,
        }
    }

    /// Create new teams state with pre-created AgentTeamService (for sharing with ProjectService)
    pub fn new_with_agent(
        team_service: TeamService,
        telegram_service: TelegramService,
        ai_manager: Arc<nexus_ai::AIModelManager>,
        agent_team_service: Arc<AgentTeamService>,
    ) -> Self {
        Self {
            team_service,
            agent_team_service: agent_team_service.as_ref().clone(),
            telegram_service,
        }
    }

    /// Create new teams state with pre-created AgentTeamService and memory state
    pub fn new_with_agent_and_memory(
        team_service: TeamService,
        telegram_service: TelegramService,
        ai_manager: Arc<nexus_ai::AIModelManager>,
        agent_team_service: Arc<AgentTeamService>,
        memory_state: Arc<crate::routes::memory::MemoryState>,
    ) -> Self {
        let mut agent = agent_team_service.as_ref().clone();
        agent.set_memory_state(memory_state);
        Self {
            team_service,
            agent_team_service: agent,
            telegram_service,
        }
    }
}