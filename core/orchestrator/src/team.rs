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

    /// Update a team via closure (acquires write lock)
    pub fn update_team(
        &self,
        team_id: TeamId,
        update: impl FnOnce(&mut Team),
    ) -> Result<(), TeamError> {
        let mut teams = self.teams.write();
        let team = teams
            .get_mut(&team_id)
            .ok_or(TeamError::TeamNotFound(team_id))?;
        update(team);
        Ok(())
    }

    /// Set a team member as leader
    pub fn set_leader(&self, team_id: TeamId, agent_id: AgentId) -> Result<(), TeamError> {
        self.update_team(team_id, |team| team.set_leader(agent_id))
    }

    /// Add a dependency between agents in a team
    pub fn add_dependency(
        &self,
        team_id: TeamId,
        leader: AgentId,
        subordinate: AgentId,
    ) -> Result<(), TeamError> {
        self.update_team(team_id, |team| team.add_dependency(leader, subordinate))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message_bus::MessageBus;

    // ── AgentId ────────────────────────────────────────────────────

    #[test]
    fn agent_id_new_creates_unique_ids() {
        let a = AgentId::new();
        let b = AgentId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn agent_id_default_is_unique() {
        let a: AgentId = Default::default();
        let b: AgentId = Default::default();
        assert_ne!(a, b);
    }

    #[test]
    fn agent_id_serde_roundtrip() {
        let original = AgentId::new();
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: AgentId = serde_json::from_str(&json).unwrap();
        assert_eq!(original, deserialized);
    }

    // ── TeamId ─────────────────────────────────────────────────────

    #[test]
    fn team_id_new_creates_unique_ids() {
        let a = TeamId::new();
        let b = TeamId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn team_id_default_is_unique() {
        let a: TeamId = Default::default();
        let b: TeamId = Default::default();
        assert_ne!(a, b);
    }

    #[test]
    fn team_id_serde_roundtrip() {
        let original = TeamId::new();
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: TeamId = serde_json::from_str(&json).unwrap();
        assert_eq!(original, deserialized);
    }

    // ── AgentRole ──────────────────────────────────────────────────

    #[test]
    fn agent_role_from_str_maps_correctly() {
        assert_eq!(AgentRole::from_str("leader"), AgentRole::Leader);
        assert_eq!(AgentRole::from_str("architect"), AgentRole::Architect);
        assert_eq!(AgentRole::from_str("developer"), AgentRole::Developer);
        assert_eq!(AgentRole::from_str("reviewer"), AgentRole::Reviewer);
        assert_eq!(AgentRole::from_str("tester"), AgentRole::Tester);
        assert_eq!(AgentRole::from_str("researcher"), AgentRole::Researcher);
        assert_eq!(AgentRole::from_str("executor"), AgentRole::Executor);
    }

    #[test]
    fn agent_role_from_str_case_insensitive() {
        assert_eq!(AgentRole::from_str("Architect"), AgentRole::Architect);
        assert_eq!(AgentRole::from_str("DEVELOPER"), AgentRole::Developer);
        assert_eq!(AgentRole::from_str("Tester"), AgentRole::Tester);
    }

    #[test]
    fn agent_role_from_str_unknown_falls_back_to_developer() {
        assert_eq!(AgentRole::from_str("unknown"), AgentRole::Developer);
        assert_eq!(AgentRole::from_str(""), AgentRole::Developer);
        assert_eq!(AgentRole::from_str("manager"), AgentRole::Developer);
    }

    #[test]
    fn agent_role_default_prompt_all_variants_non_empty() {
        for role in &[
            AgentRole::Leader,
            AgentRole::Architect,
            AgentRole::Developer,
            AgentRole::Reviewer,
            AgentRole::Tester,
            AgentRole::Researcher,
            AgentRole::Executor,
        ] {
            let prompt = role.default_prompt();
            assert!(
                !prompt.is_empty(),
                "role {:?} should have a default prompt",
                role
            );
        }
    }

    #[test]
    fn agent_role_serde_roundtrip() {
        for role in &[
            AgentRole::Leader,
            AgentRole::Architect,
            AgentRole::Developer,
            AgentRole::Reviewer,
            AgentRole::Tester,
            AgentRole::Researcher,
            AgentRole::Executor,
        ] {
            let json = serde_json::to_string(role).unwrap();
            let deserialized: AgentRole = serde_json::from_str(&json).unwrap();
            assert_eq!(*role, deserialized);
        }
    }

    #[test]
    fn agent_role_variants_distinct() {
        use std::collections::HashSet;
        let roles = vec![
            AgentRole::Leader,
            AgentRole::Architect,
            AgentRole::Developer,
            AgentRole::Reviewer,
            AgentRole::Tester,
            AgentRole::Researcher,
            AgentRole::Executor,
        ];
        let unique: HashSet<_> = roles.iter().collect();
        assert_eq!(unique.len(), 7, "all 7 roles should be distinct");
    }

    // ── TeamMember ─────────────────────────────────────────────────

    #[test]
    fn team_member_new_assigns_unique_id() {
        let m1 = TeamMember::new(AgentRole::Architect, "arch1", CliProvider::Claude);
        let m2 = TeamMember::new(AgentRole::Architect, "arch2", CliProvider::Claude);
        assert_ne!(m1.id, m2.id);
    }

    #[test]
    fn team_member_new_sets_correct_fields() {
        let m = TeamMember::new(AgentRole::Developer, "dev1", CliProvider::Claude);
        assert_eq!(m.name, "dev1");
        assert_eq!(m.role, AgentRole::Developer);
        assert_eq!(m.provider, CliProvider::Claude);
        assert_eq!(m.model, "default");
        assert_eq!(m.max_iterations, 10);
        assert_eq!(m.timeout_secs, 300);
    }

    #[test]
    fn team_member_capabilities_for_architect() {
        let m = TeamMember::new(AgentRole::Architect, "arch", CliProvider::Claude);
        assert!(m.capabilities.contains(&Capability::Planning));
        assert!(m.capabilities.contains(&Capability::Analysis));
        assert!(m.capabilities.contains(&Capability::Documentation));
        assert_eq!(m.capabilities.len(), 3);
    }

    #[test]
    fn team_member_capabilities_for_developer() {
        let m = TeamMember::new(AgentRole::Developer, "dev", CliProvider::Claude);
        assert!(m.capabilities.contains(&Capability::CodeGeneration));
        assert!(m.capabilities.contains(&Capability::Refactoring));
        assert_eq!(m.capabilities.len(), 2);
    }

    #[test]
    fn team_member_capabilities_for_reviewer() {
        let m = TeamMember::new(AgentRole::Reviewer, "rev", CliProvider::Claude);
        assert!(m.capabilities.contains(&Capability::CodeReview));
        assert!(m.capabilities.contains(&Capability::Analysis));
        assert_eq!(m.capabilities.len(), 2);
    }

    #[test]
    fn team_member_capabilities_for_tester() {
        let m = TeamMember::new(AgentRole::Tester, "test", CliProvider::Claude);
        assert!(m.capabilities.contains(&Capability::TestGeneration));
        assert!(m.capabilities.contains(&Capability::Documentation));
        assert_eq!(m.capabilities.len(), 2);
    }

    #[test]
    fn team_member_capabilities_for_researcher() {
        let m = TeamMember::new(AgentRole::Researcher, "res", CliProvider::Claude);
        assert!(m.capabilities.contains(&Capability::Research));
        assert!(m.capabilities.contains(&Capability::Analysis));
        assert_eq!(m.capabilities.len(), 2);
    }

    #[test]
    fn team_member_capabilities_for_executor() {
        let m = TeamMember::new(AgentRole::Executor, "exec", CliProvider::Claude);
        assert!(m.capabilities.contains(&Capability::Execution));
        assert_eq!(m.capabilities.len(), 1);
    }

    #[test]
    fn team_member_capabilities_for_leader() {
        let m = TeamMember::new(AgentRole::Leader, "lead", CliProvider::Claude);
        assert!(m.capabilities.contains(&Capability::Planning));
        assert!(m.capabilities.contains(&Capability::Analysis));
        assert_eq!(m.capabilities.len(), 2);
    }

    // ── Team ───────────────────────────────────────────────────────

    #[test]
    fn team_new_creates_empty_team() {
        let team = Team::new("test-team");
        assert_eq!(team.name, "test-team");
        assert!(team.members.is_empty());
        assert!(team.hierarchy.is_empty());
        assert_eq!(team.communication_mode, CommunicationMode::Hierarchical);
    }

    #[test]
    fn team_add_member_increases_count() {
        let mut team = Team::new("t");
        let m = TeamMember::new(AgentRole::Developer, "dev", CliProvider::Claude);
        let id = m.id;
        team.add_member(m);
        assert_eq!(team.members.len(), 1);
        assert!(team.members.contains_key(&id));
    }

    #[test]
    fn team_add_member_duplicate_id_replaces() {
        let mut team = Team::new("t");
        let m1 = TeamMember::new(AgentRole::Architect, "arch", CliProvider::Claude);
        let id = m1.id;
        let m2 = TeamMember {
            id,
            name: "replacement".into(),
            role: AgentRole::Developer,
            provider: CliProvider::Claude,
            model: "default".into(),
            capabilities: vec![],
            max_iterations: 5,
            timeout_secs: 100,
        };
        team.add_member(m1);
        team.add_member(m2);
        assert_eq!(team.members.len(), 1);
        assert_eq!(team.members[&id].name, "replacement");
    }

    #[test]
    fn team_set_leader_creates_empty_hierarchy_entry() {
        let mut team = Team::new("t");
        let m = TeamMember::new(AgentRole::Architect, "arch", CliProvider::Claude);
        let id = m.id;
        team.add_member(m);
        team.set_leader(id);
        assert!(team.hierarchy.contains_key(&id));
        assert!(team.hierarchy[&id].is_empty());
    }

    #[test]
    fn team_set_leader_twice_does_not_duplicate() {
        let mut team = Team::new("t");
        let m = TeamMember::new(AgentRole::Architect, "arch", CliProvider::Claude);
        let id = m.id;
        team.add_member(m);
        team.set_leader(id);
        team.set_leader(id);
        assert_eq!(team.hierarchy.len(), 1);
    }

    #[test]
    fn team_add_dependency_creates_empty_entry_if_leader_missing() {
        let mut team = Team::new("t");
        let lid = AgentId::new();
        let sid = AgentId::new();
        team.add_dependency(lid, sid);
        assert!(team.hierarchy.contains_key(&lid));
        assert_eq!(team.hierarchy[&lid], vec![sid]);
    }

    #[test]
    fn team_add_dependency_accumulates_subordinates() {
        let mut team = Team::new("t");
        let lid = AgentId::new();
        let s1 = AgentId::new();
        let s2 = AgentId::new();
        team.add_dependency(lid, s1);
        team.add_dependency(lid, s2);
        assert_eq!(team.hierarchy[&lid].len(), 2);
        assert!(team.hierarchy[&lid].contains(&s1));
        assert!(team.hierarchy[&lid].contains(&s2));
    }

    // ── Task ───────────────────────────────────────────────────────

    #[test]
    fn task_new_creates_pending_task() {
        let task = Task::new("do something", "prompt here");
        assert_eq!(task.description, "do something");
        assert_eq!(task.prompt, "prompt here");
        assert!(task.depends_on.is_empty());
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(task.result.is_none());
    }

    #[test]
    fn task_new_unique_ids() {
        let t1 = Task::new("a", "pa");
        let t2 = Task::new("b", "pb");
        assert_ne!(t1.id, t2.id);
    }

    // ── TeamManager ────────────────────────────────────────────────

    fn make_manager() -> TeamManager {
        TeamManager::new(Arc::new(MessageBus::new()))
    }

    #[test]
    fn manager_create_team_returns_valid_id() {
        let mgr = make_manager();
        let id = mgr.create_team("test");
        assert!(mgr.get_team(id).is_some());
    }

    #[test]
    fn manager_create_team_increases_count() {
        let mgr = make_manager();
        mgr.create_team("a");
        mgr.create_team("b");
        assert_eq!(mgr.list_teams().len(), 2);
    }

    #[test]
    fn manager_get_team_nonexistent() {
        let mgr = make_manager();
        let id = TeamId::new();
        assert!(mgr.get_team(id).is_none());
    }

    #[test]
    fn manager_add_member_to_team() {
        let mgr = make_manager();
        let tid = mgr.create_team("t");
        let member = TeamMember::new(AgentRole::Developer, "dev", CliProvider::Claude);
        mgr.add_member(tid, member).unwrap();

        let team = mgr.get_team(tid).unwrap();
        assert_eq!(team.members.len(), 1);
    }

    #[test]
    fn manager_add_member_to_nonexistent_team() {
        let mgr = make_manager();
        let member = TeamMember::new(AgentRole::Developer, "dev", CliProvider::Claude);
        let result = mgr.add_member(TeamId::new(), member);
        assert!(result.is_err());
        match result {
            Err(TeamError::TeamNotFound(_)) => {} // expected
            _ => panic!("expected TeamNotFound"),
        }
    }

    #[test]
    fn manager_set_leader() {
        let mgr = make_manager();
        let tid = mgr.create_team("t");
        let member = TeamMember::new(AgentRole::Architect, "arch", CliProvider::Claude);
        let mid = member.id;
        mgr.add_member(tid, member).unwrap();
        mgr.set_leader(tid, mid).unwrap();

        let team = mgr.get_team(tid).unwrap();
        assert!(team.hierarchy.contains_key(&mid));
    }

    #[test]
    fn manager_set_leader_nonexistent_team() {
        let mgr = make_manager();
        let result = mgr.set_leader(TeamId::new(), AgentId::new());
        assert!(result.is_err());
    }

    #[test]
    fn manager_add_dependency() {
        let mgr = make_manager();
        let tid = mgr.create_team("t");
        let arch = TeamMember::new(AgentRole::Architect, "arch", CliProvider::Claude);
        let dev = TeamMember::new(AgentRole::Developer, "dev", CliProvider::Claude);
        let aid = arch.id;
        let did = dev.id;
        mgr.add_member(tid, arch).unwrap();
        mgr.add_member(tid, dev).unwrap();
        mgr.set_leader(tid, aid).unwrap();
        mgr.add_dependency(tid, aid, did).unwrap();

        let team = mgr.get_team(tid).unwrap();
        assert_eq!(team.hierarchy[&aid], vec![did]);
    }

    #[test]
    fn manager_create_dev_team_has_four_members() {
        let mgr = make_manager();
        let tid = mgr.create_dev_team("dev-team");
        let team = mgr.get_team(tid).unwrap();
        assert_eq!(team.members.len(), 4);
    }

    #[test]
    fn manager_create_dev_team_has_correct_hierarchy() {
        let mgr = make_manager();
        let tid = mgr.create_dev_team("dev-team");
        let team = mgr.get_team(tid).unwrap();
        // hierarchy should have 3 entries (architect, developer→reviewer, reviewer→tester)
        assert_eq!(team.hierarchy.len(), 3);
        // find architect/developer/reviewer/tester
        let arch = team
            .members
            .values()
            .find(|m| m.role == AgentRole::Architect)
            .unwrap();
        let dev = team
            .members
            .values()
            .find(|m| m.role == AgentRole::Developer)
            .unwrap();
        assert_eq!(
            team.hierarchy[&arch.id],
            vec![dev.id],
            "architect should depend on developer"
        );
    }

    #[test]
    fn manager_agent_status_updates() {
        let mgr = make_manager();
        let tid = mgr.create_dev_team("t");
        let team = mgr.get_team(tid).unwrap();
        let member_id = team.members.values().next().unwrap().id;

        assert_eq!(mgr.get_agent_status(member_id), Some(AgentStatus::Idle));
        mgr.update_agent_status(member_id, AgentStatus::Running);
        assert_eq!(mgr.get_agent_status(member_id), Some(AgentStatus::Running));
        mgr.update_agent_status(member_id, AgentStatus::Completed);
        assert_eq!(
            mgr.get_agent_status(member_id),
            Some(AgentStatus::Completed)
        );
    }

    #[test]
    fn manager_get_agent_status_nonexistent() {
        let mgr = make_manager();
        assert!(mgr.get_agent_status(AgentId::new()).is_none());
    }

    #[test]
    fn manager_dissolve_team() {
        let mgr = make_manager();
        let tid = mgr.create_team("t");
        assert!(mgr.get_team(tid).is_some());

        mgr.dissolve_team(tid).unwrap();
        assert!(mgr.get_team(tid).is_none());
        assert_eq!(mgr.list_teams().len(), 0);
    }

    #[test]
    fn manager_dissolve_nonexistent_team() {
        let mgr = make_manager();
        let result = mgr.dissolve_team(TeamId::new());
        assert!(result.is_err());
        match result {
            Err(TeamError::TeamNotFound(_)) => {} // expected
            _ => panic!("expected TeamNotFound"),
        }
    }

    #[test]
    fn manager_list_teams_empty_initially() {
        let mgr = make_manager();
        assert!(mgr.list_teams().is_empty());
    }

    #[test]
    fn manager_multiple_teams_independent() {
        let mgr = make_manager();
        let t1 = mgr.create_team("team1");
        let t2 = mgr.create_team("team2");

        let m1 = TeamMember::new(AgentRole::Developer, "d1", CliProvider::Claude);
        let m2 = TeamMember::new(AgentRole::Developer, "d2", CliProvider::Claude);
        mgr.add_member(t1, m1).unwrap();
        mgr.add_member(t2, m2).unwrap();

        let team1 = mgr.get_team(t1).unwrap();
        let team2 = mgr.get_team(t2).unwrap();
        assert_eq!(team1.members.len(), 1);
        assert_eq!(team2.members.len(), 1);
        assert_ne!(team1.members.keys().next(), team2.members.keys().next());
    }
}
