//! Team Architecture v2 - Role-based agent collaboration

use crate::cli::{CliProvider, CliTokenUsage};
use crate::error::TeamError;
use crate::message_bus::{Channel, MessageBus, MessagePayload};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Team identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TeamId(pub Uuid);

impl TeamId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TeamId {
    fn default() -> Self {
        Self::new()
    }
}

/// Agent identifier within a team
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub Uuid);

impl AgentId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

/// Agent role in the team
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    Leader,
    Architect,
    Developer,
    Reviewer,
    Tester,
    Researcher,
    Executor,
}

impl AgentRole {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "leader" => Self::Leader,
            "architect" => Self::Architect,
            "developer" => Self::Developer,
            "reviewer" => Self::Reviewer,
            "tester" => Self::Tester,
            "researcher" => Self::Researcher,
            "executor" => Self::Executor,
            _ => Self::Developer,
        }
    }

    pub fn default_prompt(&self) -> &'static str {
        match self {
            Self::Leader => "You are the team leader coordinating a multi-agent workflow.",
            Self::Architect => "You are the architect designing system architecture and plans.",
            Self::Developer => "You are the developer implementing code solutions.",
            Self::Reviewer => "You are the code reviewer ensuring quality and best practices.",
            Self::Tester => "You are the test engineer creating comprehensive tests.",
            Self::Researcher => "You are the researcher gathering and analyzing information.",
            Self::Executor => "You are the executor running commands and validating results.",
        }
    }
}

/// Agent capabilities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Planning,
    CodeGeneration,
    CodeReview,
    Refactoring,
    TestGeneration,
    Documentation,
    Research,
    Execution,
    Analysis,
}

/// Agent status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Idle,
    Initializing,
    Running,
    Waiting,
    Completed,
    Failed,
}

/// Team member definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMember {
    pub id: AgentId,
    pub name: String,
    pub role: AgentRole,
    pub provider: CliProvider,
    pub model: String,
    pub capabilities: Vec<Capability>,
    pub max_iterations: usize,
    pub timeout_secs: u64,
}

impl TeamMember {
    pub fn new(role: AgentRole, name: &str, provider: CliProvider) -> Self {
        Self {
            id: AgentId::new(),
            name: name.to_string(),
            role,
            provider,
            model: "default".to_string(),
            capabilities: Self::capabilities_for_role(role),
            max_iterations: 10,
            timeout_secs: 300,
        }
    }

    fn capabilities_for_role(role: AgentRole) -> Vec<Capability> {
        match role {
            AgentRole::Leader => vec![Capability::Planning, Capability::Analysis],
            AgentRole::Architect => vec![
                Capability::Planning,
                Capability::Analysis,
                Capability::Documentation,
            ],
            AgentRole::Developer => vec![Capability::CodeGeneration, Capability::Refactoring],
            AgentRole::Reviewer => vec![Capability::CodeReview, Capability::Analysis],
            AgentRole::Tester => vec![Capability::TestGeneration, Capability::Documentation],
            AgentRole::Researcher => vec![Capability::Research, Capability::Analysis],
            AgentRole::Executor => vec![Capability::Execution],
        }
    }
}

/// Communication mode between agents
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommunicationMode {
    Hierarchical,
    PeerToPeer,
    MessageBus,
    Hybrid,
}

/// Team definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    pub id: TeamId,
    pub name: String,
    pub members: HashMap<AgentId, TeamMember>,
    pub hierarchy: HashMap<AgentId, Vec<AgentId>>,
    pub communication_mode: CommunicationMode,
    pub created_at: DateTime<Utc>,
}

impl Team {
    pub fn new(name: &str) -> Self {
        Self {
            id: TeamId::new(),
            name: name.to_string(),
            members: HashMap::new(),
            hierarchy: HashMap::new(),
            communication_mode: CommunicationMode::Hierarchical,
            created_at: Utc::now(),
        }
    }

    pub fn add_member(&mut self, member: TeamMember) {
        self.members.insert(member.id, member);
    }

    pub fn set_leader(&mut self, agent_id: AgentId) {
        self.hierarchy.insert(agent_id, Vec::new());
    }

    pub fn add_dependency(&mut self, leader: AgentId, subordinate: AgentId) {
        self.hierarchy.entry(leader).or_default().push(subordinate);
    }
}

/// Task assigned to an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub description: String,
    pub prompt: String,
    pub depends_on: Vec<Uuid>,
    pub status: TaskStatus,
    pub result: Option<TaskResult>,
}

impl Task {
    pub fn new(description: &str, prompt: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            description: description.to_string(),
            prompt: prompt.to_string(),
            depends_on: Vec::new(),
            status: TaskStatus::Pending,
            result: None,
        }
    }
}

/// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

/// Task result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub text: String,
    pub artifacts: Vec<String>,
    pub usage: Option<CliTokenUsage>,
}

/// Team manager handles team lifecycle and coordination
pub struct TeamManager {
    teams: RwLock<HashMap<TeamId, Team>>,
    agents: RwLock<HashMap<AgentId, AgentStatus>>,
    message_bus: Arc<MessageBus>,
}

impl TeamManager {
    pub fn new(message_bus: Arc<MessageBus>) -> Self {
        Self {
            teams: RwLock::new(HashMap::new()),
            agents: RwLock::new(HashMap::new()),
            message_bus,
        }
    }

    /// Create a new team
    pub fn create_team(&self, name: &str) -> TeamId {
        let team = Team::new(name);
        let team_id = team.id;
        self.teams.write().insert(team_id, team);
        let _ = self.message_bus.publish(
            Channel::SystemEvents,
            MessagePayload::TeamCreated { team_id },
        );
        team_id
    }

    /// Get a team by ID
    pub fn get_team(&self, team_id: TeamId) -> Option<Team> {
        self.teams.read().get(&team_id).cloned()
    }

    /// Add a member to a team
    pub fn add_member(&self, team_id: TeamId, member: TeamMember) -> Result<(), TeamError> {
        let mut teams = self.teams.write();
        let team = teams
            .get_mut(&team_id)
            .ok_or(TeamError::TeamNotFound(team_id))?;
        team.add_member(member.clone());
        self.agents.write().insert(member.id, AgentStatus::Idle);
        Ok(())
    }

    /// Create a standard development team
    pub fn create_dev_team(&self, name: &str) -> TeamId {
        let team_id = self.create_team(name);

        // Create members first to capture IDs
        let architect = TeamMember::new(AgentRole::Architect, "architect", CliProvider::Claude);
        let developer = TeamMember::new(AgentRole::Developer, "developer", CliProvider::Claude);
        let reviewer = TeamMember::new(AgentRole::Reviewer, "reviewer", CliProvider::Claude);
        let tester = TeamMember::new(AgentRole::Tester, "tester", CliProvider::Claude);

        // Capture IDs before adding to team (which takes ownership)
        let architect_id = architect.id;
        let developer_id = developer.id;
        let reviewer_id = reviewer.id;
        let tester_id = tester.id;

        let mut teams = self.teams.write();
        let team = teams.get_mut(&team_id).unwrap();

        team.add_member(architect);
        team.add_member(developer);
        team.add_member(reviewer);
        team.add_member(tester);

        // Set up hierarchy
        team.set_leader(architect_id);
        team.add_dependency(architect_id, developer_id);
        team.add_dependency(developer_id, reviewer_id);
        team.add_dependency(reviewer_id, tester_id);

        drop(teams);

        // Initialize agent statuses
        let mut agents = self.agents.write();
        if let Some(team) = self.teams.read().get(&team_id) {
            for member in team.members.values() {
                agents.insert(member.id, AgentStatus::Idle);
            }
        }

        team_id
    }

    /// Get agent status
    pub fn get_agent_status(&self, agent_id: AgentId) -> Option<AgentStatus> {
        self.agents.read().get(&agent_id).copied()
    }

    /// Update agent status
    pub fn update_agent_status(&self, agent_id: AgentId, status: AgentStatus) {
        self.agents.write().insert(agent_id, status);
    }

    /// List all teams
    pub fn list_teams(&self) -> Vec<Team> {
        self.teams.read().values().cloned().collect()
    }

    /// Dissolve a team
    pub fn dissolve_team(&self, team_id: TeamId) -> Result<(), TeamError> {
        let mut teams = self.teams.write();
        teams
            .remove(&team_id)
            .ok_or(TeamError::TeamNotFound(team_id))?;
        let _ = self.message_bus.publish(
            Channel::SystemEvents,
            MessagePayload::TeamDissolved { team_id },
        );
        Ok(())
    }
}
