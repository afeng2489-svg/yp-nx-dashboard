//! Agent team service
//!
//! Multi-agent orchestration for team collaboration.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::broadcast;
use tokio::time::timeout;
use parking_lot::RwLock as ParkingRwLock;

use nexus_ai::AIModelManager;
use nx_memory::types::{MessageRole, Transcript, MemoryChunk};

use crate::models::team::{
    ExecuteRoleTaskRequest, ExecuteRoleTaskResponse, ExecuteTeamTaskRequest,
    ExecuteTeamTaskResponse, RoleSkill, SkillPriority, TeamMessage, TeamRole,
};
use crate::routes::memory::MemoryState;
use crate::services::skill_service::SkillService;
use crate::services::telegram_service::{InboundTelegramMessage, TelegramService};
use crate::services::team_service::TeamService;
use crate::services::ai_provider_service::ProviderService;

/// Agent team service error
#[derive(Debug, Error)]
pub enum AgentTeamServiceError {
    #[error("Team not found: {0}")]
    TeamNotFound(String),

    #[error("Role not found: {0}")]
    RoleNotFound(String),

    #[error("AI execution failed: {0}")]
    AiError(String),

    #[error("Service error: {0}")]
    Service(String),

    #[error("Telegram error: {0}")]
    Telegram(String),
}

/// Role execution context built from skills
#[derive(Debug, Clone)]
struct RoleExecutionContext {
    role: TeamRole,
    skills: Vec<RoleSkill>,
    skill_contexts: Vec<String>,
}

/// Agent team service for multi-agent orchestration
pub struct AgentTeamService {
    team_service: TeamService,
    skill_service: SkillService,
    telegram_service: TelegramService,
    ai_manager: Arc<AIModelManager>,
    provider_service: Option<Arc<ProviderService>>,
    /// 当前工作区路径，用于 Claude CLI 当前目录参数
    current_workspace_path: Arc<ParkingRwLock<Option<String>>>,
    /// 记忆状态（用于后台任务存储消息到记忆）
    /// 使用 Arc<Option<Arc<...>>> 避免循环引用
    memory_state: Arc<ParkingRwLock<Option<Arc<crate::routes::memory::MemoryState>>>>,
}

impl Clone for AgentTeamService {
    fn clone(&self) -> Self {
        Self {
            team_service: self.team_service.clone(),
            skill_service: self.skill_service.clone(),
            telegram_service: self.telegram_service.clone(),
            ai_manager: Arc::clone(&self.ai_manager),
            provider_service: self.provider_service.clone(),
            current_workspace_path: self.current_workspace_path.clone(),
            memory_state: Arc::clone(&self.memory_state),
        }
    }
}

impl AgentTeamService {
    /// Create new agent team service (without starting background workers)
    pub fn new(
        team_service: TeamService,
        skill_service: SkillService,
        telegram_service: TelegramService,
        ai_manager: Arc<AIModelManager>,
        current_workspace_path: Arc<ParkingRwLock<Option<String>>>,
    ) -> Self {
        Self {
            team_service,
            skill_service,
            telegram_service,
            ai_manager,
            provider_service: None,
            current_workspace_path,
            memory_state: Arc::new(ParkingRwLock::new(None)),
        }
    }

    /// Create with provider service (for accessing API keys from database)
    pub fn with_provider_service(
        team_service: TeamService,
        skill_service: SkillService,
        telegram_service: TelegramService,
        ai_manager: Arc<AIModelManager>,
        provider_service: Arc<ProviderService>,
        current_workspace_path: Arc<ParkingRwLock<Option<String>>>,
    ) -> Self {
        Self {
            team_service,
            skill_service,
            telegram_service,
            ai_manager,
            provider_service: Some(provider_service),
            current_workspace_path,
            memory_state: Arc::new(ParkingRwLock::new(None)),
        }
    }

    /// Set memory state (for storing messages to memory in background tasks)
    pub fn set_memory_state(&self, memory_state: Arc<crate::routes::memory::MemoryState>) {
        *self.memory_state.write() = Some(memory_state);
    }

    /// Store a message to memory (internal helper for background tasks)
    /// Takes ownership of Arc<MemoryState> to avoid holding references across await
    async fn store_to_memory_internal(
        memory_state: Arc<MemoryState>,
        team_id: &str,
        role: &str,
        content: &str,
    ) -> Result<(), String> {
        // Convert role string to MessageRole
        let message_role = match role {
            "user" => MessageRole::User,
            "assistant" => MessageRole::Assistant,
            "system" => MessageRole::System,
            _ => MessageRole::User,
        };

        // Create transcript
        let transcript = Transcript::new(team_id, "claude-code", message_role, content);

        // Store transcript
        memory_state
            .store
            .store_transcript(&transcript)
            .map_err(|e| format!("Failed to store transcript: {}", e))?;

        // Create chunk from transcript
        let chunk = MemoryChunk::from_transcript(&transcript, transcript.content.clone(), 0);

        // Store chunk
        memory_state
            .store
            .store_chunk(&chunk)
            .map_err(|e| format!("Failed to store chunk: {}", e))?;

        // Index chunk (generate embedding if provider available)
        let metadata = serde_json::to_value(&transcript.metadata).unwrap_or_default();
        memory_state
            .search
            .index_chunk(team_id, &chunk.id, &chunk.content, metadata)
            .await
            .map_err(|e| format!("Failed to index chunk: {}", e))?;

        Ok(())
    }

    /// Start background workers (call once after construction)
    pub fn start_workers(&self) {
        self.spawn_telegram_handler();
    }

    /// Spawn background task that listens for Telegram messages and processes them
    fn spawn_telegram_handler(&self) {
        let mut receiver = self.telegram_service.subscribe();

        let team_service = self.team_service.clone();
        let skill_service = self.skill_service.clone();
        let ai_manager = Arc::clone(&self.ai_manager);
        let provider_service = self.provider_service.clone();
        let current_workspace_path = self.current_workspace_path.clone();
        let memory_state = Arc::clone(&self.memory_state);

        tokio::spawn(async move {
            while let Ok(message) = receiver.recv().await {
                let handler = Self {
                    team_service: team_service.clone(),
                    skill_service: skill_service.clone(),
                    telegram_service: TelegramService::new(),
                    ai_manager: Arc::clone(&ai_manager),
                    provider_service: provider_service.clone(),
                    current_workspace_path: current_workspace_path.clone(),
                    memory_state: Arc::clone(&memory_state),
                };

                if let Err(e) = handler.handle_telegram_message(message).await {
                    tracing::error!("Failed to handle Telegram message: {}", e);
                }
            }
        });
    }

    /// Execute a task across a team
    ///
    /// Sends team/role/skill context to Claude in ONE call, letting Claude decide:
    /// - If a skill matches → execute the skill chain
    /// - If no skill matches → return normal AI response
    pub async fn execute_team_task(
        &self,
        request: ExecuteTeamTaskRequest,
    ) -> Result<ExecuteTeamTaskResponse, AgentTeamServiceError> {
        // Load team
        let team = self
            .team_service
            .get_team(&request.team_id)
            .map_err(|e| AgentTeamServiceError::TeamNotFound(e.to_string()))?;

        // Load roles with skills
        let team_with_roles = self
            .team_service
            .get_team_with_roles(&request.team_id)
            .map_err(|e| AgentTeamServiceError::Service(e.to_string()))?;

        if team_with_roles.roles.is_empty() {
            return Ok(ExecuteTeamTaskResponse {
                success: false,
                team_id: request.team_id,
                messages: vec![],
                final_output: String::new(),
                error: Some("No roles defined in team".to_string()),
            });
        }

        // Save initial user message immediately
        let user_msg = TeamMessage::user_message(team.id.clone(), request.task.clone());
        let _ = self.team_service.add_message(user_msg.clone());

        // 提取记忆上下文
        let memory_context = request.context.get("memory_context").cloned().unwrap_or_default();

        // Build team context for Claude
        let team_context = Self::build_team_context(&team, &team_with_roles.roles);

        // Build the full prompt - Claude decides if any skill matches
        let full_prompt = if memory_context.is_empty() {
            format!(
                r#"You are the team dispatcher. Given the team context and user message, decide what to do.

## Team Context
{}

## User Message
{}

## Your Decision
Read the user's message and the available skills in the team context.
- If a skill's trigger keywords match the user's message → use that skill
- If no skill matches → answer the user directly as a helpful AI assistant

## Output Format
Return your response directly. If using a skill, invoke it according to its execution instructions."#,
                team_context, request.task
            )
        } else {
            format!(
                r#"You are the team dispatcher. Given the team context and user message, decide what to do.

## Team Context
{}

## User Message
{}

## Your Decision
Read the user's message and the available skills in the team context.
- If a skill's trigger keywords match the user's message → use that skill
- If no skill matches → answer the user directly as a helpful AI assistant

## Output Format
Return your response directly. If using a skill, invoke it according to its execution instructions.

{}
"#,
                team_context, request.task, memory_context
            )
        };

        // 获取当前工作区路径
        let working_dir = self.current_workspace_path.read().clone();

        // Single Claude CLI call (同步等待)
        let mut cmd = tokio::process::Command::new("claude");
        cmd.args(["-p", "--dangerously-skip-permissions", &full_prompt]);
        if let Some(ref dir) = working_dir {
            cmd.current_dir(dir);
            tracing::info!("[AgentTeam] 执行 Claude CLI，当前目录: {}", dir);
        }
        // 设置 120 秒超时，防止请求挂起
        let output = timeout(Duration::from_secs(120), cmd.output()).await;

        let response = match output {
            Ok(Ok(out)) if out.status.success() => {
                String::from_utf8_lossy(&out.stdout).trim().to_string()
            }
            Ok(Ok(out)) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                return Err(AgentTeamServiceError::AiError(stderr.to_string()));
            }
            Ok(Err(e)) => {
                return Err(AgentTeamServiceError::AiError(format!("Failed to execute Claude CLI: {}", e)));
            }
            Err(_) => {
                return Err(AgentTeamServiceError::AiError("Claude CLI execution timeout after 120 seconds".to_string()));
            }
        };

        // Save assistant message (use first role as responder if skill was used)
        let responder_id = team_with_roles.roles.first().map(|r| r.role.id.clone()).unwrap_or_default();
        let assistant_msg = TeamMessage::assistant_message(team.id.clone(), responder_id, response.clone());
        let _ = self.team_service.add_message(assistant_msg);

        // Return the actual response
        Ok(ExecuteTeamTaskResponse {
            success: true,
            team_id: request.team_id,
            messages: vec![],
            final_output: response,
            error: None,
        })
    }

    /// Build team context string for Claude
    fn build_team_context(
        team: &crate::models::team::Team,
        roles: &[crate::services::team_service::RoleWithSkills],
    ) -> String {
        let mut context = format!(
            "# {}\n{}\n\n## Available Skills\n",
            team.name, team.description
        );

        for role_with_skills in roles {
            let role = &role_with_skills.role;
            context.push_str(&format!("### {}\n", role.name));

            if !role.system_prompt.is_empty() {
                context.push_str(&format!("{}\n", role.system_prompt));
            }

            // List skills for this role
            for skill in &role_with_skills.skills {
                context.push_str(&format!("- skill: {}\n", skill.skill_id));
            }
            context.push('\n');
        }

        context
    }

    /// Build skill contexts for a role (static version for background tasks)
    async fn build_skill_contexts_for_role_static(skills: &[RoleSkill]) -> Vec<String> {
        let mut contexts = Vec::new();
        for skill in skills {
            let priority_str = skill.priority.as_str();
            let context = format!("[{}] {}", priority_str.to_uppercase(), skill.skill_id);
            contexts.push(context);
        }
        contexts
    }

    /// Execute a task for a single role (with its assigned skills)
    ///
    /// This method processes the AI request synchronously and returns the actual response.
    pub async fn execute_role_task(
        &self,
        request: ExecuteRoleTaskRequest,
    ) -> Result<ExecuteRoleTaskResponse, AgentTeamServiceError> {
        println!("[AGENT-1] execute_role_task CALLED, role_id: {}", request.role_id);
        tracing::info!("[AgentTeamService] execute_role_task 被调用，role_id: {}", request.role_id);

        // Load role with skills
        println!("[AGENT-2] Getting role_with_skills...");
        tracing::info!("[AgentTeamService-2] Getting role_with_skills...");
        let role_with_skills = self
            .team_service
            .get_role_with_skills(&request.role_id)
            .map_err(|e| AgentTeamServiceError::RoleNotFound(e.to_string()))?;
        println!("[AGENT-3] Got role_with_skills, role: {}", role_with_skills.role.name);
        tracing::info!("[AgentTeamService-3] Got role_with_skills");

        let role = role_with_skills.role.clone();
        let team_id = role.team_id.clone().unwrap_or_default();

        // Save user message immediately
        let user_msg = TeamMessage::user_message(team_id.clone(), request.task.clone());
        let _ = self.team_service.add_message(user_msg);

        // Build skill contexts from skills list
        let skill_contexts = Self::build_skill_contexts_from_skills(&role_with_skills.skills);

        // Build system prompt
        let mut prompt = role.system_prompt.clone();
        if !skill_contexts.is_empty() {
            prompt.push_str("\n\n## Available Skills\n");
            for ctx in &skill_contexts {
                prompt.push_str(&format!("- {}\n", ctx));
            }
        }

        // Execute AI call synchronously
        let auto_yes_prefix = "You are operating in auto-yes mode. If you ask any question requiring confirmation (yes/no, continue?, proceed?, etc.), always assume the answer is YES and proceed automatically. Never ask for confirmation.";

        // 构建最终 prompt（包含记忆上下文）
        let memory_context = request.context.get("memory_context").cloned().unwrap_or_default();
        tracing::info!("[AgentTeam] role_id: {}, task: {}, memory_context length: {}",
            request.role_id, request.task, memory_context.len());
        tracing::debug!("[AgentTeam] memory_context: {}", memory_context);

        // Build the full prompt
        let full_prompt = if memory_context.is_empty() {
            format!(
                "{}\n\n<system>\n{}\n</system>\n\n<user>\n{}\n</user>",
                auto_yes_prefix, prompt, request.task
            )
        } else {
            format!(
                "{}\n\n<system>\n{}\n</system>\n\n<user>\n{}\n</user>\n\n{}",
                auto_yes_prefix, prompt, request.task, memory_context
            )
        };

        tracing::info!("[AgentTeam] Full prompt length: {}, memory_context included: {}", full_prompt.len(), !memory_context.is_empty());

        // 获取当前工作区路径
        let working_dir = self.current_workspace_path.read().clone();

        println!("[AGENT-4] Working dir: {:?}", working_dir);
        tracing::info!("[AgentTeamService-4] Working dir: {:?}", working_dir);

        // 先检查 claude 命令是否存在
        let claude_check = tokio::process::Command::new("which")
            .arg("claude")
            .output()
            .await;
        match &claude_check {
            Ok(out) if out.status.success() => {
                let path = String::from_utf8_lossy(&out.stdout);
                println!("[AGENT-4b] claude found at: {}", path.trim());
                tracing::info!("[AgentTeamService-4b] claude found at: {}", path.trim());
            }
            _ => {
                println!("[AGENT-4b] WARNING: claude command not found in PATH!");
                tracing::warn!("[AgentTeamService-4b] claude command not found!");
            }
        }

        println!("[AGENT-5] Creating cmd...");
        tracing::info!("[AgentTeamService-5] Creating cmd...");

        let mut cmd = tokio::process::Command::new("claude");
        cmd.args(["-p", "--dangerously-skip-permissions", &full_prompt]);
        // 不继承 stdin，避免挂起
        cmd.stdin(std::process::Stdio::null());
        if let Some(ref dir) = working_dir {
            cmd.current_dir(dir);
            tracing::info!("[AgentTeam] 执行 Claude CLI，当前目录: {}", dir);
        }

        // 设置 120 秒超时
        println!("[AGENT-5b] About to call timeout(cmd.output())...");
        tracing::info!("[AgentTeamService-5b] About to call timeout...");
        let output = timeout(Duration::from_secs(120), cmd.output()).await;

        println!("[AGENT-6] timeout() returned");
        tracing::info!("[AgentTeamService-6] timeout() returned");

        // 处理超时和结果
        let output = match output {
            Ok(Ok(out)) => {
                println!("[DEBUG] cmd.output() succeeded");
                tracing::info!("[AgentTeam] cmd.output() succeeded");
                out
            }
            Ok(Err(e)) => {
                println!("[DEBUG] cmd.output() error: {:?}", e);
                tracing::error!("[AgentTeam] cmd.output() error: {:?}", e);
                return Err(AgentTeamServiceError::AiError(format!("cmd.output() error: {}", e)));
            }
            Err(_) => {
                println!("[DEBUG] TIMEOUT after 120 seconds!");
                tracing::error!("[AgentTeam] TIMEOUT after 120 seconds!");
                return Err(AgentTeamServiceError::AiError("Claude CLI execution timeout after 120 seconds".to_string()));
            }
        };

        println!("[DEBUG] Checking output status...");
        tracing::info!("[AgentTeam] Checking output status");

        let response = match output.status.success() {
            true => {
                println!("[DEBUG] Claude CLI success");
                tracing::info!("[AgentTeam] Claude CLI success");
                String::from_utf8_lossy(&output.stdout).trim().to_string()
            }
            false => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                println!("[DEBUG] Claude CLI failed, stderr: {}", stderr);
                tracing::error!("[AgentTeam] Claude CLI failed: {}", stderr);
                return Err(AgentTeamServiceError::AiError(stderr.to_string()));
            }
        };

        // Save assistant message
        let assistant_msg = TeamMessage::assistant_message(
            team_id.clone(),
            role.id.clone(),
            response.clone(),
        );
        let _ = self.team_service.add_message(assistant_msg);

        // Return the actual response
        Ok(ExecuteRoleTaskResponse {
            success: true,
            role_id: role.id.clone(),
            response,
            error: None,
        })
    }

    /// Build skill contexts for a role (static version for use in spawned tasks)
    async fn build_skill_contexts_static(_role: &TeamRole) -> Vec<String> {
        // Return empty - skill contexts will be part of role.system_prompt
        vec![]
    }

    /// Build skill contexts from skills list (for use in spawned tasks)
    fn build_skill_contexts_from_skills(skills: &[RoleSkill]) -> Vec<String> {
        let mut contexts = Vec::new();
        for skill in skills {
            let priority_str = skill.priority.as_str();
            let context = format!("[{}] skill: {}", priority_str.to_uppercase(), skill.skill_id);
            contexts.push(context);
        }
        contexts
    }

    /// Handle inbound Telegram message from a role's bot
    pub async fn handle_telegram_message(
        &self,
        message: InboundTelegramMessage,
    ) -> Result<String, AgentTeamServiceError> {
        // Get role's Telegram config
        let config = self
            .team_service
            .get_telegram_config(&message.role_id)
            .map_err(|e| AgentTeamServiceError::Telegram(e.to_string()))?;

        if !config.enabled || !config.conversation_enabled {
            return Err(AgentTeamServiceError::Telegram(
                "Conversation not enabled for this role".to_string(),
            ));
        }

        // Get role info
        let role_with_skills = self
            .team_service
            .get_role_with_skills(&message.role_id)
            .map_err(|e| AgentTeamServiceError::RoleNotFound(e.to_string()))?;

        let role = &role_with_skills.role;

        // Build skill context
        let skill_contexts = self
            .build_skill_contexts(&role_with_skills.skills)
            .await;

        // Build system prompt
        let system_prompt = self
            .build_system_prompt(role, &skill_contexts)
            .await;

        // Execute AI
        let response = self
            .execute_role_ai(role, &system_prompt, &message.text)
            .await
            .map_err(|e| AgentTeamServiceError::AiError(e.to_string()))?;

        // Save conversation
        let user_msg = TeamMessage::user_message(
            role.team_id.clone().unwrap_or_default(),
            message.text
        );
        let _ = self.team_service.add_message(user_msg);

        let assistant_msg = TeamMessage::assistant_message(
            role.team_id.clone().unwrap_or_default(),
            role.id.clone(),
            response.clone(),
        );
        let _ = self.team_service.add_message(assistant_msg);

        // Send response via Telegram
        if let Some(chat_id) = &config.chat_id {
            self.telegram_service
                .send_message(&config.bot_token, chat_id, &response)
                .await
                .map_err(|e| AgentTeamServiceError::Telegram(e.to_string()))?;
        }

        Ok(response)
    }

    /// Build execution context for a role from its assigned skills
    async fn build_role_context(
        &self,
        role: &TeamRole,
        skills: &[RoleSkill],
        _previous_context: &HashMap<String, String>,
    ) -> RoleExecutionContext {
        let skill_contexts = self.build_skill_contexts(skills).await;

        RoleExecutionContext {
            role: role.clone(),
            skills: skills.to_vec(),
            skill_contexts,
        }
    }

    /// Build skill context strings from assigned skills
    async fn build_skill_contexts(&self, skills: &[RoleSkill]) -> Vec<String> {
        let mut contexts = Vec::new();

        for skill in skills {
            // Get skill details from skill service
            if let Ok(skill_detail) = self.skill_service.get_skill(&skill.skill_id) {
                let priority_str = skill.priority.as_str();
                let context = format!(
                    "[{}] {}: {}",
                    priority_str.to_uppercase(),
                    skill_detail.name,
                    skill_detail.description
                );
                contexts.push(context);
            }
        }

        contexts
    }

    /// Build system prompt including skill context
    async fn build_system_prompt(
        &self,
        role: &TeamRole,
        skill_contexts: &[String],
    ) -> String {
        let mut prompt = role.system_prompt.clone();

        if !skill_contexts.is_empty() {
            prompt.push_str("\n\n## Available Skills\n");
            for ctx in skill_contexts {
                prompt.push_str(&format!("- {}\n", ctx));
            }
        }

        prompt
    }

    /// Execute AI call for a role using Claude CLI
    ///
    /// Claude CLI reads its local configuration (modified by Claude Switch)
    /// and automatically uses the currently configured model.
    async fn execute_role_ai(
        &self,
        _role: &TeamRole,
        system_prompt: &str,
        user_message: &str,
    ) -> Result<String, AgentTeamServiceError> {
        // Auto-yes prefix to skip confirmation prompts
        let auto_yes_prefix = "You are operating in auto-yes mode. If you ask any question requiring confirmation (yes/no, continue?, proceed?, etc.), always assume the answer is YES and proceed automatically. Never ask for confirmation.";

        // Build the full prompt with auto-yes prefix, system context and user message
        let full_prompt = format!(
            "{}\n\n<system>\n{}\n</system>\n\n<user>\n{}\n</user>",
            auto_yes_prefix, system_prompt, user_message
        );

        // Execute Claude CLI with the prompt
        // Claude CLI will automatically use the model configured locally
        // (which Claude Switch updates when switching models)
        let working_dir = self.current_workspace_path.read().clone();

        let mut cmd = tokio::process::Command::new("claude");
        cmd.args(["-p", "--dangerously-skip-permissions", &full_prompt]);
        if let Some(ref dir) = working_dir {
            cmd.current_dir(dir);
            tracing::info!("[AgentTeam] execute_role_ai 执行 Claude CLI，当前目录: {}", dir);
        }
        let output = cmd.output()
            .await
            .map_err(|e| AgentTeamServiceError::AiError(format!("Failed to execute Claude CLI: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AgentTeamServiceError::AiError(format!(
                "Claude CLI error: {}",
                stderr
            )));
        }

        let response = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(response.trim().to_string())
    }

    /// Broadcast completion notification to all enabled Telegram bots
    async fn broadcast_completion(
        &self,
        roles: &[crate::services::team_service::RoleWithSkills],
        messages: &[TeamMessage],
    ) {
        // Find the last assistant message as the result
        let result = messages
            .iter()
            .rev()
            .find(|m| m.message_type == crate::models::team::MessageType::Assistant)
            .map(|m| format!("Task completed:\n\n{}", m.content))
            .unwrap_or_else(|| "Task completed with no output".to_string());

        // Collect enabled telegram configs
        for role_with_skills in roles {
            if let Ok(config) = self.team_service.get_telegram_config(&role_with_skills.role.id) {
                if config.enabled && config.notifications_enabled {
                    if let Some(chat_id) = &config.chat_id {
                        let _ = self
                            .telegram_service
                            .send_message(&config.bot_token, chat_id, &result)
                            .await;
                    }
                }
            }
        }
    }

    /// Subscribe to inbound Telegram messages
    pub fn subscribe_telegram(&self) -> broadcast::Receiver<InboundTelegramMessage> {
        self.telegram_service.subscribe()
    }
}
