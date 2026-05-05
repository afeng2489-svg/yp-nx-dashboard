//! Group Chat Service
//!
//! Service layer for multi-agent group discussion orchestration.

use parking_lot::RwLock as ParkingRwLock;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

use nexus_ai::AIModelManager;

use crate::models::group_chat::{
    ConcludeDiscussionRequest, ConsensusStrategy, CreateGroupSessionRequest, DiscussionTurnInfo,
    GetMessagesRequest, GroupConclusion, GroupMessage, GroupParticipant, GroupSession,
    GroupSessionDetail, GroupStatus, SendMessageRequest, SpeakingStrategy, StartDiscussionRequest,
    ToolCall, UpdateGroupSessionRequest,
};
use crate::services::claude_cli;
use crate::services::group_chat_repository::{
    GroupChatRepository, GroupChatRepositoryError, SqliteGroupChatRepository,
};
use crate::services::team_service::TeamService;

/// 历史上下文优化配置
const HISTORY_THRESHOLD: usize = 20; // 超过此数量启用摘要
const RECENT_MESSAGE_COUNT: usize = 10; // 保留最近 N 条消息

/// 优化后的历史上下文
struct HistoryContext {
    /// 历史摘要（当消息超过阈值时生成）
    summary: Option<String>,
    /// 保留的最近消息
    recent_messages: Vec<GroupMessage>,
}

impl HistoryContext {
    /// 从消息列表构建优化后的上下文
    fn from_messages(messages: &[GroupMessage]) -> Self {
        if messages.len() <= HISTORY_THRESHOLD {
            // 消息不多，全部保留
            return Self {
                summary: None,
                recent_messages: messages.to_vec(),
            };
        }

        // 消息较多，生成摘要 + 保留最近消息
        let recent_start = messages.len().saturating_sub(RECENT_MESSAGE_COUNT);
        let recent_messages: Vec<_> = messages[recent_start..].to_vec();

        // 生成早期消息摘要
        let early_messages = &messages[..recent_start];
        let summary = Self::generate_summary(early_messages);

        Self {
            summary: Some(summary),
            recent_messages,
        }
    }

    /// 生成早期消息摘要
    fn generate_summary(early_messages: &[GroupMessage]) -> String {
        if early_messages.is_empty() {
            return String::new();
        }

        // 提取关键信息：发言者、主题、结论
        let mut topics: Vec<String> = Vec::new();
        let mut decisions: Vec<String> = Vec::new();

        for msg in early_messages {
            let content = &msg.content;
            let speaker = &msg.role_name;

            // 检测决策性语句
            if content.contains("决定") || content.contains("采用") || content.contains("共识")
            {
                decisions.push(format!("[{}]: {}", speaker, Self::truncate(content, 100)));
            }

            // 收集主题关键词（简化处理：取每条消息的前50字作为主题）
            if !topics.iter().any(|t: &String| t.contains(content)) {
                topics.push(format!("[{}]: {}", speaker, Self::truncate(content, 80)));
            }
        }

        let mut summary = String::from("【早期讨论摘要】\n");

        // 添加主题概览
        if !topics.is_empty() {
            summary.push_str("讨论内容：\n");
            for topic in topics.iter().take(5) {
                summary.push_str(&format!("  - {}\n", topic));
            }
        }

        // 添加决策
        if !decisions.is_empty() {
            summary.push_str("\n已达成共识：\n");
            for decision in decisions.iter().take(3) {
                summary.push_str(&format!("  - {}\n", decision));
            }
        }

        summary.push_str(&format!(
            "\n（共 {} 条早期消息已省略）",
            early_messages.len()
        ));
        summary
    }

    /// 截断字符串到指定长度
    fn truncate(s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            s.to_string()
        } else {
            format!("{}...", &s[..max_len.saturating_sub(3)])
        }
    }

    /// 渲染为 prompt 字符串
    fn render(&self) -> String {
        let mut output = String::new();

        if let Some(ref summary) = self.summary {
            output.push_str(summary);
            output.push_str("\n\n");
        }

        if self.recent_messages.is_empty() {
            output.push_str("（暂无历史记录）");
        } else {
            for msg in &self.recent_messages {
                output.push_str(&format!("[{}]: {}\n", msg.role_name, msg.content));
            }
        }

        output
    }
}

/// Group chat service error
#[derive(Debug, Error)]
pub enum GroupChatServiceError {
    #[error("Repository error: {0}")]
    Repository(#[from] GroupChatRepositoryError),
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    #[error("Session not active: {0}")]
    SessionNotActive(String),
    #[error("Role not found: {0}")]
    RoleNotFound(String),
    #[error("Team not found: {0}")]
    TeamNotFound(String),
    #[error("Claude CLI error: {0}")]
    ClaudeCli(String),
    #[error("Max turns reached")]
    MaxTurnsReached,
    #[error("Service error: {0}")]
    Service(String),
}

/// Shared group chat service
pub type SharedGroupChatService = Arc<GroupChatService>;

/// Group chat service
pub struct GroupChatService {
    repo: Arc<SqliteGroupChatRepository>,
    team_service: TeamService,
    ai_manager: Arc<AIModelManager>,
    /// 当前工作区路径，用于 Claude CLI --project 参数
    current_workspace_path: Arc<ParkingRwLock<Option<String>>>,
    // In-memory state for active discussions
    active_sessions: RwLock<HashMap<String, ActiveSessionState>>,
}

/// Active session state (in-memory)
struct ActiveSessionState {
    session_id: String,
    speaking_order: Vec<String>, // 发言顺序
    current_speaker_index: usize,
    last_turn: u32,
}

impl GroupChatService {
    pub fn new(
        repo: Arc<SqliteGroupChatRepository>,
        team_service: TeamService,
        ai_manager: Arc<AIModelManager>,
        current_workspace_path: Arc<ParkingRwLock<Option<String>>>,
    ) -> Self {
        Self {
            repo,
            team_service,
            ai_manager,
            current_workspace_path,
            active_sessions: RwLock::new(HashMap::new()),
        }
    }

    /// Create a new group session
    pub async fn create_session(
        &self,
        request: CreateGroupSessionRequest,
    ) -> Result<GroupSession, GroupChatServiceError> {
        let session = GroupSession::new(
            request.team_id.clone(),
            request.name,
            request.topic,
            request.speaking_strategy.unwrap_or_default(),
            request.consensus_strategy.unwrap_or_default(),
            request.moderator_role_id,
            request.max_turns.unwrap_or(10),
            request.turn_policy.unwrap_or_else(|| "all".to_string()),
        );

        self.repo.create_session(&session)?;
        Ok(session)
    }

    /// Get session by ID
    pub async fn get_session(&self, id: &str) -> Result<GroupSession, GroupChatServiceError> {
        self.repo
            .get_session(id)?
            .ok_or_else(|| GroupChatServiceError::SessionNotFound(id.to_string()))
    }

    /// Get sessions by team ID
    pub async fn get_sessions_by_team(
        &self,
        team_id: &str,
    ) -> Result<Vec<GroupSession>, GroupChatServiceError> {
        Ok(self.repo.get_sessions_by_team(team_id)?)
    }

    /// Get all sessions
    pub async fn get_all_sessions(&self) -> Result<Vec<GroupSession>, GroupChatServiceError> {
        Ok(self.repo.get_all_sessions()?)
    }

    /// Get session detail with participants and conclusion
    pub async fn get_session_detail(
        &self,
        id: &str,
    ) -> Result<GroupSessionDetail, GroupChatServiceError> {
        let session = self.get_session(id).await?;
        let participants = self.repo.get_participants(id)?;
        let message_count = self.repo.get_message_count(id)?;
        let conclusion = self.repo.get_conclusion(id)?;

        Ok(GroupSessionDetail {
            session,
            participants,
            message_count,
            conclusion,
        })
    }

    /// Update session
    pub async fn update_session(
        &self,
        id: &str,
        request: UpdateGroupSessionRequest,
    ) -> Result<GroupSession, GroupChatServiceError> {
        let mut session = self.get_session(id).await?;

        if let Some(name) = request.name {
            session.name = name;
        }
        if let Some(topic) = request.topic {
            session.topic = topic;
        }
        if let Some(strategy) = request.speaking_strategy {
            session.speaking_strategy = strategy;
        }
        if let Some(strategy) = request.consensus_strategy {
            session.consensus_strategy = strategy;
        }
        if let Some(moderator) = request.moderator_role_id {
            session.moderator_role_id = Some(moderator);
        }
        if let Some(max_turns) = request.max_turns {
            session.max_turns = max_turns;
        }

        self.repo.update_session(&session)?;
        Ok(session)
    }

    /// Start discussion
    pub async fn start_discussion(
        &self,
        session_id: &str,
        request: StartDiscussionRequest,
    ) -> Result<DiscussionTurnInfo, GroupChatServiceError> {
        let mut session = self.get_session(session_id).await?;

        if session.status != GroupStatus::Pending {
            return Err(GroupChatServiceError::SessionNotActive(
                session_id.to_string(),
            ));
        }

        // Add participants — fetch actual role names from team service
        let mut role_name_map: HashMap<String, String> = HashMap::new();
        for role_id in &request.participant_role_ids {
            let role_name = match self.team_service.get_role(role_id) {
                Ok(role) => role.name.clone(),
                Err(_) => role_id.clone(), // fallback to role_id if not found
            };
            role_name_map.insert(role_id.clone(), role_name.clone());
            let participant = GroupParticipant {
                role_id: role_id.clone(),
                role_name,
                joined_at: chrono::Utc::now(),
                last_spoke_at: None,
                message_count: 0,
            };
            self.repo.add_participant(session_id, &participant)?;
        }

        // Update session status
        session.status = GroupStatus::Active;
        self.repo.update_session(&session)?;

        // Initialize speaking order
        let speaking_order = match session.speaking_strategy {
            SpeakingStrategy::RoundRobin => request.participant_role_ids.clone(),
            SpeakingStrategy::Moderator => {
                // Moderator speaks first
                if let Some(ref mod_id) = session.moderator_role_id {
                    let mut order = vec![mod_id.clone()];
                    for id in &request.participant_role_ids {
                        if id != mod_id {
                            order.push(id.clone());
                        }
                    }
                    order
                } else {
                    request.participant_role_ids.clone()
                }
            }
            SpeakingStrategy::Debate => {
                // Two sides alternate
                let mid = request.participant_role_ids.len() / 2;
                let mut order = Vec::new();
                for i in 0..mid {
                    if i < request.participant_role_ids.len() - mid {
                        order.push(request.participant_role_ids[mid + i].clone());
                    }
                    order.push(request.participant_role_ids[i].clone());
                }
                order
            }
            SpeakingStrategy::Free => request.participant_role_ids.clone(),
        };

        let state = ActiveSessionState {
            session_id: session_id.to_string(),
            speaking_order: speaking_order.clone(),
            current_speaker_index: 0,
            last_turn: 0,
        };

        {
            let mut active = self.active_sessions.write().await;
            active.insert(session_id.to_string(), state);
        }

        let next_speaker = speaking_order.first().cloned();

        Ok(DiscussionTurnInfo {
            current_turn: 0,
            max_turns: session.max_turns,
            next_speaker_role_id: next_speaker.clone(),
            next_speaker_role_name: next_speaker,
            speaking_order,
        })
    }

    /// Send a message in the discussion
    pub async fn send_message(
        &self,
        session_id: &str,
        request: SendMessageRequest,
    ) -> Result<GroupMessage, GroupChatServiceError> {
        let session = self.get_session(session_id).await?;

        if session.status != GroupStatus::Active {
            return Err(GroupChatServiceError::SessionNotActive(
                session_id.to_string(),
            ));
        }

        // Build prompt for Claude CLI with optimized history
        let history_ctx = self.get_optimized_history(session_id).await?;
        let prompt =
            self.build_discussion_prompt_with_context(&session, &history_ctx, &request.content);

        // 获取当前工作区路径
        let working_dir = self.current_workspace_path.read().clone();
        let working_dir_ref = working_dir.as_deref();
        tracing::info!(
            "[GroupChat] 调用 Claude CLI，当前工作区路径: {:?}",
            working_dir_ref
        );

        // Call Claude CLI
        let response = claude_cli::call_claude_cli(&prompt, working_dir_ref)
            .await
            .map_err(|e| GroupChatServiceError::ClaudeCli(e))?;

        // Parse response and create message
        let role_name = match self.team_service.get_role(&request.role_id) {
            Ok(role) => role.name.clone(),
            Err(_) => request.role_id.clone(),
        };
        let turn_number = session.current_turn;
        let message = GroupMessage::new(
            session_id.to_string(),
            request.role_id.clone(),
            role_name,
            response,
            vec![], // TODO: parse tool calls if any
            request.reply_to,
            turn_number,
        );

        self.repo.create_message(&message)?;

        // Update participant stats
        self.update_participant_stats(session_id, &request.role_id)
            .await?;

        Ok(message)
    }

    /// Execute a role's turn using Claude CLI
    pub async fn execute_role_turn(
        &self,
        session_id: &str,
        role_id: &str,
    ) -> Result<GroupMessage, GroupChatServiceError> {
        let session = self.get_session(session_id).await?;

        if session.status != GroupStatus::Active {
            return Err(GroupChatServiceError::SessionNotActive(
                session_id.to_string(),
            ));
        }

        // Get optimized conversation history (with summarization if needed)
        let history_ctx = self.get_optimized_history(session_id).await?;

        // Build prompt using optimized history
        let prompt = self.build_role_prompt_with_context(&session, role_id, &history_ctx);

        // 获取当前工作区路径
        let working_dir = self.current_workspace_path.read().clone();
        let working_dir_ref = working_dir.as_deref();
        tracing::info!(
            "[GroupChat] 执行角色 turn，当前工作区路径: {:?}",
            working_dir_ref
        );

        // Execute with Claude CLI
        let response = claude_cli::call_claude_cli(&prompt, working_dir_ref)
            .await
            .map_err(|e| GroupChatServiceError::ClaudeCli(e))?;

        // Resolve role name
        let role_name = match self.team_service.get_role(role_id) {
            Ok(role) => role.name.clone(),
            Err(_) => role_id.to_string(),
        };

        // Create message
        let turn_number = session.current_turn;
        let message = GroupMessage::new(
            session_id.to_string(),
            role_id.to_string(),
            role_name,
            response,
            vec![],
            None,
            turn_number,
        );

        self.repo.create_message(&message)?;

        // Update participant
        self.update_participant_stats(session_id, role_id).await?;

        // Update session turn
        let mut updated_session = session.clone();
        updated_session.current_turn += 1;
        self.repo.update_session(&updated_session)?;

        Ok(message)
    }

    /// Get next speaker based on strategy
    pub async fn get_next_speaker(
        &self,
        session_id: &str,
    ) -> Result<Option<(String, String)>, GroupChatServiceError> {
        let session = self.get_session(session_id).await?;

        if session.status != GroupStatus::Active {
            return Ok(None);
        }

        let active = self.active_sessions.read().await;
        let state = match active.get(session_id) {
            Some(s) => s,
            None => return Ok(None),
        };

        if session.speaking_strategy == SpeakingStrategy::Free {
            // In free mode, anyone can speak
            return Ok(None);
        }

        if state.current_speaker_index >= state.speaking_order.len() {
            return Ok(None);
        }

        let role_id = state.speaking_order[state.current_speaker_index].clone();
        let role_name = match self.team_service.get_role(&role_id) {
            Ok(role) => role.name.clone(),
            Err(_) => role_id.clone(),
        };
        Ok(Some((role_id, role_name)))
    }

    /// Advance to next speaker
    pub async fn advance_speaker(&self, session_id: &str) -> Result<(), GroupChatServiceError> {
        let mut active = self.active_sessions.write().await;
        if let Some(state) = active.get_mut(session_id) {
            state.current_speaker_index += 1;
        }
        Ok(())
    }

    /// Conclude discussion and generate conclusion
    pub async fn conclude_discussion(
        &self,
        session_id: &str,
        request: ConcludeDiscussionRequest,
    ) -> Result<GroupConclusion, GroupChatServiceError> {
        let mut session = self.get_session(session_id).await?;

        if request.force != Some(true) && session.status != GroupStatus::Active {
            return Err(GroupChatServiceError::SessionNotActive(
                session_id.to_string(),
            ));
        }

        // Get all messages
        let messages = self.repo.get_messages_by_session(session_id, None, None)?;

        // Generate conclusion using Claude CLI
        let conclusion_prompt = self.build_conclusion_prompt(&session, &messages);
        let working_dir = self.current_workspace_path.read().clone();
        let working_dir_ref = working_dir.as_deref();
        let conclusion_content = claude_cli::call_claude_cli(&conclusion_prompt, working_dir_ref)
            .await
            .unwrap_or_else(|_| "讨论已结束，未能生成结论。".to_string());

        // Calculate consensus (simplified: based on agreement)
        let participant_ids: Vec<String> = messages
            .iter()
            .map(|m| m.role_id.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        let conclusion = GroupConclusion::new(
            session_id.to_string(),
            conclusion_content,
            0.8, // TODO: calculate actual consensus
            HashMap::new(),
            participant_ids,
        );

        self.repo.save_conclusion(&conclusion)?;

        // Update session status
        session.status = GroupStatus::Concluded;
        self.repo.update_session(&session)?;

        // Remove from active sessions
        {
            let mut active = self.active_sessions.write().await;
            active.remove(session_id);
        }

        Ok(conclusion)
    }

    /// Get messages for a session
    pub async fn get_messages(
        &self,
        session_id: &str,
        request: GetMessagesRequest,
    ) -> Result<Vec<GroupMessage>, GroupChatServiceError> {
        Ok(self.repo.get_messages_by_session(
            session_id,
            request.limit,
            request.before.as_deref(),
        )?)
    }

    /// Delete a session
    pub async fn delete_session(&self, id: &str) -> Result<(), GroupChatServiceError> {
        // Remove from active if present
        {
            let mut active = self.active_sessions.write().await;
            active.remove(id);
        }

        self.repo.delete_session(id)?;
        Ok(())
    }

    // ============== Helper Methods ==============

    /// Get conversation history
    async fn get_conversation_history(
        &self,
        session_id: &str,
        limit: u32,
    ) -> Result<Vec<GroupMessage>, GroupChatServiceError> {
        Ok(self
            .repo
            .get_messages_by_session(session_id, Some(limit), None)?)
    }

    /// Get optimized conversation history with summarization
    async fn get_optimized_history(
        &self,
        session_id: &str,
    ) -> Result<HistoryContext, GroupChatServiceError> {
        // 获取足够多的历史消息用于摘要
        let all_messages = self.repo.get_messages_by_session(session_id, None, None)?;
        Ok(HistoryContext::from_messages(&all_messages))
    }

    /// Update participant stats
    async fn update_participant_stats(
        &self,
        session_id: &str,
        role_id: &str,
    ) -> Result<(), GroupChatServiceError> {
        let participants = self.repo.get_participants(session_id)?;
        if let Some(mut p) = participants.into_iter().find(|p| p.role_id == role_id) {
            p.message_count += 1;
            p.last_spoke_at = Some(chrono::Utc::now());
            self.repo.update_participant(session_id, &p)?;
        }
        Ok(())
    }

    /// Build discussion prompt using optimized history context
    fn build_discussion_prompt_with_context(
        &self,
        session: &GroupSession,
        history_ctx: &HistoryContext,
        new_input: &str,
    ) -> String {
        let history = history_ctx.render();

        let mut prompt = format!(
            r#"<system>
你是一个有记忆的 AI 助手，正在参与团队讨论。
当前讨论主题：{}
讨论目标：{}

【对话历史】
{}
</system>"#,
            session.topic, session.name, history
        );

        prompt.push_str(&format!(
            "\n\n【最新用户输入】：{}\n\n请基于以上对话历史，回复用户。保持对话的连贯性和记忆。",
            new_input
        ));
        prompt
    }

    /// Build discussion prompt (legacy, for backwards compatibility)
    fn build_discussion_prompt(
        &self,
        session: &GroupSession,
        messages: &[GroupMessage],
        new_input: &str,
    ) -> String {
        let history_ctx = HistoryContext::from_messages(messages);
        self.build_discussion_prompt_with_context(session, &history_ctx, new_input)
    }

    /// Build role-specific prompt using optimized history context
    fn build_role_prompt_with_context(
        &self,
        session: &GroupSession,
        role_id: &str,
        history_ctx: &HistoryContext,
    ) -> String {
        let history = history_ctx.render();

        let role_context = match self.team_service.get_role(role_id) {
            Ok(role) if !role.system_prompt.is_empty() => format!(
                "你的角色：{}\n你的职责：{}\n\n{}",
                role.name, role.description, role.system_prompt
            ),
            Ok(role) => format!("你的角色：{}\n你的职责：{}", role.name, role.description),
            Err(_) => format!("你的角色 ID：{}", role_id),
        };

        format!(
            r#"<system>
{}

当前讨论主题：{}
讨论目标：{}

【对话历史】
{}
</system>

<user>
请继续基于以上对话历史，以你的角色身份回复。保持对话的连贯性，记住之前讨论过的内容，继续推进讨论。
</user>"#,
            role_context, session.topic, session.name, history
        )
    }

    /// Build role-specific prompt (legacy, for backwards compatibility)
    fn build_role_prompt(
        &self,
        session: &GroupSession,
        role_id: &str,
        messages: &[GroupMessage],
    ) -> String {
        let history_ctx = HistoryContext::from_messages(messages);
        self.build_role_prompt_with_context(session, role_id, &history_ctx)
    }

    /// Build conclusion prompt
    fn build_conclusion_prompt(&self, session: &GroupSession, messages: &[GroupMessage]) -> String {
        let all_content = messages
            .iter()
            .map(|m| format!("{}: {}", m.role_name, m.content))
            .collect::<Vec<_>>()
            .join("\n\n");

        format!(
            r#"<system>
讨论主题：{}
目标：{}

所有发言记录：
{}

请总结本次讨论的主要观点和建议，并给出一个最终的结论或建议方案。
结论应该简洁明了，便于决策。
</system>"#,
            session.topic, session.name, all_content
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_build_prompt() {
        // Test prompt building logic
        let session = GroupSession::new(
            "team-1".to_string(),
            "架构讨论".to_string(),
            "微服务 vs 单体".to_string(),
            SpeakingStrategy::Free,
            ConsensusStrategy::Majority,
            None,
            10,
            "all".to_string(),
        );

        // Just verify the session was created correctly
        assert_eq!(session.name, "架构讨论");
        assert_eq!(session.topic, "微服务 vs 单体");
    }

    // ── HistoryContext ────────────────────────────────────────────────────────

    fn make_messages(count: usize) -> Vec<GroupMessage> {
        (0..count)
            .map(|i| {
                GroupMessage::new(
                    "session-1".to_string(),
                    "role-1".to_string(),
                    "Speaker".to_string(),
                    format!("Message {}", i),
                    vec![],
                    None,
                    i as u32,
                )
            })
            .collect()
    }

    #[test]
    fn test_history_context_under_threshold_keeps_all() {
        let messages = make_messages(HISTORY_THRESHOLD);
        let ctx = HistoryContext::from_messages(&messages);

        assert!(ctx.summary.is_none(), "no summary when under threshold");
        assert_eq!(ctx.recent_messages.len(), HISTORY_THRESHOLD);
    }

    #[test]
    fn test_history_context_over_threshold_generates_summary() {
        let messages = make_messages(HISTORY_THRESHOLD + 5);
        let ctx = HistoryContext::from_messages(&messages);

        assert!(
            ctx.summary.is_some(),
            "summary should be present over threshold"
        );
        assert_eq!(ctx.recent_messages.len(), RECENT_MESSAGE_COUNT);
    }

    #[test]
    fn test_history_context_empty_messages() {
        let ctx = HistoryContext::from_messages(&[]);
        assert!(ctx.summary.is_none());
        assert!(ctx.recent_messages.is_empty());
    }

    #[test]
    fn test_history_context_recent_messages_are_the_last_ones() {
        let total = HISTORY_THRESHOLD + 10;
        let messages = make_messages(total);
        let ctx = HistoryContext::from_messages(&messages);

        // Recent messages should be the last RECENT_MESSAGE_COUNT items
        let expected_start = total - RECENT_MESSAGE_COUNT;
        for (i, msg) in ctx.recent_messages.iter().enumerate() {
            assert_eq!(
                msg.content,
                format!("Message {}", expected_start + i),
                "recent[{}] should be message {}",
                i,
                expected_start + i
            );
        }
    }
}
