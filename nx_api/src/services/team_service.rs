//! Team service
//!
//! Business logic for team management, role management, and message persistence.

use std::sync::Arc;
use thiserror::Error;

use crate::models::team::{
    CreateRoleRequest, CreateTeamRequest, ModelConfig, RoleSkill, SkillPriority, Team,
    TeamMessage, TeamRole, TelegramBotConfig, UpdateRoleRequest, UpdateTeamRequest,
};

use super::team_repository::{TeamRepository, TeamRepositoryError};

/// Team service error
#[derive(Debug, Error)]
pub enum TeamServiceError {
    #[error("Team not found: {0}")]
    TeamNotFound(String),

    #[error("Role not found: {0}")]
    RoleNotFound(String),

    #[error("Telegram config not found for role: {0}")]
    TelegramConfigNotFound(String),

    #[error("Repository error: {0}")]
    Repository(#[from] TeamRepositoryError),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}

/// Team with roles for detailed view
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TeamWithRoles {
    pub team: Team,
    pub roles: Vec<RoleWithSkills>,
}

/// Role with assigned skills
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoleWithSkills {
    pub role: TeamRole,
    pub skills: Vec<RoleSkill>,
}

/// Team service
#[derive(Clone)]
pub struct TeamService {
    repository: Arc<dyn TeamRepository>,
}

impl TeamService {
    /// Create new team service
    pub fn new(repository: Arc<dyn TeamRepository>) -> Self {
        Self { repository }
    }

    // Team CRUD
    pub fn create_team(&self, request: CreateTeamRequest) -> Result<Team, TeamServiceError> {
        let team = Team::new(request.name, request.description.unwrap_or_default());
        self.repository.create_team(&team)?;
        Ok(team)
    }

    pub fn get_team(&self, id: &str) -> Result<Team, TeamServiceError> {
        self.repository
            .find_team_by_id(id)?
            .ok_or_else(|| TeamServiceError::TeamNotFound(id.to_string()))
    }

    pub fn list_teams(&self) -> Result<Vec<Team>, TeamServiceError> {
        Ok(self.repository.find_all_teams()?)
    }

    pub fn get_team_with_roles(&self, id: &str) -> Result<TeamWithRoles, TeamServiceError> {
        let team = self.get_team(id)?;
        let roles = self.repository.find_roles_by_team(id)?;

        let roles_with_skills: Vec<RoleWithSkills> = roles
            .into_iter()
            .map(|role| {
                let skills = self.repository.find_skills_by_role(&role.id).unwrap_or_default();
                RoleWithSkills { role, skills }
            })
            .collect();

        Ok(TeamWithRoles {
            team,
            roles: roles_with_skills,
        })
    }

    pub fn update_team(
        &self,
        id: &str,
        request: UpdateTeamRequest,
    ) -> Result<Team, TeamServiceError> {
        let mut team = self.get_team(id)?;

        if let Some(name) = request.name {
            team.name = name;
        }
        if let Some(description) = request.description {
            team.description = description;
        }
        team.updated_at = chrono::Utc::now();

        self.repository.update_team(&team)?;
        Ok(team)
    }

    pub fn delete_team(&self, id: &str) -> Result<bool, TeamServiceError> {
        // Verify team exists
        let _ = self.get_team(id)?;
        self.repository.delete_team(id).map_err(Into::into)
    }

    // Role CRUD
    pub fn create_role(
        &self,
        team_id: &str,
        request: CreateRoleRequest,
    ) -> Result<TeamRole, TeamServiceError> {
        // Verify team exists
        let _ = self.get_team(team_id)?;

        let model_config = request.model_config.unwrap_or(ModelConfig::default());
        // Create role with NULL team_id (global/shared role)
        // Role will be associated with team via junction table
        let trigger_keywords = request.trigger_keywords.clone().unwrap_or_default();
        let role = TeamRole::new(
            None,  // team_id is NULL - role is global
            request.name,
            request.description,
            model_config,
            request.system_prompt,
            trigger_keywords,
        );

        self.repository.create_role(&role)?;

        // Associate role with team via junction table
        self.repository.add_role_to_team(&role.id, team_id)
            .map_err(|e| TeamServiceError::Repository(e))?;

        Ok(role)
    }

    pub fn get_role(&self, id: &str) -> Result<TeamRole, TeamServiceError> {
        self.repository
            .find_role_by_id(id)?
            .ok_or_else(|| TeamServiceError::RoleNotFound(id.to_string()))
    }

    pub fn get_role_with_skills(&self, id: &str) -> Result<RoleWithSkills, TeamServiceError> {
        let role = self.get_role(id)?;
        let skills = self.repository.find_skills_by_role(id).unwrap_or_default();
        Ok(RoleWithSkills { role, skills })
    }

    pub fn list_roles(&self, team_id: &str) -> Result<Vec<TeamRole>, TeamServiceError> {
        // Verify team exists
        let _ = self.get_team(team_id)?;
        self.repository.find_roles_by_team(team_id).map_err(Into::into)
    }

    pub fn list_all_roles(&self) -> Result<Vec<TeamRole>, TeamServiceError> {
        let teams = self.list_teams()?;
        let mut all_roles = Vec::new();
        for team in teams {
            if let Ok(roles) = self.repository.find_roles_by_team(&team.id) {
                all_roles.extend(roles);
            }
        }
        Ok(all_roles)
    }

    pub fn update_role(
        &self,
        id: &str,
        request: UpdateRoleRequest,
    ) -> Result<TeamRole, TeamServiceError> {
        let mut role = self.get_role(id)?;

        if let Some(name) = request.name {
            role.name = name;
        }
        if let Some(description) = request.description {
            role.description = description;
        }
        if let Some(model_config) = request.model_config {
            role.model_config = model_config;
        }
        if let Some(system_prompt) = request.system_prompt {
            role.system_prompt = system_prompt;
        }
        role.updated_at = chrono::Utc::now();

        self.repository.update_role(&role)?;
        Ok(role)
    }

    pub fn delete_role(&self, id: &str) -> Result<bool, TeamServiceError> {
        // Verify role exists
        let _ = self.get_role(id)?;
        self.repository.delete_role(id).map_err(Into::into)
    }

    pub fn assign_role_to_team(&self, role_id: &str, team_id: &str) -> Result<TeamRole, TeamServiceError> {
        // Verify team exists
        let _ = self.get_team(team_id)?;
        // Verify role exists
        let _ = self.get_role(role_id)?;
        // Add role to team via junction table
        self.repository.add_role_to_team(role_id, team_id)
            .map_err(|e| TeamServiceError::Repository(e))?;
        // Return the updated role
        self.get_role(role_id)
    }

    pub fn remove_role_from_team(&self, team_id: &str, role_id: &str) -> Result<bool, TeamServiceError> {
        // Verify team exists
        let _ = self.get_team(team_id)?;
        // Verify role exists
        let _ = self.get_role(role_id)?;
        // Remove role from team via junction table (only removes assignment, doesn't delete the role)
        self.repository.remove_role_from_team(role_id, team_id)
            .map_err(|e| TeamServiceError::Repository(e))
    }

    // Skill management
    pub fn assign_skill(
        &self,
        role_id: &str,
        skill_id: &str,
        priority: Option<SkillPriority>,
    ) -> Result<RoleSkill, TeamServiceError> {
        // Verify role exists
        let _ = self.get_role(role_id)?;

        let priority = priority.unwrap_or(SkillPriority::Medium);
        self.repository
            .assign_skill(role_id, skill_id, priority)?;

        Ok(RoleSkill {
            role_id: role_id.to_string(),
            skill_id: skill_id.to_string(),
            priority,
        })
    }

    pub fn remove_skill(&self, role_id: &str, skill_id: &str) -> Result<bool, TeamServiceError> {
        // Verify role exists
        let _ = self.get_role(role_id)?;
        self.repository
            .remove_skill(role_id, skill_id)
            .map_err(Into::into)
    }

    pub fn get_role_skills(&self, role_id: &str) -> Result<Vec<RoleSkill>, TeamServiceError> {
        // Verify role exists
        let _ = self.get_role(role_id)?;
        self.repository
            .find_skills_by_role(role_id)
            .map_err(Into::into)
    }

    // Message management
    pub fn add_message(&self, message: TeamMessage) -> Result<TeamMessage, TeamServiceError> {
        // Verify team exists
        let _ = self.get_team(&message.team_id)?;
        self.repository.create_message(&message)?;
        Ok(message)
    }

    pub fn get_team_messages(
        &self,
        team_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<TeamMessage>, TeamServiceError> {
        // Verify team exists
        let _ = self.get_team(team_id)?;
        self.repository
            .find_messages_by_team(team_id, limit)
            .map_err(Into::into)
    }

    pub fn get_role_messages(
        &self,
        role_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<TeamMessage>, TeamServiceError> {
        // Verify role exists
        let _ = self.get_role(role_id)?;
        self.repository
            .find_messages_by_role(role_id, limit)
            .map_err(Into::into)
    }

    // Telegram config management
    pub fn configure_telegram(
        &self,
        role_id: &str,
        bot_token: String,
        chat_id: Option<String>,
        notifications_enabled: Option<bool>,
        conversation_enabled: Option<bool>,
    ) -> Result<TelegramBotConfig, TeamServiceError> {
        // Verify role exists
        let _ = self.get_role(role_id)?;

        let mut config = self
            .repository
            .find_telegram_config_by_role(role_id)?
            .unwrap_or_else(|| TelegramBotConfig::new(role_id.to_string(), bot_token.clone()));

        config.bot_token = bot_token;
        if let Some(chat_id) = chat_id {
            config.chat_id = Some(chat_id);
        }
        if let Some(enabled) = notifications_enabled {
            config.notifications_enabled = enabled;
        }
        if let Some(enabled) = conversation_enabled {
            config.conversation_enabled = enabled;
        }

        self.repository.upsert_telegram_config(&config)?;
        Ok(config)
    }

    pub fn get_telegram_config(
        &self,
        role_id: &str,
    ) -> Result<TelegramBotConfig, TeamServiceError> {
        // Verify role exists
        let _ = self.get_role(role_id)?;
        self.repository
            .find_telegram_config_by_role(role_id)?
            .ok_or_else(|| TeamServiceError::TelegramConfigNotFound(role_id.to_string()))
    }

    pub fn enable_telegram(&self, role_id: &str, enabled: bool) -> Result<TelegramBotConfig, TeamServiceError> {
        let mut config = self.get_telegram_config(role_id)?;
        config.enabled = enabled;
        self.repository.upsert_telegram_config(&config)?;
        Ok(config)
    }

    pub fn delete_telegram_config(&self, role_id: &str) -> Result<bool, TeamServiceError> {
        // Verify role exists
        let _ = self.get_role(role_id)?;
        self.repository
            .delete_telegram_config(role_id)
            .map_err(Into::into)
    }
}