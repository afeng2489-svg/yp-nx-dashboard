//! Team Session Actor — 团队会话主循环控制器
//!
//! 编排 Architect→Developer→Reviewer→Tester 串行链，
//! 通过 MessageBus 广播 Agent 事件，集成质量门禁。

use crate::cli::{CliManager, CliProvider, CliRequest};
use crate::error::OrchestratorError;
use crate::message_bus::{Channel, MessageBus, MessagePayload, MessageSource};
use crate::team::{AgentId, AgentRole, AgentStatus, Team, TeamManager};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use uuid::Uuid;

/// 单个 Agent 执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    pub agent_id: AgentId,
    pub role: AgentRole,
    pub agent_name: String,
    pub text: String,
    pub duration_ms: u64,
    pub attempts: u32,
}

/// 链执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainResult {
    pub execution_id: Uuid,
    pub task: String,
    pub agent_results: Vec<AgentResult>,
    pub total_duration_ms: u64,
}

/// 最大重试次数（超过后升级模型）
const MAX_RETRIES: u32 = 3;

/// 用户消息 — 从 TUI/CLI 发送到会话主循环
///
/// `target_agent` 为 `None` 时表示消息将注入当前正在执行的 Agent 上下文。
/// `target_agent` 为 `Some(name)` 时表示消息定向到指定 Agent。
#[derive(Debug, Clone)]
pub struct UserMessage {
    /// 目标 Agent 名称（`@agent_name`），None = 当前 Agent
    pub target_agent: Option<String>,
    /// 消息内容
    pub content: String,
}

impl UserMessage {
    /// 解析用户输入，提取 @agent_name 前缀
    ///
    /// 格式支持：
    /// - `@architect 请用 Rust 实现` → target: Some("architect"), content: "请用 Rust 实现"
    /// - `请用 Rust 实现` → target: None, content: "请用 Rust 实现"
    pub fn parse(input: &str) -> Self {
        let trimmed = input.trim();
        if let Some(rest) = trimmed.strip_prefix('@') {
            if let Some((name, content)) = rest.split_once(char::is_whitespace) {
                return Self {
                    target_agent: Some(name.to_string()),
                    content: content.trim().to_string(),
                };
            }
            // 只有 @agent_name 没有内容
            return Self {
                target_agent: Some(trimmed[1..].to_string()),
                content: String::new(),
            };
        }
        Self {
            target_agent: None,
            content: trimmed.to_string(),
        }
    }

    /// 检查消息是否定向到指定的 Agent
    pub fn is_targeting(&self, agent_name: &str) -> bool {
        self.target_agent
            .as_ref()
            .is_none_or(|t| t.eq_ignore_ascii_case(agent_name))
    }
}

/// 用户消息队列 — 包装 UnboundedReceiver 并维护未消费消息的缓冲区
///
/// 确保定向到其他 Agent 的消息不会被消费掉，
/// 而是保存在缓冲区中，等目标 Agent 执行时再注入。
#[derive(Debug)]
pub struct UserMessageQueue {
    receiver: mpsc::UnboundedReceiver<UserMessage>,
    buffer: Vec<UserMessage>,
}

impl UserMessageQueue {
    pub fn new(receiver: mpsc::UnboundedReceiver<UserMessage>) -> Self {
        Self {
            receiver,
            buffer: Vec::new(),
        }
    }

    /// 将所有待处理消息（包括缓冲区中的和通道中的）中
    /// 定向到 `agent_name` 的消息取出并返回。
    ///
    /// 非定向消息保留在缓冲区中，供后续 Agent 使用。
    pub fn drain_for(&mut self, agent_name: &str) -> Vec<UserMessage> {
        // 将通道中所有待处理消息拉入缓冲区
        while let Ok(msg) = self.receiver.try_recv() {
            self.buffer.push(msg);
        }

        let mut result = Vec::new();
        let mut remaining = Vec::new();
        for msg in self.buffer.drain(..) {
            if msg.is_targeting(agent_name) {
                result.push(msg);
            } else {
                remaining.push(msg);
            }
        }
        self.buffer = remaining;
        result
    }
}

/// 链执行进度事件 — 通过 mpsc 发送给 TUI/CLI 层
#[derive(Debug, Clone)]
pub enum ChainEvent {
    AgentStarted {
        agent_id: AgentId,
        role: AgentRole,
        name: String,
    },
    AgentCompleted {
        agent_id: AgentId,
        role: AgentRole,
        name: String,
        summary: String,
    },
    ChainCompleted {
        total_duration_ms: u64,
    },
    ChainFailed {
        error: String,
    },
    /// Agent 正在等待用户输入
    AgentWaitingForInput {
        agent_id: AgentId,
        role: AgentRole,
        name: String,
        reason: String,
    },
    /// Agent 收到了用户输入
    AgentReceivedInput {
        agent_id: AgentId,
        message: String,
    },
}

/// TeamSessionActor — 团队会话主循环
///
/// 持有 CliManager 和 MessageBus 的引用，驱动串行链循环。
/// CLI 只负责启动和渲染，业务逻辑集中于此。
pub struct TeamSessionActor {
    #[allow(dead_code)] // reserved for AF-01+ team↔workflow CLI dispatch
    cli_manager: Arc<CliManager>,
    message_bus: Arc<MessageBus>,
}

impl TeamSessionActor {
    pub fn new(cli_manager: Arc<CliManager>, message_bus: Arc<MessageBus>) -> Self {
        Self {
            cli_manager,
            message_bus,
        }
    }

    /// 运行团队串行链
    ///
    /// 按 hierarchy 顺序遍历 Agent，每个 Agent 的输出作为下一个的上下文。
    /// 如果提供了 `events` 发送器，会在每个阶段推送 ChainEvent。
    /// 如果提供了 `user_messages` 接收器，会在每个 Agent 执行前处理用户消息。
    pub async fn run_chain(
        &self,
        team: &Team,
        task: &str,
        events: Option<mpsc::UnboundedSender<ChainEvent>>,
        user_messages: Option<mpsc::UnboundedReceiver<UserMessage>>,
    ) -> Result<ChainResult, OrchestratorError> {
        let execution_id = Uuid::new_v4();
        let start = Instant::now();

        // 输出 session_id 供 Tauri 等外部调用方解析
        println!("session_id:{}", execution_id);

        // 将 user_messages 包装为 UserMessageQueue 再包 Mutex 以支持多次借用
        let user_messages =
            user_messages.map(|rx| std::sync::Mutex::new(UserMessageQueue::new(rx)));

        tracing::info!(
            execution_id = %execution_id,
            team = %team.name,
            task = %task,
            "启动团队会话"
        );

        // 获取执行顺序：从 hierarchy 中拓扑排序
        let execution_order = Self::resolve_execution_order(team);

        let mut agent_results: Vec<AgentResult> = Vec::new();
        let mut accumulated_context = String::new();

        for &agent_id in &execution_order {
            let member = team
                .members
                .get(&agent_id)
                .ok_or_else(|| OrchestratorError::NotFound(format!("agent {:?}", agent_id)))?;

            // 发布 AgentStarted 事件
            let _ = self.message_bus.publish(
                Channel::AgentEvents,
                MessagePayload::AgentStarted { agent_id },
            );
            if let Some(ref tx) = events {
                let _ = tx.send(ChainEvent::AgentStarted {
                    agent_id,
                    role: member.role,
                    name: member.name.clone(),
                });
            }

            // 构建基础上下文（包含前面 Agent 的输出）
            let mut context = Self::build_context(task, &accumulated_context, member.role);

            // 注入等待的用户消息，如果有
            let injected = Self::inject_user_messages(
                &user_messages,
                &member.name,
                &mut context,
                &events,
                agent_id,
                member.role,
            );

            // 如果有定向消息且内容非空，通知 TUI 层
            if injected > 0 {
                tracing::info!(
                    agent = %member.name,
                    injected = injected,
                    "注入用户消息到 Agent 上下文"
                );
            }

            // 执行 Agent（含质量门禁）
            let result = self.execute_with_gate(member, task, &context).await?;

            // 发布 AgentCompleted 事件
            let _ = self.message_bus.publish(
                Channel::AgentEvents,
                MessagePayload::AgentCompleted {
                    agent_id,
                    outputs: vec![result.text.clone()],
                },
            );
            if let Some(ref tx) = events {
                let _ = tx.send(ChainEvent::AgentCompleted {
                    agent_id,
                    role: member.role,
                    name: member.name.clone(),
                    summary: result.text.chars().take(200).collect(),
                });
            }

            // 累积上下文
            if !result.text.is_empty() {
                accumulated_context
                    .push_str(&format!("\n\n## {} 输出\n{}\n", member.name, result.text));
            }

            agent_results.push(result);
        }

        let total_duration_ms = start.elapsed().as_millis() as u64;

        tracing::info!(
            execution_id = %execution_id,
            agents = agent_results.len(),
            duration_ms = total_duration_ms,
            "团队会话完成"
        );

        if let Some(ref tx) = events {
            let _ = tx.send(ChainEvent::ChainCompleted { total_duration_ms });
        }

        Ok(ChainResult {
            execution_id,
            task: task.to_string(),
            agent_results,
            total_duration_ms,
        })
    }

    /// 从 hierarchy 解析执行顺序（拓扑排序）
    fn resolve_execution_order(team: &Team) -> Vec<AgentId> {
        if team.hierarchy.is_empty() {
            // 无 hierarchy 时按成员列表顺序
            return team.members.keys().copied().collect();
        }

        let mut order = Vec::new();
        let mut visited = HashMap::new();

        // 找到所有顶层（leader）节点
        let mut roots: Vec<AgentId> = team
            .hierarchy
            .keys()
            .filter(|id| {
                // leader 是指不在任何人依赖列表中的节点
                !team.hierarchy.values().any(|subs| subs.contains(id))
            })
            .copied()
            .collect();

        roots.sort_by_key(|id| id.0);

        for root in &roots {
            Self::topo_dfs(*root, team, &mut visited, &mut order);
        }
        // 后序遍历得到的是逆拓扑序，反转得到正确顺序
        order.reverse();

        // 把不在 hierarchy 中的成员追加到末尾
        for member_id in team.members.keys() {
            if !order.contains(member_id) {
                order.push(*member_id);
            }
        }

        order
    }

    fn topo_dfs(
        node: AgentId,
        team: &Team,
        visited: &mut HashMap<AgentId, bool>,
        order: &mut Vec<AgentId>,
    ) {
        if visited.contains_key(&node) {
            return;
        }
        visited.insert(node, true);
        if let Some(children) = team.hierarchy.get(&node) {
            for child in children {
                Self::topo_dfs(*child, team, visited, order);
            }
        }
        order.push(node);
    }

    /// 从 user_messages 队列中消费所有定向到当前 Agent 的消息，
    /// 将其注入上下文。非定向消息保留在缓冲区中供后续 Agent 使用。
    ///
    /// 返回注入的消息数量。
    fn inject_user_messages(
        user_queue: &Option<std::sync::Mutex<UserMessageQueue>>,
        agent_name: &str,
        context: &mut String,
        events: &Option<mpsc::UnboundedSender<ChainEvent>>,
        agent_id: AgentId,
        role: AgentRole,
    ) -> usize {
        let mut guard = match user_queue {
            Some(ref g) => g.lock().expect("user_message_queue lock poisoned"),
            None => return 0,
        };

        let messages = guard.drain_for(agent_name);
        let count = messages.len();

        if count > 0 {
            let user_section = format!(
                "\n\n## 用户反馈/指令\n{}\n",
                messages
                    .iter()
                    .map(|m| m.content.as_str())
                    .collect::<Vec<_>>()
                    .join("\n---\n")
            );
            context.push_str(&user_section);

            if let Some(ref tx) = events {
                for msg in &messages {
                    let _ = tx.send(ChainEvent::AgentReceivedInput {
                        agent_id,
                        message: msg.content.clone(),
                    });
                }
            }
        }

        count
    }

    /// 为 Agent 构建上下文提示词
    fn build_context(task: &str, accumulated: &str, role: AgentRole) -> String {
        let role_instruction = match role {
            AgentRole::Architect => {
                "你是系统架构师。分析任务需求，输出清晰的架构设计方案。\n\
                 包括：模块划分、数据流、接口定义、技术选型理由。\n\
                 不要写代码，只输出设计文档。"
            }
            AgentRole::Developer => {
                "你是开发者。根据架构师的设计方案，编写可工作的代码实现。\n\
                 输出完整的代码文件，包含必要的注释和错误处理。"
            }
            AgentRole::Reviewer => {
                "你是代码审查者。审查前面 Agent 的输出，指出问题并给出改进建议。\n\
                 检查：正确性、性能、安全性、可维护性。\n\
                 如果代码没有问题，明确表示通过审查。"
            }
            AgentRole::Tester => {
                "你是测试工程师。为前面的代码编写全面的测试。\n\
                 包括：单元测试、边界条件测试、异常路径测试。\n\
                 输出可运行的测试代码。"
            }
            _ => "根据任务要求和前面的上下文，完成你的工作。",
        };

        if accumulated.is_empty() {
            format!("## 任务\n{}\n\n## 角色指令\n{}", task, role_instruction)
        } else {
            format!(
                "## 任务\n{}\n\n## 角色指令\n{}\n\n## 前面的工作成果\n{}",
                task, role_instruction, accumulated
            )
        }
    }

    /// 执行单个 Agent（带质量门禁）
    async fn execute_with_gate(
        &self,
        member: &crate::team::TeamMember,
        task: &str,
        context: &str,
    ) -> Result<AgentResult, OrchestratorError> {
        let start = Instant::now();
        let system_prompt = member.role.default_prompt();
        let prompt = format!(
            "{}\n\n<system>\n{}\n</system>\n\n<user>\n{}\n</user>",
            Self::auto_yes_prefix(),
            system_prompt,
            context
        );

        let mut model = Self::role_to_model(member.role);
        let mut attempts: u32 = 0;
        let mut last_text;

        loop {
            attempts += 1;

            tracing::info!(
                agent = %member.name,
                role = ?member.role,
                model = %model,
                attempt = attempts,
                "执行 Agent"
            );

            last_text = self.invoke_claude(&prompt, model, member.provider).await?;

            // 质量检查通过，立即返回
            if TeamSessionActor::quick_quality_check(&last_text, member.role) {
                break;
            }

            // 质量未通过 — 优先升级模型再重试，避免同模型无意义重试
            if attempts < MAX_RETRIES {
                let upgraded = Self::upgrade_model(member.role, model);
                if upgraded != model {
                    tracing::info!(
                        agent = %member.name,
                        old_model = %model,
                        new_model = %upgraded,
                        attempt = attempts,
                        "质量检查未通过，升级模型重试"
                    );
                    model = upgraded;
                    continue;
                }
                // 已是最强模型，无需再重试
                tracing::warn!(
                    agent = %member.name,
                    model = %model,
                    attempt = attempts,
                    "质量检查未通过但已是最强模型，接受当前结果"
                );
                break;
            }

            // 已达最大重试次数
            tracing::warn!(
                agent = %member.name,
                attempts = attempts,
                "质量检查未通过但已达最大重试次数，接受当前结果"
            );
            break;
        }

        let duration_ms = start.elapsed().as_millis() as u64;
        Ok(AgentResult {
            agent_id: member.id,
            role: member.role,
            agent_name: member.name.clone(),
            text: last_text,
            duration_ms,
            attempts,
        })
    }

    /// 调用 Claude CLI
    async fn invoke_claude(
        &self,
        prompt: &str,
        model: &str,
        _provider: CliProvider,
    ) -> Result<String, OrchestratorError> {
        let model = model.to_string();
        let prompt = prompt.to_string();
        // 使用 spawn_blocking + std::process::Command 避免 tokio process spawn 在
        // macOS 上的 ENXIO (os error 6) 问题
        tokio::task::spawn_blocking(move || {
            let claude_bin =
                std::env::var("CLAUDE_BIN").unwrap_or_else(|_| "/opt/homebrew/bin/claude".into());
            let output = std::process::Command::new(&claude_bin)
                .args(["--print", "--model", &model, "--", &prompt])
                .output()
                .map_err(|e| OrchestratorError::Execution(format!("无法启动 claude: {}", e)))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(OrchestratorError::Execution(format!(
                    "claude 执行失败: {}",
                    stderr
                )));
            }

            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        })
        .await
        .map_err(|e| OrchestratorError::Execution(format!("spawn_blocking 失败: {}", e)))?
    }

    /// 快速质量检查 — 检查输出是否非空且基本有效
    fn quick_quality_check(text: &str, role: AgentRole) -> bool {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return false;
        }

        if trimmed.len() < 20 {
            return false;
        }

        // 超过 100 字符的输出视为有效，不做模式匹配
        if trimmed.len() > 100 {
            return true;
        }

        // 中等长度输出做角色特征检查
        match role {
            AgentRole::Architect => {
                trimmed.contains("模块")
                    || trimmed.contains("架构")
                    || trimmed.contains("设计")
                    || trimmed.contains("##")
                    || trimmed.contains("```")
                    || trimmed.contains("module")
                    || trimmed.contains("architecture")
                    || trimmed.contains("design")
            }
            AgentRole::Developer => {
                trimmed.contains("```")
                    || trimmed.contains("fn ")
                    || trimmed.contains("fn(")
                    || trimmed.contains("impl")
                    || trimmed.contains("struct ")
                    || trimmed.contains("enum ")
                    || trimmed.contains("let ")
                    || trimmed.contains("pub ")
                    || trimmed.contains("mod ")
                    || trimmed.contains("use ")
                    || trimmed.contains("fn main")
                    || trimmed.contains("print")
                    || trimmed.contains("def ")
                    || trimmed.contains("function ")
                    || trimmed.contains("class ")
            }
            AgentRole::Reviewer => true,
            AgentRole::Tester => {
                trimmed.contains("```")
                    || trimmed.contains("#[test]")
                    || trimmed.contains("fn test_")
                    || trimmed.contains("#[cfg(test)]")
                    || trimmed.contains("assert")
                    || trimmed.contains("mod tests")
                    || trimmed.contains("@Test")
                    || trimmed.contains("describe(")
                    || trimmed.contains("it(")
            }
            _ => true,
        }
    }

    /// 角色→默认模型映射
    pub fn role_to_model(role: AgentRole) -> &'static str {
        match role {
            AgentRole::Architect => "claude-opus-4-5",
            AgentRole::Leader => "claude-opus-4-5",
            AgentRole::Developer => "claude-sonnet-4-5",
            AgentRole::Reviewer => "claude-haiku-4-5",
            AgentRole::Tester => "claude-haiku-4-5",
            AgentRole::Researcher => "claude-sonnet-4-5",
            AgentRole::Executor => "claude-haiku-4-5",
        }
    }

    /// 模型升级策略
    fn upgrade_model(_role: AgentRole, current: &str) -> &'static str {
        match current {
            "claude-haiku-4-5" => "claude-sonnet-4-5",
            "claude-sonnet-4-5" => "claude-opus-4-5",
            _ => "claude-opus-4-5", // 兜底：使用最强模型
        }
    }

    fn auto_yes_prefix() -> &'static str {
        "You are operating in auto-yes mode. \
         If you ask any question requiring confirmation, \
         always assume the answer is YES and proceed automatically. \
         Never ask for confirmation."
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::team::{AgentId, Team, TeamMember};

    // ── role_to_model ──────────────────────────────────────────────

    #[test]
    fn role_to_model_maps_correctly() {
        assert_eq!(
            TeamSessionActor::role_to_model(AgentRole::Architect),
            "claude-opus-4-5"
        );
        assert_eq!(
            TeamSessionActor::role_to_model(AgentRole::Developer),
            "claude-sonnet-4-5"
        );
        assert_eq!(
            TeamSessionActor::role_to_model(AgentRole::Reviewer),
            "claude-haiku-4-5"
        );
        assert_eq!(
            TeamSessionActor::role_to_model(AgentRole::Tester),
            "claude-haiku-4-5"
        );
    }

    #[test]
    fn role_to_model_all_variants_covered() {
        // 确保所有角色都有映射，不会 panic
        for role in &[
            AgentRole::Leader,
            AgentRole::Architect,
            AgentRole::Developer,
            AgentRole::Reviewer,
            AgentRole::Tester,
            AgentRole::Researcher,
            AgentRole::Executor,
        ] {
            let model = TeamSessionActor::role_to_model(*role);
            assert!(!model.is_empty(), "role {:?} should map to a model", role);
        }
    }

    // ── upgrade_model ──────────────────────────────────────────────

    #[test]
    fn upgrade_model_escalates() {
        assert_eq!(
            TeamSessionActor::upgrade_model(AgentRole::Tester, "claude-haiku-4-5"),
            "claude-sonnet-4-5"
        );
        assert_eq!(
            TeamSessionActor::upgrade_model(AgentRole::Developer, "claude-sonnet-4-5"),
            "claude-opus-4-5"
        );
        // 已到顶或未知模型 → 用最强
        assert_eq!(
            TeamSessionActor::upgrade_model(AgentRole::Architect, "claude-opus-4-5"),
            "claude-opus-4-5"
        );
    }

    #[test]
    fn upgrade_model_unknown_model_returns_opus() {
        assert_eq!(
            TeamSessionActor::upgrade_model(AgentRole::Developer, "unknown-model"),
            "claude-opus-4-5"
        );
    }

    #[test]
    fn upgrade_model_empty_string_returns_opus() {
        assert_eq!(
            TeamSessionActor::upgrade_model(AgentRole::Tester, ""),
            "claude-opus-4-5"
        );
    }

    // ── quick_quality_check ────────────────────────────────────────

    #[test]
    fn quick_quality_check_rejects_empty() {
        assert!(!TeamSessionActor::quick_quality_check(
            "",
            AgentRole::Developer
        ));
        assert!(!TeamSessionActor::quick_quality_check(
            "   ",
            AgentRole::Architect
        ));
        // 太短的输出也拒绝
        assert!(!TeamSessionActor::quick_quality_check(
            "ok",
            AgentRole::Developer
        ));
    }

    #[test]
    fn quick_quality_check_accepts_code() {
        assert!(TeamSessionActor::quick_quality_check(
            "```rust\nfn main() {}\n```",
            AgentRole::Developer
        ));
    }

    #[test]
    fn quick_quality_check_accepts_architect_patterns() {
        // Architect 特征词汇
        assert!(TeamSessionActor::quick_quality_check(
            "## 架构设计\n模块划分：...",
            AgentRole::Architect
        ));
        assert!(TeamSessionActor::quick_quality_check(
            "architecture design document",
            AgentRole::Architect
        ));
        assert!(TeamSessionActor::quick_quality_check(
            "```mermaid\nflowchart\n```",
            AgentRole::Architect
        ));
    }

    #[test]
    fn quick_quality_check_rejects_architect_irrelevant() {
        // 中等长度（20-200 字符）且不含架构关键词 → 拒绝
        assert!(!TeamSessionActor::quick_quality_check(
            "some random text without any key architectural terms",
            AgentRole::Architect
        ));
    }

    #[test]
    fn quick_quality_check_reviewer_always_true() {
        // Reviewer 总是返回 true（设计如此），但仍需满足最小长度
        assert!(TeamSessionActor::quick_quality_check(
            "reviewer output with sufficient length here",
            AgentRole::Reviewer
        ));
        assert!(TeamSessionActor::quick_quality_check(
            "this is more than twenty characters!", // reviewer 仍然需要满足最小长度
            AgentRole::Reviewer
        ));
    }

    #[test]
    fn quick_quality_check_tester_detects_patterns() {
        assert!(TeamSessionActor::quick_quality_check(
            "#[test]\nfn test_foo() {}",
            AgentRole::Tester
        ));
        assert!(TeamSessionActor::quick_quality_check(
            "assert_eq!(result, expected_value);",
            AgentRole::Tester
        ));
        assert!(TeamSessionActor::quick_quality_check(
            "mod tests {\n    fn test_features() {}",
            AgentRole::Tester
        ));
        assert!(TeamSessionActor::quick_quality_check(
            "describe('foo', () => { it('bar', ...) })",
            AgentRole::Tester
        ));
        assert!(TeamSessionActor::quick_quality_check(
            "@Test\npublic void testFoo()",
            AgentRole::Tester
        ));
    }

    #[test]
    fn quick_quality_check_tester_rejects_irrelevant() {
        assert!(!TeamSessionActor::quick_quality_check(
            "some random text without test keywords",
            AgentRole::Tester
        ));
    }

    #[test]
    fn quick_quality_check_developer_accepts_various_patterns() {
        assert!(TeamSessionActor::quick_quality_check(
            "impl Foo { fn bar() {} }",
            AgentRole::Developer
        ));
        assert!(TeamSessionActor::quick_quality_check(
            "enum Color { Red, Green }",
            AgentRole::Developer
        ));
        assert!(TeamSessionActor::quick_quality_check(
            "mod utils;\npub fn helper() {}",
            AgentRole::Developer
        ));
    }

    #[test]
    fn quick_quality_check_long_output_always_accepted() {
        let long_text = "x".repeat(101);
        assert!(TeamSessionActor::quick_quality_check(
            &long_text,
            AgentRole::Developer
        ));
        assert!(TeamSessionActor::quick_quality_check(
            &long_text,
            AgentRole::Architect
        ));
        assert!(TeamSessionActor::quick_quality_check(
            &long_text,
            AgentRole::Tester
        ));
        assert!(TeamSessionActor::quick_quality_check(
            &long_text,
            AgentRole::Leader
        ));
        assert!(TeamSessionActor::quick_quality_check(
            &long_text,
            AgentRole::Researcher
        ));
        assert!(TeamSessionActor::quick_quality_check(
            &long_text,
            AgentRole::Executor
        ));
    }

    #[test]
    fn quick_quality_check_accepts_rust_patterns() {
        assert!(TeamSessionActor::quick_quality_check(
            "pub fn main() { println!(\"hi\"); }",
            AgentRole::Developer
        ));
        assert!(TeamSessionActor::quick_quality_check(
            "struct Foo { x: i32 }",
            AgentRole::Developer
        ));
        assert!(TeamSessionActor::quick_quality_check(
            "let mut x = 42; x += 1;",
            AgentRole::Developer
        ));
    }

    #[test]
    fn quick_quality_check_boundary_20_chars() {
        // 正好 20 字符 — 边界条件
        assert!(!TeamSessionActor::quick_quality_check(
            "12345678901234567890",
            AgentRole::Developer
        ));
        // 19 字符也拒绝
        assert!(!TeamSessionActor::quick_quality_check(
            "1234567890123456789",
            AgentRole::Developer
        ));
    }

    #[test]
    fn quick_quality_check_medium_output_needs_keywords() {
        // 21 字符，不含关键词 → 拒绝
        assert!(!TeamSessionActor::quick_quality_check(
            "xyz zyx xyz zyx xyz zyx", // > 20 chars, no code keywords
            AgentRole::Developer
        ));
        // 21 字符含关键词 → 接受
        assert!(TeamSessionActor::quick_quality_check(
            "abc fn abc fn abc fn abc", // has "fn "
            AgentRole::Developer
        ));
    }

    // ── build_context ──────────────────────────────────────────────

    #[test]
    fn build_context_includes_role_instruction() {
        let ctx = TeamSessionActor::build_context("write a CLI", "", AgentRole::Architect);
        assert!(ctx.contains("write a CLI"));
        assert!(ctx.contains("系统架构师"));
        assert!(ctx.contains("模块划分"));

        let ctx = TeamSessionActor::build_context("write a CLI", "", AgentRole::Developer);
        assert!(ctx.contains("write a CLI"));
        assert!(ctx.contains("开发者"));
        assert!(ctx.contains("代码"));

        let ctx = TeamSessionActor::build_context("write a CLI", "", AgentRole::Reviewer);
        assert!(ctx.contains("代码审查者"));
        assert!(ctx.contains("正确性"));

        let ctx = TeamSessionActor::build_context("write a CLI", "", AgentRole::Tester);
        assert!(ctx.contains("测试工程师"));
        assert!(ctx.contains("单元测试"));
    }

    #[test]
    fn build_context_fallback_role() {
        // Leader 等没有特定中文指令，应使用回退
        let ctx = TeamSessionActor::build_context("task", "", AgentRole::Leader);
        assert!(ctx.contains("任务"));
        assert!(ctx.contains("task"));
        assert!(!ctx.contains("前面的工作成果")); // 没有累积上下文时不显示
    }

    #[test]
    fn build_context_appends_accumulated_work() {
        let prev = "## Architect 输出\nsome design doc";
        let ctx = TeamSessionActor::build_context("task", prev, AgentRole::Developer);
        assert!(ctx.contains("前面的工作成果"));
        assert!(ctx.contains("some design doc"));
        assert!(ctx.contains("task"));
    }

    #[test]
    fn build_context_empty_accumulated_no_section() {
        let ctx = TeamSessionActor::build_context("task", "", AgentRole::Developer);
        assert!(!ctx.contains("前面的工作成果"));
    }

    #[test]
    fn build_context_with_whitespace_accumulated() {
        let ctx = TeamSessionActor::build_context("task", "  ", AgentRole::Developer);
        assert!(ctx.contains("前面的工作成果"));
        assert!(ctx.contains("task"));
    }

    // ── resolve_execution_order ────────────────────────────────────

    #[test]
    fn resolve_order_follows_hierarchy() {
        let mut team = Team::new("test");
        let a = TeamMember::new(AgentRole::Architect, "a", CliProvider::Claude);
        let d = TeamMember::new(AgentRole::Developer, "d", CliProvider::Claude);
        let r = TeamMember::new(AgentRole::Reviewer, "r", CliProvider::Claude);

        let aid = a.id;
        let did = d.id;
        let rid = r.id;

        team.add_member(a);
        team.add_member(d);
        team.add_member(r);
        team.set_leader(aid);
        team.add_dependency(aid, did);
        team.add_dependency(did, rid);

        let order = TeamSessionActor::resolve_execution_order(&team);
        // Architect 在 Developer 之前，Developer 在 Reviewer 之前
        let aidx = order.iter().position(|id| *id == aid).unwrap();
        let didx = order.iter().position(|id| *id == did).unwrap();
        let ridx = order.iter().position(|id| *id == rid).unwrap();
        assert!(aidx < didx);
        assert!(didx < ridx);
    }

    #[test]
    fn resolve_order_with_no_hierarchy_returns_all_members() {
        let mut team = Team::new("test");
        let a = TeamMember::new(AgentRole::Architect, "a", CliProvider::Claude);
        let d = TeamMember::new(AgentRole::Developer, "d", CliProvider::Claude);
        let r = TeamMember::new(AgentRole::Reviewer, "r", CliProvider::Claude);

        let aid = a.id;
        let did = d.id;
        let rid = r.id;

        team.add_member(a);
        team.add_member(d);
        team.add_member(r);

        let order = TeamSessionActor::resolve_execution_order(&team);
        assert_eq!(order.len(), 3);
        assert!(order.contains(&aid));
        assert!(order.contains(&did));
        assert!(order.contains(&rid));
    }

    #[test]
    fn resolve_order_with_disconnected_graph_appends_remaining() {
        let mut team = Team::new("test");
        let a = TeamMember::new(AgentRole::Architect, "a", CliProvider::Claude);
        let d = TeamMember::new(AgentRole::Developer, "d", CliProvider::Claude);
        let r = TeamMember::new(AgentRole::Reviewer, "r", CliProvider::Claude);

        let aid = a.id;
        let did = d.id;
        let rid = r.id;

        team.add_member(a);
        team.add_member(d);
        team.add_member(r);
        // 只把 architect 和 developer 放入 hierarchy，reviewer 游离在外
        team.set_leader(aid);
        team.add_dependency(aid, did);

        let order = TeamSessionActor::resolve_execution_order(&team);
        assert_eq!(order.len(), 3);
        // Architect 在 Developer 之前
        let aidx = order.iter().position(|id| *id == aid).unwrap();
        let didx = order.iter().position(|id| *id == did).unwrap();
        assert!(aidx < didx);
        // Reviewer（不在 hierarchy 中）被追加到末尾
        let ridx = order.iter().position(|id| *id == rid).unwrap();
        assert!(ridx > aidx);
        assert!(ridx > didx);
    }

    #[test]
    fn resolve_order_single_agent() {
        let mut team = Team::new("solo");
        let a = TeamMember::new(AgentRole::Developer, "solo", CliProvider::Claude);
        let aid = a.id;
        team.add_member(a);

        let order = TeamSessionActor::resolve_execution_order(&team);
        assert_eq!(order, vec![aid]);
    }

    #[test]
    fn resolve_order_empty_team() {
        let team = Team::new("empty");
        let order = TeamSessionActor::resolve_execution_order(&team);
        assert!(order.is_empty());
    }

    #[test]
    fn resolve_order_linear_chain_of_five() {
        let mut team = Team::new("chain");
        let ids: Vec<AgentId> = (0..5)
            .map(|i| {
                let m = TeamMember::new(
                    AgentRole::Developer,
                    &format!("agent-{}", i),
                    CliProvider::Claude,
                );
                let id = m.id;
                team.add_member(m);
                id
            })
            .collect();

        // 设置链：0→1→2→3→4
        team.set_leader(ids[0]);
        for i in 0..4 {
            team.add_dependency(ids[i], ids[i + 1]);
        }

        let order = TeamSessionActor::resolve_execution_order(&team);
        // 顺序应该是 [0, 1, 2, 3, 4]
        for i in 0..5 {
            let pos = order.iter().position(|id| *id == ids[i]).unwrap();
            assert_eq!(pos, i, "agent {} should be at position {}", i, i);
        }
    }

    #[test]
    fn resolve_order_diamond_dependency() {
        // 架构：A 依赖 B 和 C，B 和 C 都依赖 D
        // 预期：B 和 C 在 A 之前，D 在 B 和 C 之前
        // A→B, A→C, B→D, C→D 意味着 A 先执行，然后 B/C，最后 D
        // 合法顺序: [A, B, C, D] 或 [A, C, B, D]
        let mut team = Team::new("diamond");
        let a = TeamMember::new(AgentRole::Leader, "A", CliProvider::Claude);
        let b = TeamMember::new(AgentRole::Architect, "B", CliProvider::Claude);
        let c = TeamMember::new(AgentRole::Developer, "C", CliProvider::Claude);
        let d = TeamMember::new(AgentRole::Reviewer, "D", CliProvider::Claude);

        let aid = a.id;
        let bid = b.id;
        let cid = c.id;
        let did = d.id;

        team.add_member(a);
        team.add_member(b);
        team.add_member(c);
        team.add_member(d);

        team.set_leader(aid);
        team.add_dependency(aid, bid);
        team.add_dependency(aid, cid);
        team.add_dependency(bid, did);
        team.add_dependency(cid, did);

        let order = TeamSessionActor::resolve_execution_order(&team);
        assert_eq!(order.len(), 4);

        // A 必须在 B 和 C 之前
        let aidx = order.iter().position(|id| *id == aid).unwrap();
        let bidx = order.iter().position(|id| *id == bid).unwrap();
        let cidx = order.iter().position(|id| *id == cid).unwrap();
        let didx = order.iter().position(|id| *id == did).unwrap();
        assert!(aidx < bidx, "A should be before B");
        assert!(aidx < cidx, "A should be before C");

        // B 和 C 必须在 D 之前
        assert!(bidx < didx, "B should be before D");
        assert!(cidx < didx, "C should be before D");
    }

    #[test]
    fn resolve_order_members_not_in_hierarchy_are_appended() {
        let mut team = Team::new("hybrid");
        let a = TeamMember::new(AgentRole::Architect, "arch", CliProvider::Claude);
        let d = TeamMember::new(AgentRole::Developer, "dev", CliProvider::Claude);
        let r = TeamMember::new(AgentRole::Reviewer, "review", CliProvider::Claude);
        let t = TeamMember::new(AgentRole::Tester, "test", CliProvider::Claude);

        let aid = a.id;
        let did = d.id;
        let rid = r.id;
        let tid = t.id;

        team.add_member(a);
        team.add_member(d);
        team.add_member(r);
        team.add_member(t);

        // 只有 architect → developer → reviewer 在 hierarchy 中
        team.set_leader(aid);
        team.add_dependency(aid, did);
        team.add_dependency(did, rid);

        let order = TeamSessionActor::resolve_execution_order(&team);
        assert_eq!(order.len(), 4);

        // tester（不在 hierarchy）应在末尾
        let tidx = order.iter().position(|id| *id == tid).unwrap();
        assert_eq!(tidx, 3);
    }

    // ── AgentResult / ChainResult ──────────────────────────────────

    #[test]
    fn agent_result_can_be_serialized() {
        let result = AgentResult {
            agent_id: AgentId::new(),
            role: AgentRole::Developer,
            agent_name: "test".into(),
            text: "some output".into(),
            duration_ms: 1000,
            attempts: 1,
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: AgentResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.agent_name, "test");
        assert_eq!(deserialized.duration_ms, 1000);
        assert_eq!(deserialized.attempts, 1);
    }

    #[test]
    fn chain_result_tracks_agent_results() {
        let result = ChainResult {
            execution_id: Uuid::new_v4(),
            task: "integration test".into(),
            agent_results: vec![],
            total_duration_ms: 0,
        };
        assert!(result.agent_results.is_empty());
        assert_eq!(result.total_duration_ms, 0);
    }

    #[test]
    fn auto_yes_prefix_is_non_empty() {
        let prefix = TeamSessionActor::auto_yes_prefix();
        assert!(!prefix.is_empty());
        assert!(prefix.contains("auto-yes"));
    }

    // ── UserMessage::parse ──────────────────────────────────────────

    #[test]
    fn user_message_parse_with_at_prefix() {
        let msg = UserMessage::parse("@architect 请用 Rust 实现这个功能");
        assert_eq!(msg.target_agent, Some("architect".into()));
        assert_eq!(msg.content, "请用 Rust 实现这个功能");
    }

    #[test]
    fn user_message_parse_without_at_prefix() {
        let msg = UserMessage::parse("请用 Rust 实现这个功能");
        assert_eq!(msg.target_agent, None);
        assert_eq!(msg.content, "请用 Rust 实现这个功能");
    }

    #[test]
    fn user_message_parse_empty_input() {
        let msg = UserMessage::parse("");
        assert_eq!(msg.target_agent, None);
        assert_eq!(msg.content, "");
    }

    #[test]
    fn user_message_parse_at_only_no_content() {
        let msg = UserMessage::parse("@architect");
        assert_eq!(msg.target_agent, Some("architect".into()));
        assert_eq!(msg.content, "");
    }

    #[test]
    fn user_message_parse_whitespace_trimmed() {
        let msg = UserMessage::parse("  @developer   请添加错误处理  ");
        assert_eq!(msg.target_agent, Some("developer".into()));
        assert_eq!(msg.content, "请添加错误处理");
    }

    #[test]
    fn user_message_parse_multiple_words_no_at() {
        let msg = UserMessage::parse("hello world this is a test");
        assert_eq!(msg.target_agent, None);
        assert_eq!(msg.content, "hello world this is a test");
    }

    #[test]
    fn user_message_parse_at_with_trailing_spaces() {
        let msg = UserMessage::parse("@reviewer   ");
        assert_eq!(msg.target_agent, Some("reviewer".into()));
        assert_eq!(msg.content, "");
    }

    #[test]
    fn user_message_parse_at_case_preserved() {
        let msg = UserMessage::parse("@Architect Please review");
        assert_eq!(msg.target_agent, Some("Architect".into()));
        assert_eq!(msg.content, "Please review");
    }

    // ── UserMessage::is_targeting ────────────────────────────────────

    #[test]
    fn user_message_is_targeting_when_no_target() {
        let msg = UserMessage {
            target_agent: None,
            content: "fix this".into(),
        };
        assert!(msg.is_targeting("architect"));
        assert!(msg.is_targeting("developer"));
        assert!(msg.is_targeting("anyone"));
    }

    #[test]
    fn user_message_is_targeting_specific_agent() {
        let msg = UserMessage {
            target_agent: Some("developer".into()),
            content: "fix this".into(),
        };
        assert!(msg.is_targeting("developer"));
        assert!(!msg.is_targeting("architect"));
        assert!(!msg.is_targeting("reviewer"));
    }

    #[test]
    fn user_message_is_targeting_case_insensitive() {
        let msg = UserMessage {
            target_agent: Some("Developer".into()),
            content: "fix this".into(),
        };
        assert!(msg.is_targeting("developer"));
        assert!(msg.is_targeting("Developer"));
        assert!(!msg.is_targeting("architect"));
    }

    #[test]
    fn user_message_is_targeting_empty_target() {
        let msg = UserMessage {
            target_agent: Some(String::new()),
            content: "hi".into(),
        };
        assert!(!msg.is_targeting("developer"));
        assert!(!msg.is_targeting("architect"));
    }

    // ── inject_user_messages ────────────────────────────────────────

    #[test]
    fn inject_user_messages_no_channel_does_nothing() {
        let mut context = "original context".to_string();
        let count = TeamSessionActor::inject_user_messages(
            &None,
            "developer",
            &mut context,
            &None,
            AgentId::new(),
            AgentRole::Developer,
        );
        assert_eq!(count, 0);
        assert_eq!(context, "original context");
    }

    #[test]
    fn inject_user_messages_empty_channel_does_nothing() {
        let (tx, rx) = mpsc::unbounded_channel();
        let queue = std::sync::Mutex::new(UserMessageQueue::new(rx));
        let mut context = "original context".to_string();
        let count = TeamSessionActor::inject_user_messages(
            &Some(queue),
            "developer",
            &mut context,
            &None,
            AgentId::new(),
            AgentRole::Developer,
        );
        assert_eq!(count, 0);
        assert_eq!(context, "original context");
        drop(tx);
    }

    #[test]
    fn inject_user_messages_injects_untargeted_messages() {
        let (tx, rx) = mpsc::unbounded_channel();
        tx.send(UserMessage {
            target_agent: None,
            content: "please add tests".into(),
        })
        .unwrap();
        let queue = std::sync::Mutex::new(UserMessageQueue::new(rx));
        let mut context = "original".to_string();
        let count = TeamSessionActor::inject_user_messages(
            &Some(queue),
            "developer",
            &mut context,
            &None,
            AgentId::new(),
            AgentRole::Developer,
        );
        assert_eq!(count, 1);
        assert!(context.contains("original"));
        assert!(context.contains("用户反馈/指令"));
        assert!(context.contains("please add tests"));
    }

    #[test]
    fn inject_user_messages_skips_other_agent_messages() {
        let (tx, rx) = mpsc::unbounded_channel();
        tx.send(UserMessage {
            target_agent: Some("architect".into()),
            content: "redesign this".into(),
        })
        .unwrap();
        let queue_opt = Some(std::sync::Mutex::new(UserMessageQueue::new(rx)));
        let mut context = "original".to_string();
        let count = TeamSessionActor::inject_user_messages(
            &queue_opt,
            "developer",
            &mut context,
            &None,
            AgentId::new(),
            AgentRole::Developer,
        );
        assert_eq!(count, 0);
        assert_eq!(context, "original");
        // Verify message preserved in queue for architect
        let mut guard = queue_opt.as_ref().unwrap().lock().unwrap();
        let arch_msgs = guard.drain_for("architect");
        assert_eq!(arch_msgs.len(), 1);
        assert_eq!(arch_msgs[0].content, "redesign this");
    }

    #[test]
    fn inject_user_messages_multiple_messages() {
        let (tx, rx) = mpsc::unbounded_channel();
        tx.send(UserMessage {
            target_agent: Some("developer".into()),
            content: "fix the bug".into(),
        })
        .unwrap();
        tx.send(UserMessage {
            target_agent: None,
            content: "add error handling".into(),
        })
        .unwrap();
        // This one should be skipped (targeted at architect)
        tx.send(UserMessage {
            target_agent: Some("architect".into()),
            content: "redesign".into(),
        })
        .unwrap();
        let queue_opt = Some(std::sync::Mutex::new(UserMessageQueue::new(rx)));
        let mut context = "".to_string();
        let count = TeamSessionActor::inject_user_messages(
            &queue_opt,
            "developer",
            &mut context,
            &None,
            AgentId::new(),
            AgentRole::Developer,
        );
        assert_eq!(count, 2);
        assert!(context.contains("fix the bug"));
        assert!(context.contains("add error handling"));
        assert!(!context.contains("redesign"));
        // Verify architect message preserved in queue
        let mut guard = queue_opt.as_ref().unwrap().lock().unwrap();
        let arch_msgs = guard.drain_for("architect");
        assert_eq!(arch_msgs.len(), 1);
        assert_eq!(arch_msgs[0].content, "redesign");
    }

    #[test]
    fn inject_user_messages_mixed_targeting_for_agent() {
        let (tx, rx) = mpsc::unbounded_channel();
        tx.send(UserMessage {
            target_agent: Some("tester".into()),
            content: "test edge cases".into(),
        })
        .unwrap();
        tx.send(UserMessage {
            target_agent: None,
            content: "general feedback".into(),
        })
        .unwrap();
        tx.send(UserMessage {
            target_agent: Some("tester".into()),
            content: "cover error paths".into(),
        })
        .unwrap();
        let queue = std::sync::Mutex::new(UserMessageQueue::new(rx));
        let mut context = "".to_string();
        let count = TeamSessionActor::inject_user_messages(
            &Some(queue),
            "tester",
            &mut context,
            &None,
            AgentId::new(),
            AgentRole::Tester,
        );
        assert_eq!(count, 3);
        assert!(context.contains("test edge cases"));
        assert!(context.contains("general feedback"));
        assert!(context.contains("cover error paths"));
    }

    // ── UserMessageQueue ─────────────────────────────────────────

    #[test]
    fn user_message_queue_drain_for_empty() {
        let (_tx, rx) = mpsc::unbounded_channel();
        let mut queue = UserMessageQueue::new(rx);
        let msgs = queue.drain_for("developer");
        assert!(msgs.is_empty());
    }

    #[test]
    fn user_message_queue_drain_for_filters_by_target() {
        let (tx, rx) = mpsc::unbounded_channel();
        tx.send(UserMessage {
            target_agent: Some("architect".into()),
            content: "design".into(),
        })
        .unwrap();
        tx.send(UserMessage {
            target_agent: None,
            content: "general".into(),
        })
        .unwrap();
        tx.send(UserMessage {
            target_agent: Some("developer".into()),
            content: "code".into(),
        })
        .unwrap();
        let mut queue = UserMessageQueue::new(rx);

        let dev_msgs = queue.drain_for("developer");
        assert_eq!(dev_msgs.len(), 2); // targeted + untargeted
        let names: Vec<Option<String>> = dev_msgs.iter().map(|m| m.target_agent.clone()).collect();
        assert!(names.contains(&Some("developer".into())));
        assert!(names.contains(&None));
    }

    #[test]
    fn user_message_queue_preserves_non_targeted_across_calls() {
        let (tx, rx) = mpsc::unbounded_channel();
        tx.send(UserMessage {
            target_agent: Some("architect".into()),
            content: "design".into(),
        })
        .unwrap();
        tx.send(UserMessage {
            target_agent: Some("developer".into()),
            content: "code".into(),
        })
        .unwrap();
        let mut queue = UserMessageQueue::new(rx);

        // First call for developer — should pick up "code" + untargeted
        let dev_msgs = queue.drain_for("developer");
        assert_eq!(dev_msgs.len(), 1); // only "code" is developer
        assert_eq!(dev_msgs[0].content, "code");

        // Architect message preserved for next call
        let arch_msgs = queue.drain_for("architect");
        assert_eq!(arch_msgs.len(), 1);
        assert_eq!(arch_msgs[0].content, "design");
    }

    #[test]
    fn user_message_queue_no_loss_with_multiple_agents() {
        let (tx, rx) = mpsc::unbounded_channel();
        tx.send(UserMessage {
            target_agent: Some("alpha".into()),
            content: "a".into(),
        })
        .unwrap();
        tx.send(UserMessage {
            target_agent: Some("beta".into()),
            content: "b".into(),
        })
        .unwrap();
        tx.send(UserMessage {
            target_agent: Some("gamma".into()),
            content: "c".into(),
        })
        .unwrap();
        let mut queue = UserMessageQueue::new(rx);

        let alpha = queue.drain_for("alpha");
        let beta = queue.drain_for("beta");
        let gamma = queue.drain_for("gamma");

        assert_eq!(alpha.len(), 1, "alpha lost");
        assert_eq!(beta.len(), 1, "beta lost");
        assert_eq!(gamma.len(), 1, "gamma lost");
    }

    #[test]
    fn user_message_queue_untargeted_goes_to_all() {
        let (tx, rx) = mpsc::unbounded_channel();
        tx.send(UserMessage {
            target_agent: None,
            content: "everyone".into(),
        })
        .unwrap();
        let mut queue = UserMessageQueue::new(rx);

        let first = queue.drain_for("first");
        assert_eq!(first.len(), 1);
        // Untargeted message consumed — it was injected into first agent
        let second = queue.drain_for("second");
        assert!(
            second.is_empty(),
            "untargeted message should not be duplicated"
        );
    }

    #[test]
    fn user_message_queue_messages_arriving_later() {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut queue = UserMessageQueue::new(rx);

        // No messages yet
        assert!(queue.drain_for("dev").is_empty());

        // Messages arrive
        tx.send(UserMessage {
            target_agent: None,
            content: "late".into(),
        })
        .unwrap();

        // Now drain_for picks them up
        let msgs = queue.drain_for("dev");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].content, "late");
    }

    // ── inject_user_messages with events channel ─────────────────

    #[test]
    fn inject_user_messages_sends_events_for_targeted() {
        let (tx, rx) = mpsc::unbounded_channel();
        tx.send(UserMessage {
            target_agent: Some("dev".into()),
            content: "fix it".into(),
        })
        .unwrap();
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        let queue = std::sync::Mutex::new(UserMessageQueue::new(rx));

        let count = TeamSessionActor::inject_user_messages(
            &Some(queue),
            "dev",
            &mut String::new(),
            &Some(event_tx),
            AgentId::new(),
            AgentRole::Developer,
        );
        assert_eq!(count, 1);

        let event = event_rx.try_recv().unwrap();
        match event {
            ChainEvent::AgentReceivedInput { message, .. } => {
                assert_eq!(message, "fix it");
            }
            other => panic!("expected AgentReceivedInput, got {:?}", other),
        }
    }

    #[test]
    fn inject_user_messages_no_events_for_non_targeted() {
        let (tx, rx) = mpsc::unbounded_channel();
        tx.send(UserMessage {
            target_agent: Some("architect".into()),
            content: "design".into(),
        })
        .unwrap();
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        let queue = std::sync::Mutex::new(UserMessageQueue::new(rx));

        let count = TeamSessionActor::inject_user_messages(
            &Some(queue),
            "dev",
            &mut String::new(),
            &Some(event_tx),
            AgentId::new(),
            AgentRole::Developer,
        );
        assert_eq!(count, 0);
        assert!(event_rx.try_recv().is_err());
    }

    #[test]
    fn inject_user_messages_multiple_events_for_multiple_messages() {
        let (tx, rx) = mpsc::unbounded_channel();
        tx.send(UserMessage {
            target_agent: None,
            content: "first".into(),
        })
        .unwrap();
        tx.send(UserMessage {
            target_agent: None,
            content: "second".into(),
        })
        .unwrap();
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        let queue = std::sync::Mutex::new(UserMessageQueue::new(rx));

        TeamSessionActor::inject_user_messages(
            &Some(queue),
            "dev",
            &mut String::new(),
            &Some(event_tx),
            AgentId::new(),
            AgentRole::Developer,
        );
        let e1 = event_rx.try_recv().unwrap();
        let e2 = event_rx.try_recv().unwrap();
        assert!(event_rx.try_recv().is_err());

        match e1 {
            ChainEvent::AgentReceivedInput { ref message, .. } => assert_eq!(message, "first"),
            _ => panic!(),
        }
        match e2 {
            ChainEvent::AgentReceivedInput { ref message, .. } => assert_eq!(message, "second"),
            _ => panic!(),
        }
    }

    // ── UserMessage::parse additional edge cases ─────────────────

    #[test]
    fn user_message_parse_at_symbol_only() {
        let msg = UserMessage::parse("@");
        assert_eq!(msg.target_agent, Some("".into()));
        assert_eq!(msg.content, "");
    }

    #[test]
    fn user_message_parse_at_with_only_whitespace() {
        let msg = UserMessage::parse("@   ");
        assert_eq!(msg.target_agent, Some("".into()));
        assert_eq!(msg.content, "");
    }

    #[test]
    fn user_message_parse_unicode_agent_name() {
        let msg = UserMessage::parse("@ロボット コードを書いて");
        assert_eq!(msg.target_agent, Some("ロボット".into()));
        assert_eq!(msg.content, "コードを書いて");
    }

    #[test]
    fn user_message_parse_very_long_content() {
        let content = "x".repeat(10_000);
        let input = format!("@architect {}", content);
        let msg = UserMessage::parse(&input);
        assert_eq!(msg.target_agent, Some("architect".into()));
        assert_eq!(msg.content.len(), 10_000);
    }

    #[test]
    fn user_message_parse_newline_in_content() {
        let msg = UserMessage::parse("@developer\nfix this bug");
        assert_eq!(msg.target_agent, Some("developer".into()));
        assert_eq!(msg.content, "fix this bug");
    }

    #[test]
    fn user_message_parse_no_space_after_at() {
        let msg = UserMessage::parse("@developermessage");
        assert_eq!(msg.target_agent, Some("developermessage".into()));
        assert_eq!(msg.content, "");
    }

    #[test]
    fn user_message_parse_multiple_at_signs() {
        let msg = UserMessage::parse("@@architect hello");
        assert_eq!(msg.target_agent, Some("@architect".into()));
        assert_eq!(msg.content, "hello");
    }

    // ── UserMessage::is_targeting additional edge cases ──────────

    #[test]
    fn user_message_is_targeting_both_empty() {
        let msg = UserMessage {
            target_agent: Some("".into()),
            content: "hi".into(),
        };
        assert!(msg.is_targeting(""));
        assert!(!msg.is_targeting("anything"));
    }

    #[test]
    fn user_message_is_targeting_special_chars_in_name() {
        let msg = UserMessage {
            target_agent: Some("test-agent_123".into()),
            content: "hi".into(),
        };
        assert!(msg.is_targeting("test-agent_123"));
        assert!(!msg.is_targeting("test-agent"));
    }

    #[test]
    fn user_message_is_targeting_unicode_agent() {
        let msg = UserMessage {
            target_agent: Some("ロボット".into()),
            content: "hi".into(),
        };
        assert!(msg.is_targeting("ロボット"));
        assert!(!msg.is_targeting("robot"));
    }

    #[test]
    fn user_message_is_targeting_ascii_case_edge() {
        let msg = UserMessage {
            target_agent: Some("DEV".into()),
            content: "hi".into(),
        };
        assert!(msg.is_targeting("dev"));
        assert!(msg.is_targeting("Dev"));
        assert!(msg.is_targeting("DEV"));
    }

    // ── ChainEvent — all variants constructible and debuggable ──

    #[test]
    fn chain_event_agent_started_debug() {
        let e = ChainEvent::AgentStarted {
            agent_id: AgentId::new(),
            role: AgentRole::Developer,
            name: "dev-1".into(),
        };
        let s = format!("{:?}", e);
        assert!(s.contains("AgentStarted"));
        assert!(s.contains("dev-1"));
    }

    #[test]
    fn chain_event_agent_completed_debug() {
        let e = ChainEvent::AgentCompleted {
            agent_id: AgentId::new(),
            role: AgentRole::Reviewer,
            name: "rev".into(),
            summary: "done".into(),
        };
        let s = format!("{:?}", e);
        assert!(s.contains("AgentCompleted"));
        assert!(s.contains("done"));
    }

    #[test]
    fn chain_event_chain_completed_debug() {
        let e = ChainEvent::ChainCompleted {
            total_duration_ms: 1234,
        };
        let s = format!("{:?}", e);
        assert!(s.contains("ChainCompleted"));
        assert!(s.contains("1234"));
    }

    #[test]
    fn chain_event_chain_failed_debug() {
        let e = ChainEvent::ChainFailed {
            error: "timeout".into(),
        };
        let s = format!("{:?}", e);
        assert!(s.contains("ChainFailed"));
        assert!(s.contains("timeout"));
    }

    #[test]
    fn chain_event_waiting_for_input_debug() {
        let e = ChainEvent::AgentWaitingForInput {
            agent_id: AgentId::new(),
            role: AgentRole::Tester,
            name: "tester".into(),
            reason: "need input".into(),
        };
        let s = format!("{:?}", e);
        assert!(s.contains("AgentWaitingForInput"));
        assert!(s.contains("need input"));
    }

    #[test]
    fn chain_event_received_input_debug() {
        let e = ChainEvent::AgentReceivedInput {
            agent_id: AgentId::new(),
            message: "user said hi".into(),
        };
        let s = format!("{:?}", e);
        assert!(s.contains("AgentReceivedInput"));
        assert!(s.contains("user said hi"));
    }

    #[test]
    fn chain_event_all_variants_clone() {
        let id = AgentId::new();
        let events = vec![
            ChainEvent::AgentStarted {
                agent_id: id,
                role: AgentRole::Architect,
                name: "a".into(),
            },
            ChainEvent::AgentCompleted {
                agent_id: id,
                role: AgentRole::Developer,
                name: "d".into(),
                summary: "s".into(),
            },
            ChainEvent::ChainCompleted {
                total_duration_ms: 0,
            },
            ChainEvent::ChainFailed { error: "e".into() },
            ChainEvent::AgentWaitingForInput {
                agent_id: id,
                role: AgentRole::Tester,
                name: "t".into(),
                reason: "r".into(),
            },
            ChainEvent::AgentReceivedInput {
                agent_id: id,
                message: "m".into(),
            },
        ];
        for e in &events {
            let cloned = e.clone();
            assert_eq!(format!("{:?}", cloned), format!("{:?}", e));
        }
    }

    // ── quick_quality_check — all roles and boundaries ──────────

    #[test]
    fn quick_quality_check_all_roles_min_length() {
        for role in &[
            AgentRole::Leader,
            AgentRole::Architect,
            AgentRole::Developer,
            AgentRole::Reviewer,
            AgentRole::Tester,
            AgentRole::Researcher,
            AgentRole::Executor,
        ] {
            // All roles reject empty
            assert!(
                !TeamSessionActor::quick_quality_check("", *role),
                "role {:?} should reject empty",
                role
            );
        }
    }

    #[test]
    fn quick_quality_check_exactly_100_chars_needs_keywords() {
        // 100 chars is medium range (≤ 100 needs keywords, > 100 accepted)
        // At exactly 100 chars, keyword check still applies
        let text = "x".repeat(100);
        assert!(!TeamSessionActor::quick_quality_check(
            &text,
            AgentRole::Developer
        ));
        assert!(!TeamSessionActor::quick_quality_check(
            &text,
            AgentRole::Architect
        ));
        // Reviewer bypasses keyword check
        assert!(TeamSessionActor::quick_quality_check(
            &text,
            AgentRole::Reviewer
        ));

        // 101 chars bypasses keyword check for all roles
        let text = "x".repeat(101);
        assert!(TeamSessionActor::quick_quality_check(
            &text,
            AgentRole::Developer
        ));
        assert!(TeamSessionActor::quick_quality_check(
            &text,
            AgentRole::Architect
        ));
        assert!(TeamSessionActor::quick_quality_check(
            &text,
            AgentRole::Tester
        ));
    }

    #[test]
    fn quick_quality_check_exactly_99_chars_with_keywords() {
        // 99 chars with code keyword
        let mut text = "fn ".to_string();
        text.push_str(&"x".repeat(96));
        assert!(text.len() == 99);
        assert!(TeamSessionActor::quick_quality_check(
            &text,
            AgentRole::Developer
        ));
    }

    #[test]
    fn quick_quality_check_exactly_99_chars_without_keywords() {
        let text = "x".repeat(99);
        // 99 chars, no code keywords — depends on role
        // Developer needs keywords in medium range
        assert!(!TeamSessionActor::quick_quality_check(
            &text,
            AgentRole::Developer
        ));
        // Architect needs keywords
        assert!(!TeamSessionActor::quick_quality_check(
            &text,
            AgentRole::Architect
        ));
        // Reviewer bypasses keyword check > 20 chars
        assert!(TeamSessionActor::quick_quality_check(
            &text,
            AgentRole::Reviewer
        ));
    }

    #[test]
    fn quick_quality_check_researcher_role() {
        // Researcher returns true like Leader (no specific keywords checked)
        assert!(TeamSessionActor::quick_quality_check(
            "research output with sufficient length",
            AgentRole::Researcher
        ));
        assert!(TeamSessionActor::quick_quality_check(
            "short but meaningful text for researcher",
            AgentRole::Executor
        ));
    }

    #[test]
    fn quick_quality_check_whitespace_only() {
        assert!(!TeamSessionActor::quick_quality_check(
            "   ",
            AgentRole::Developer
        ));
        assert!(!TeamSessionActor::quick_quality_check(
            "\n\t\n",
            AgentRole::Architect
        ));
        assert!(!TeamSessionActor::quick_quality_check(
            "  \n  \n  ",
            AgentRole::Reviewer
        ));
    }

    // ── build_context — all roles ─────────────────────────────

    #[test]
    fn build_context_all_roles_non_empty() {
        for role in &[
            AgentRole::Leader,
            AgentRole::Architect,
            AgentRole::Developer,
            AgentRole::Reviewer,
            AgentRole::Tester,
            AgentRole::Researcher,
            AgentRole::Executor,
        ] {
            let ctx = TeamSessionActor::build_context("task", "", *role);
            assert!(
                !ctx.is_empty(),
                "role {:?} should produce non-empty context",
                role
            );
            assert!(
                ctx.contains("任务"),
                "role {:?} context should mention 任务",
                role
            );
            assert!(ctx.contains("task"));
        }
    }

    #[test]
    fn build_context_with_very_long_task() {
        let task = "x".repeat(10_000);
        let ctx = TeamSessionActor::build_context(&task, "", AgentRole::Developer);
        assert!(ctx.contains(&task));
    }

    #[test]
    fn build_context_with_very_long_accumulated() {
        let acc = "y".repeat(10_000);
        let ctx = TeamSessionActor::build_context("task", &acc, AgentRole::Developer);
        assert!(ctx.contains("前面的工作成果"));
        assert!(ctx.contains(&acc));
    }

    #[test]
    fn build_context_fallback_role_includes_task() {
        let ctx = TeamSessionActor::build_context("do something", "", AgentRole::Executor);
        assert!(ctx.contains("do something"));
        assert!(ctx.contains("任务"));
    }

    // ── resolve_execution_order — complex graphs ─────────────

    #[test]
    fn resolve_order_ten_agent_linear_chain() {
        let mut team = Team::new("long-chain");
        let ids: Vec<AgentId> = (0..10)
            .map(|i| {
                let m = TeamMember::new(
                    AgentRole::Developer,
                    &format!("a{}", i),
                    CliProvider::Claude,
                );
                let id = m.id;
                team.add_member(m);
                id
            })
            .collect();

        team.set_leader(ids[0]);
        for i in 0..9 {
            team.add_dependency(ids[i], ids[i + 1]);
        }

        let order = TeamSessionActor::resolve_execution_order(&team);
        assert_eq!(order.len(), 10);
        for i in 0..10 {
            let pos = order.iter().position(|id| *id == ids[i]).unwrap();
            assert_eq!(pos, i, "agent {} should be at position {}", i, i);
        }
    }

    #[test]
    fn resolve_order_disjoint_subgraphs() {
        // Two independent chains: A→B and C→D, plus E alone
        let mut team = Team::new("disjoint");
        let a = TeamMember::new(AgentRole::Architect, "A", CliProvider::Claude);
        let b = TeamMember::new(AgentRole::Developer, "B", CliProvider::Claude);
        let c = TeamMember::new(AgentRole::Reviewer, "C", CliProvider::Claude);
        let d = TeamMember::new(AgentRole::Tester, "D", CliProvider::Claude);
        let e = TeamMember::new(AgentRole::Leader, "E", CliProvider::Claude);

        let aid = a.id;
        let bid = b.id;
        let cid = c.id;
        let did = d.id;
        let eid = e.id;
        team.add_member(a);
        team.add_member(b);
        team.add_member(c);
        team.add_member(d);
        team.add_member(e);

        team.set_leader(aid);
        team.add_dependency(aid, bid);
        team.set_leader(cid);
        team.add_dependency(cid, did);

        let order = TeamSessionActor::resolve_execution_order(&team);
        assert_eq!(order.len(), 5);

        // Each chain maintains internal order
        let a_pos = order.iter().position(|id| *id == aid).unwrap();
        let b_pos = order.iter().position(|id| *id == bid).unwrap();
        assert!(a_pos < b_pos, "A before B");

        let c_pos = order.iter().position(|id| *id == cid).unwrap();
        let d_pos = order.iter().position(|id| *id == did).unwrap();
        assert!(c_pos < d_pos, "C before D");

        // E (not in hierarchy) at end
        let e_pos = order.iter().position(|id| *id == eid).unwrap();
        assert!(
            e_pos > a_pos && e_pos > b_pos && e_pos > c_pos && e_pos > d_pos,
            "E (no hierarchy) should be at end"
        );
    }

    // ── AgentResult / ChainResult additional ─────────────────

    #[test]
    fn agent_result_round_trip_with_all_fields() {
        let result = AgentResult {
            agent_id: AgentId::new(),
            role: AgentRole::Reviewer,
            agent_name: "code-review".into(),
            text: "looks good".into(),
            duration_ms: u64::MAX,
            attempts: u32::MAX,
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: AgentResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.agent_name, "code-review");
        assert_eq!(back.role, AgentRole::Reviewer);
        assert_eq!(back.duration_ms, u64::MAX);
        assert_eq!(back.attempts, u32::MAX);
    }

    #[test]
    fn agent_result_long_text_round_trip() {
        let result = AgentResult {
            agent_id: AgentId::new(),
            role: AgentRole::Developer,
            agent_name: "dev".into(),
            text: "x".repeat(100_000),
            duration_ms: 0,
            attempts: 1,
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: AgentResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.text.len(), 100_000);
    }

    #[test]
    fn agent_result_empty_text() {
        let result = AgentResult {
            agent_id: AgentId::new(),
            role: AgentRole::Tester,
            agent_name: "tester".into(),
            text: String::new(),
            duration_ms: 0,
            attempts: 0,
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: AgentResult = serde_json::from_str(&json).unwrap();
        assert!(back.text.is_empty());
        assert_eq!(back.attempts, 0);
    }

    // ── role_to_model exhaustive ────────────────────────────

    #[test]
    fn role_to_model_no_empty_or_none() {
        for role in &[
            AgentRole::Leader,
            AgentRole::Architect,
            AgentRole::Developer,
            AgentRole::Reviewer,
            AgentRole::Tester,
            AgentRole::Researcher,
            AgentRole::Executor,
        ] {
            let model = TeamSessionActor::role_to_model(*role);
            assert!(
                !model.is_empty(),
                "role {:?} should map to non-empty model",
                role
            );
            assert!(
                model.contains("claude"),
                "role {:?} map '{}' should contain 'claude'",
                role,
                model
            );
        }
    }

    // ── upgrade_model edge cases ────────────────────────────

    #[test]
    fn upgrade_model_all_steps() {
        // haiku → sonnet → opus → opus (stays)
        assert_eq!(
            TeamSessionActor::upgrade_model(AgentRole::Tester, "claude-haiku-4-5"),
            "claude-sonnet-4-5"
        );
        assert_eq!(
            TeamSessionActor::upgrade_model(AgentRole::Tester, "claude-sonnet-4-5"),
            "claude-opus-4-5"
        );
        assert_eq!(
            TeamSessionActor::upgrade_model(AgentRole::Tester, "claude-opus-4-5"),
            "claude-opus-4-5"
        );
    }

    #[test]
    fn upgrade_model_nonexistent_model_returns_opus() {
        assert_eq!(
            TeamSessionActor::upgrade_model(AgentRole::Developer, "nonexistent-model"),
            "claude-opus-4-5"
        );
    }

    // ── auto_yes_prefix ─────────────────────────────────────

    #[test]
    fn auto_yes_prefix_contains_key_phrases() {
        let prefix = TeamSessionActor::auto_yes_prefix();
        assert!(prefix.contains("auto-yes"), "should mention auto-yes");
        assert!(
            prefix.contains("proceed automatically"),
            "should mention automatic proceed"
        );
        assert!(prefix.contains("Never ask"), "should forbid asking");
    }
}
