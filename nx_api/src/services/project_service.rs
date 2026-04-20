//! Project service
//!
//! Business logic for project management and team execution.

use std::collections::HashMap;
use std::sync::Arc;
use chrono::Utc;
use thiserror::Error;

use crate::models::project::{
    Project, ProjectStatus, ProjectWithTeam, ExecuteProjectRequest, ExecuteProjectResponse, ProjectMessage,
    CreateProjectRequest, UpdateProjectRequest,
};
use crate::services::project_repository::{ProjectRepository, RepositoryError, SqliteProjectRepository};

#[derive(Error, Debug)]
pub enum ProjectError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),

    #[error("Project not found: {0}")]
    NotFound(String),

    #[error("Team not found: {0}")]
    TeamNotFound(String),

    #[error("Execution error: {0}")]
    ExecutionError(String),
}

pub struct ProjectService {
    project_repo: Arc<dyn ProjectRepository>,
    team_service: Arc<crate::services::TeamService>,
    agent_team_service: Arc<crate::services::AgentTeamService>,
    workspace_service: Arc<crate::services::WorkspaceService>,
}

impl ProjectService {
    pub fn new(
        project_repo: Arc<dyn ProjectRepository>,
        team_service: Arc<crate::services::TeamService>,
        agent_team_service: Arc<crate::services::AgentTeamService>,
        workspace_service: Arc<crate::services::WorkspaceService>,
    ) -> Self {
        Self {
            project_repo,
            team_service,
            agent_team_service,
            workspace_service,
        }
    }

    pub fn create_project(&self, req: CreateProjectRequest) -> Result<Project, ProjectError> {
        let project = Project::new(
            req.name,
            req.description,
            req.team_id,
            req.workspace_id,
            req.workflow_id,
        );
        self.project_repo.create(&project)?;
        Ok(project)
    }

    pub fn get_project(&self, id: &str) -> Result<Option<Project>, ProjectError> {
        let project = self.project_repo.find_by_id(id)?;
        Ok(project)
    }

    pub fn list_projects(&self) -> Result<Vec<Project>, ProjectError> {
        let projects = self.project_repo.find_all()?;
        Ok(projects)
    }

    pub fn list_projects_by_team(&self, team_id: &str) -> Result<Vec<Project>, ProjectError> {
        let projects = self.project_repo.find_by_team(team_id)?;
        Ok(projects)
    }

    pub fn update_project(&self, id: &str, req: UpdateProjectRequest) -> Result<Project, ProjectError> {
        let mut project = self.project_repo.find_by_id(id)?
            .ok_or_else(|| ProjectError::NotFound(id.to_string()))?;

        if let Some(name) = req.name {
            project.name = name;
        }
        if let Some(description) = req.description {
            project.description = description;
        }
        if let Some(team_id) = req.team_id {
            project.team_id = team_id;
        }
        if let Some(workspace_id) = req.workspace_id {
            project.workspace_id = Some(workspace_id);
        }
        if let Some(workflow_id) = req.workflow_id {
            project.workflow_id = Some(workflow_id);
        }
        if let Some(status) = req.status {
            project.status = status;
        }
        project.updated_at = Utc::now();

        self.project_repo.update(&project)?;
        Ok(project)
    }

    pub fn delete_project(&self, id: &str) -> Result<bool, ProjectError> {
        let deleted = self.project_repo.delete(id)?;
        Ok(deleted)
    }

    pub async fn execute_project(&self, req: ExecuteProjectRequest) -> Result<ExecuteProjectResponse, ProjectError> {
        // Load project
        let project = self.project_repo.find_by_id(&req.project_id)?
            .ok_or_else(|| ProjectError::NotFound(req.project_id.clone()))?;

        // Update project status to in progress
        let mut updating_project = project.clone();
        updating_project.status = ProjectStatus::InProgress;
        updating_project.updated_at = Utc::now();
        self.project_repo.update(&updating_project)?;

        // Get team info
        let team = self.team_service.get_team(&project.team_id)
            .map_err(|e| ProjectError::TeamNotFound(e.to_string()))?;

        // Build context with project variables
        let mut context = req.context.clone();
        context.insert("project_name".to_string(), project.name.clone());
        context.insert("project_id".to_string(), project.id.clone());
        for (key, value) in project.variables.iter() {
            context.insert(key.clone(), value.clone());
        }

        // Execute team task via AgentTeamService
        let execute_req = crate::models::team::ExecuteTeamTaskRequest {
            team_id: project.team_id.clone(),
            task: req.task.clone(),
            context,
        };

        let result = self.agent_team_service.execute_team_task(execute_req, None)
            .await
            .map_err(|e| ProjectError::ExecutionError(e.to_string()))?;

        // Update project status based on result
        let mut final_project = project.clone();
        final_project.status = if result.success {
            ProjectStatus::Completed
        } else {
            ProjectStatus::Failed
        };
        final_project.updated_at = Utc::now();
        self.project_repo.update(&final_project)?;

        // Get team with roles to map role_id to role_name
        let team_with_roles = self.team_service.get_team_with_roles(&project.team_id)
            .map_err(|e| ProjectError::ExecutionError(e.to_string()))?;

        // Convert messages
        let messages: Vec<ProjectMessage> = result.messages.into_iter().map(|m| {
            let role_name = m.role_id.as_ref().and_then(|rid| {
                team_with_roles.roles.iter().find(|r| r.role.id == *rid).map(|r| r.role.name.clone())
            });
            ProjectMessage {
                id: m.id,
                project_id: project.id.clone(),
                role_id: m.role_id,
                role_name,
                content: m.content,
                message_type: m.message_type.as_str().to_string(),
                created_at: m.created_at,
            }
        }).collect();

        Ok(ExecuteProjectResponse {
            success: result.success,
            project_id: project.id,
            team_id: project.team_id,
            messages,
            final_output: result.final_output,
            error: result.error,
        })
    }

    pub fn get_project_with_team(&self, id: &str) -> Result<Option<ProjectWithTeam>, ProjectError> {
        let project = self.project_repo.find_by_id(id)?
            .ok_or_else(|| ProjectError::NotFound(id.to_string()))?;

        let team = self.team_service.get_team(&project.team_id)
            .map_err(|e| ProjectError::TeamNotFound(e.to_string()))?;

        // Get workspace info if workspace_id is set
        let (workspace_name, workspace_path) = if let Some(ref workspace_id) = project.workspace_id {
            match self.workspace_service.get_workspace(workspace_id) {
                Ok(Some(workspace)) => (Some(workspace.name), workspace.root_path),
                Ok(None) => (None, None),
                Err(_) => (None, None),
            }
        } else {
            (None, None)
        };

        Ok(Some(ProjectWithTeam {
            project,
            team_name: team.name,
            workflow_name: None,
            workspace_name,
            workspace_path,
        }))
    }
}
