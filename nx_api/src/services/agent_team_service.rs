//! Agent team service
//!
//! Multi-agent orchestration for team collaboration.

use parking_lot::RwLock as ParkingRwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::broadcast;

use nexus_ai::AIModelManager;
use nx_memory::types::{MemoryChunk, MessageRole, Transcript};

use crate::models::team::{
    ExecuteRoleTaskRequest, ExecuteRoleTaskResponse, ExecuteTeamTaskRequest,
    ExecuteTeamTaskResponse, RoleSkill, SkillPriority, TeamMessage, TeamRole,
};
use crate::routes::memory::MemoryState;
use crate::services::ai_provider_service::ProviderService;
use crate::services::skill_service::SkillService;
use crate::services::team_service::TeamService;
use crate::services::telegram_service::{InboundTelegramMessage, TelegramService};

/// Strip ANSI escape codes from a string (for clean card display).
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut iter = s.chars().peekable();
    while let Some(c) = iter.next() {
        if c == '\x1b' {
            match iter.peek() {
                Some(&'[') => {
                    iter.next();
                    for ch in iter.by_ref() {
                        if ch.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
                Some(&']') => {
                    iter.next();
                    for ch in iter.by_ref() {
                        if ch == '\x07' || ch == '\x1b' {
                            break;
                        }
                    }
                }
                _ => {
                    iter.next();
                }
            }
        } else if c != '\r' {
            out.push(c);
        }
    }
    out
}

/// Run Claude CLI interactively, streaming output and supporting user confirmations.
/// When CLI needs confirmation, sends ConfirmationRequired event and waits for user response.
/// If auto_confirm is true, automatically sends 'y' without waiting.
/// Returns (pid, success, full_output).
async fn run_claude_interactive(
    args: &[&str],
    working_dir: Option<&str>,
    stream_tx: &Option<(
        broadcast::Sender<crate::ws::agent_execution::AgentExecutionEvent>,
        String,
    )>,
    confirm_rx: Option<tokio::sync::mpsc::Receiver<String>>,
    auto_confirm: bool,
    timeout_secs: u64,
) -> Result<(Option<u32>, bool, String), String> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

    let (exe_path, prefix_args) = crate::services::claude_cli::get_claude_cli_executable()
        .ok_or_else(|| "Claude CLI not found".to_string())?;
    let mut cmd = tokio::process::Command::new(&exe_path);
    for arg in &prefix_args {
        cmd.arg(arg);
    }
    cmd.args(args);
    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    cmd.kill_on_drop(true); // ensure child is killed when this future is dropped (e.g. on cancellation)
    if let Some(dir) = working_dir {
        cmd.current_dir(dir);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn Claude CLI: {}", e))?;
    let pid = child.id();

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let stdin = child.stdin.take().unwrap();

    // Clone senders
    let stderr_sender = stream_tx.as_ref().map(|(tx, id)| (tx.clone(), id.clone()));
    let stdout_sender = stream_tx.as_ref().map(|(tx, id)| (tx.clone(), id.clone()));

    // Drain stderr concurrently
    let stderr_handle = tokio::spawn(async move {
        let mut stderr_lines = tokio::io::BufReader::new(stderr).lines();
        while let Ok(Some(line)) = stderr_lines.next_line().await {
            if let Some((ref tx, ref id)) = stderr_sender {
                let clean = strip_ansi(&line);
                let trimmed = clean.trim().to_string();
                if !trimmed.is_empty() {
                    let _ = tx.send(crate::ws::agent_execution::AgentExecutionEvent::Output {
                        execution_id: id.clone(),
                        partial_output: format!("{}\n", trimmed),
                    });
                }
            }
        }
    });

    // Clone stdin for the confirmation handler
    let mut stdin_clone = stdin;
    let mut confirm_rx_opt = confirm_rx;

    let mut stdout_lines = tokio::io::BufReader::new(stdout).lines();
    let mut full_output = String::new();

    // Confirmation detection patterns
    let confirm_patterns = [
        "y/n",
        "yes/no",
        "proceed?",
        "continue?",
        "confirm?",
        "press enter to continue",
        "press enter to proceed",
        "override?",
        "skip?",
        "abort?",
        "[y/n]",
        "[yes/no]",
    ];

    let exec_id = stream_tx
        .as_ref()
        .map(|(_, id)| id.clone())
        .unwrap_or_default();

    let timed_out = tokio::time::timeout(Duration::from_secs(timeout_secs), async {
        while let Ok(Some(line)) = stdout_lines.next_line().await {
            // Stream to frontend in real-time
            if let Some((ref tx, ref id)) = stdout_sender {
                let clean = strip_ansi(&line);
                let trimmed = clean.trim().to_string();
                if !trimmed.is_empty() {
                    let _ = tx.send(crate::ws::agent_execution::AgentExecutionEvent::Output {
                        execution_id: id.clone(),
                        partial_output: format!("{}\n", trimmed),
                    });
                }
            }

            // Check for confirmation patterns (case-insensitive)
            let line_lower = line.to_lowercase();
            let needs_confirmation = confirm_patterns.iter().any(|p| line_lower.contains(p));

            if needs_confirmation && confirm_rx_opt.is_some() {
                // Auto-confirm mode: skip waiting, just send 'y'
                if auto_confirm {
                    let _ = stdin_clone.write_all(b"y\n").await;
                    let _ = stdin_clone.flush().await;
                    tracing::info!("[run_claude] Auto-confirm: sent 'y'");
                    full_output.push_str(&line);
                    full_output.push('\n');
                    continue;
                }

                // Send confirmation required event
                if let Some((ref tx, ref id)) = stdout_sender {
                    let question = strip_ansi(&line).trim().to_string();
                    let _ = tx.send(
                        crate::ws::agent_execution::AgentExecutionEvent::ConfirmationRequired {
                            execution_id: id.clone(),
                            question: question.clone(),
                            options: vec!["y".to_string(), "n".to_string()],
                            needs_input: false,
                        },
                    );
                }

                // Wait for user confirmation response (supports multiple rounds via mpsc)
                if let Some(ref mut rx) = confirm_rx_opt {
                    match tokio::time::timeout(Duration::from_secs(300), rx.recv()).await {
                        Ok(Some(response)) => {
                            // Write response to stdin
                            let response_line = format!("{}\n", response.trim());
                            if let Err(e) = stdin_clone.write_all(response_line.as_bytes()).await {
                                tracing::error!("[run_claude] Failed to write to stdin: {}", e);
                            }
                            let _ = stdin_clone.flush().await;
                            tracing::info!("[run_claude] Sent confirmation response: {}", response);
                        }
                        Ok(None) => {
                            tracing::info!("[run_claude] Confirmation channel closed, aborting");
                            break;
                        }
                        Err(_) => {
                            tracing::warn!("[run_claude] Confirmation timeout (5min), sending 'y'");
                            let _ = stdin_clone.write_all(b"y\n").await;
                            let _ = stdin_clone.flush().await;
                        }
                    }
                }
            }

            full_output.push_str(&line);
            full_output.push('\n');
        }
    })
    .await
    .is_err();

    // Kill BEFORE wait
    if timed_out {
        stderr_handle.abort();
        let _ = child.kill().await;
        let _ = child.wait().await;
        return Err(format!("Claude CLI timeout after {}s", timeout_secs));
    }

    // Process exited (stdout EOF) — give stderr task up to 2s to flush remaining lines
    let _ = tokio::time::timeout(Duration::from_secs(2), stderr_handle).await;

    let status = child
        .wait()
        .await
        .map_err(|e| format!("wait failed: {}", e))?;
    Ok((pid, status.success(), full_output.trim().to_string()))
}

/// Headless Claude CLI（stream-json），与 workflow 引擎同路径，不依赖 PTY / stdin 交互
async fn run_claude_headless(
    prompt: &str,
    working_dir: Option<&str>,
    stream_tx: &Option<(
        broadcast::Sender<crate::ws::agent_execution::AgentExecutionEvent>,
        String,
    )>,
    timeout_secs: u64,
) -> Result<(Option<u32>, bool, String), String> {
    use tokio::io::AsyncBufReadExt;
    use tokio::process::Command;

    let (exe_path, prefix_args) = crate::services::claude_cli::get_claude_cli_executable()
        .ok_or_else(|| "Claude CLI not found".to_string())?;

    let mut cmd = Command::new(&exe_path);
    for arg in &prefix_args {
        cmd.arg(arg);
    }
    for arg in crate::services::claude_cli::build_prompt_cli_args(true) {
        cmd.arg(arg);
    }
    cmd.args(["--verbose", "--output-format", "stream-json"]);
    cmd.arg(prompt);
    if let Some(dir) = working_dir {
        cmd.current_dir(dir);
    }
    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    cmd.kill_on_drop(true);

    tracing::info!(
        "[TeamCLI] spawn headless claude, cwd={:?}, prompt_len={}",
        working_dir,
        prompt.len()
    );

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn Claude CLI: {}", e))?;
    let pid = child.id();

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let stderr_sender = stream_tx.clone();

    let stderr_handle = tokio::spawn(async move {
        let mut lines = tokio::io::BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let clean = strip_ansi(&line).trim().to_string();
            if clean.is_empty() {
                continue;
            }
            if let Some((ref tx, ref id)) = stderr_sender {
                let _ = tx.send(crate::ws::agent_execution::AgentExecutionEvent::Output {
                    execution_id: id.clone(),
                    partial_output: format!("[stderr] {}\n", clean),
                });
            }
        }
    });

    let mut stdout_lines = tokio::io::BufReader::new(stdout).lines();
    let mut text_parts: Vec<String> = Vec::new();
    let mut final_result = String::new();

    let read_stdout = async {
        while let Ok(Some(line)) = stdout_lines.next_line().await {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) {
                if let Some(r) = json.get("result").and_then(|v| v.as_str()) {
                    if !r.is_empty() {
                        final_result = r.to_string();
                    }
                }
                if let Some(display) = extract_stream_json_display(&json) {
                    if let Some((ref tx, ref id)) = stream_tx {
                        let _ = tx.send(crate::ws::agent_execution::AgentExecutionEvent::Output {
                            execution_id: id.clone(),
                            partial_output: format!("{}\n", display),
                        });
                    }
                    text_parts.push(display);
                }
            } else {
                let clean = strip_ansi(trimmed).trim().to_string();
                if !clean.is_empty() {
                    if let Some((ref tx, ref id)) = stream_tx {
                        let _ = tx.send(crate::ws::agent_execution::AgentExecutionEvent::Output {
                            execution_id: id.clone(),
                            partial_output: format!("{}\n", clean),
                        });
                    }
                    text_parts.push(clean);
                }
            }
        }
    };

    let timed_out = tokio::time::timeout(Duration::from_secs(timeout_secs), read_stdout)
        .await
        .is_err();

    if timed_out {
        stderr_handle.abort();
        let _ = child.kill().await;
        let _ = child.wait().await;
        return Err(format!("Claude CLI timeout after {}s", timeout_secs));
    }

    let _ = tokio::time::timeout(Duration::from_secs(2), stderr_handle).await;
    let status = child
        .wait()
        .await
        .map_err(|e| format!("wait failed: {}", e))?;

    let text = if !final_result.is_empty() {
        final_result
    } else {
        text_parts.join("\n").trim().to_string()
    };

    tracing::info!(
        "[TeamCLI] finished success={}, output_len={}",
        status.success(),
        text.len()
    );

    Ok((pid, status.success(), text))
}

fn extract_stream_json_display(json: &serde_json::Value) -> Option<String> {
    if let Some(result) = json.get("result").and_then(|r| r.as_str()) {
        if !result.is_empty() {
            return Some(result.to_string());
        }
    }
    if let Some(content) = json.get("content").and_then(|c| c.as_str()) {
        if !content.is_empty() {
            return Some(content.to_string());
        }
    }
    if let Some(text) = json.get("text").and_then(|t| t.as_str()) {
        if !text.is_empty() {
            return Some(text.to_string());
        }
    }
    if json.get("type").and_then(|t| t.as_str()) == Some("assistant") {
        if let Some(content) = json.get("message").and_then(|m| m.get("content")) {
            if let Some(arr) = content.as_array() {
                let mut parts = Vec::new();
                for block in arr {
                    if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                        if !text.is_empty() {
                            parts.push(text.to_string());
                        }
                    }
                }
                if !parts.is_empty() {
                    return Some(parts.join(""));
                }
            }
        }
    }
    None
}

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

/// Running process info for monitoring
#[derive(Debug, Clone)]
pub struct RunningProcess {
    pub pid: Option<u32>,
    pub role_id: String,
    pub role_name: String,
    pub team_id: String,
    pub task: String,
    pub start_time: Instant,
    pub status: ProcessStatus,
    pub output: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProcessStatus {
    Running,
    Completed,
    Failed,
    Killed,
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
    /// 运行中的进程追踪器
    processes: Arc<ParkingRwLock<HashMap<String, RunningProcess>>>,
    /// 项目模块服务（用于构建项目状态摘要注入 AI prompt）
    project_module_service:
        Option<Arc<crate::services::project_module_service::ProjectModuleService>>,
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
            processes: self.processes.clone(),
            project_module_service: self.project_module_service.clone(),
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
            processes: Arc::new(ParkingRwLock::new(HashMap::new())),
            project_module_service: None,
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
            processes: Arc::new(ParkingRwLock::new(HashMap::new())),
            project_module_service: None,
        }
    }

    /// Set project module service (for injecting project state into prompts)
    pub fn set_project_module_service(
        &mut self,
        service: Arc<crate::services::project_module_service::ProjectModuleService>,
    ) {
        self.project_module_service = Some(service);
    }

    /// Build project state summary if project_id is available in context
    fn build_project_state_summary(&self, context: &HashMap<String, String>) -> String {
        let project_id = match context.get("project_id") {
            Some(id) if !id.is_empty() => id,
            _ => return String::new(),
        };
        match &self.project_module_service {
            Some(svc) => svc.build_state_summary(project_id),
            None => String::new(),
        }
    }

    /// Set memory state (for storing messages to memory in background tasks)
    pub fn set_memory_state(&self, memory_state: Arc<crate::routes::memory::MemoryState>) {
        *self.memory_state.write() = Some(memory_state);
    }

    // ==================== Process Tracking ====================

    /// Register a new running process
    pub fn register_process(
        &self,
        execution_id: &str,
        role_id: &str,
        role_name: &str,
        team_id: &str,
        task: &str,
    ) {
        let process = RunningProcess {
            pid: None,
            role_id: role_id.to_string(),
            role_name: role_name.to_string(),
            team_id: team_id.to_string(),
            task: task.to_string(),
            start_time: Instant::now(),
            status: ProcessStatus::Running,
            output: String::new(),
        };
        self.processes
            .write()
            .insert(execution_id.to_string(), process);
    }

    /// Update process PID
    pub fn set_process_pid(&self, execution_id: &str, pid: u32) {
        if let Some(p) = self.processes.write().get_mut(execution_id) {
            p.pid = Some(pid);
        }
    }

    /// Append output to process
    pub fn append_process_output(&self, execution_id: &str, output: &str) {
        if let Some(p) = self.processes.write().get_mut(execution_id) {
            p.output.push_str(output);
        }
    }

    /// Update process status
    pub fn set_process_status(&self, execution_id: &str, status: ProcessStatus) {
        if let Some(p) = self.processes.write().get_mut(execution_id) {
            p.status = status;
        }
    }

    /// Mark process as completed
    pub fn complete_process(&self, execution_id: &str, output: &str) {
        if let Some(p) = self.processes.write().get_mut(execution_id) {
            p.status = ProcessStatus::Completed;
            p.output = output.to_string();
        }
    }

    /// Get all running processes
    pub fn get_processes(&self) -> Vec<RunningProcess> {
        self.processes.read().values().cloned().collect()
    }

    /// Kill a running process by execution_id
    pub fn kill_process(&self, execution_id: &str) -> Result<(), String> {
        let mut processes = self.processes.write();
        if let Some(p) = processes.get_mut(execution_id) {
            if let Some(pid) = p.pid {
                // Try to kill the process
                #[cfg(unix)]
                {
                    use std::process::Command;
                    let _ = Command::new("kill").arg("-9").arg(pid.to_string()).output();
                }
                #[cfg(windows)]
                {
                    use std::process::Command;
                    let _ = Command::new("taskkill")
                        .args(["/F", "/PID", &pid.to_string()])
                        .output();
                }
                p.status = ProcessStatus::Killed;
            }
            Ok(())
        } else {
            Err("Process not found".to_string())
        }
    }

    /// Remove a process from tracking
    pub fn remove_process(&self, execution_id: &str) {
        self.processes.write().remove(execution_id);
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
        self.resume_telegram_polling();
    }

    /// Resume long polling for every Telegram bot that was `enabled=true` at shutdown.
    /// Without this, restarting the backend leaves polling off until the user
    /// toggles bots back on from the UI.
    fn resume_telegram_polling(&self) {
        let configs = match self.team_service.list_enabled_telegram_configs() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("[Telegram] Failed to load enabled bots on startup: {}", e);
                return;
            }
        };

        if configs.is_empty() {
            return;
        }

        tracing::info!(
            "[Telegram] Resuming long polling for {} enabled bot(s)",
            configs.len()
        );
        for cfg in configs {
            self.telegram_service
                .start_polling(cfg.role_id.clone(), cfg.bot_token.clone());
        }
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
        let processes = self.processes.clone();
        let project_module_service = self.project_module_service.clone();

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
                    processes: processes.clone(),
                    project_module_service: project_module_service.clone(),
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
    ///
    /// `stream_tx`: if Some, streams stdout lines as AgentExecutionEvent::Output events
    /// `confirm_rx`: if Some, waits for user confirmation when CLI needs input
    /// `auto_confirm`: if true, automatically confirms without waiting for user response
    pub async fn execute_team_task(
        &self,
        request: ExecuteTeamTaskRequest,
        stream_tx: Option<(
            broadcast::Sender<crate::ws::agent_execution::AgentExecutionEvent>,
            String,
        )>,
        confirm_rx: Option<tokio::sync::mpsc::Receiver<String>>,
        auto_confirm: bool,
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

        // Save user message first — always visible in chat even if CLI fails
        let user_msg = TeamMessage::user_message(team.id.clone(), request.task.clone());
        if let Err(e) = self.team_service.add_message(user_msg.clone()) {
            tracing::warn!("[TeamMsg] Failed to save user message: {}", e);
        }

        if team_with_roles.roles.is_empty() {
            tracing::warn!(
                "[Team] team {} has no roles — using generic solo dispatcher",
                request.team_id
            );
        }

        // 提取记忆上下文
        let memory_context = request
            .context
            .get("memory_context")
            .cloned()
            .unwrap_or_default();

        // Build team context for Claude
        let team_context = Self::build_team_context(&team, &team_with_roles.roles);

        // Build project state summary if project_id is in context
        let project_state = self.build_project_state_summary(&request.context);

        // Build the full prompt using shared function
        let mut full_prompt = build_team_prompt(
            &team_context,
            &memory_context,
            &request.task,
            self.current_workspace_path.read().as_deref(),
            &project_state,
        );

        // @mention 定向派活：用户显式 @了某些成员时，指示调度器把任务交给他们处理。
        if let Some(target_roles) = request.context.get("target_roles") {
            if !target_roles.trim().is_empty() {
                full_prompt = format!(
                    "## 定向派活（用户指定）\n用户通过 @ 显式指派以下成员负责本次任务：{}。\n请优先以这些成员的角色定位、技能和职责来处理，必要时由他们协作完成，不要交给未被指派的成员。\n\n{}",
                    target_roles, full_prompt
                );
            }
        }

        // 获取当前工作区路径
        let working_dir = self.current_workspace_path.read().clone();

        // Register process for monitoring
        let proc_exec_id = format!("team-{}", uuid::Uuid::new_v4());
        self.register_process(&proc_exec_id, &team.id, &team.name, &team.id, &request.task);

        // Single Claude CLI call — 600s timeout for long-running tasks
        let mut prompt_arg_storage = crate::services::claude_cli::build_prompt_cli_args(true);
        prompt_arg_storage.push(full_prompt.clone());
        let cli_arg_refs: Vec<&str> = prompt_arg_storage.iter().map(String::as_str).collect();

        crate::services::claude_cli::guard_claude_invocation(
            &full_prompt,
            working_dir.as_deref(),
            working_dir.as_deref(),
        )
        .map_err(AgentTeamServiceError::AiError)?;

        let args: Vec<&str> = if auto_confirm {
            cli_arg_refs
        } else {
            // 非 auto_confirm 时不传 skip-permissions（即使用户为 trusted，此处保留交互确认路径）
            vec!["-p", &full_prompt]
        };

        let cli_result = if auto_confirm {
            run_claude_headless(
                &full_prompt,
                working_dir.as_deref(),
                &stream_tx,
                1800,
            )
            .await
        } else {
            run_claude_interactive(
                &args,
                working_dir.as_deref(),
                &stream_tx,
                confirm_rx,
                auto_confirm,
                1800,
            )
            .await
        };
        let (pid, success, response) = match cli_result {
            Err(e) => {
                self.set_process_status(&proc_exec_id, ProcessStatus::Failed);
                return Err(AgentTeamServiceError::AiError(e));
            }
            Ok(r) => r,
        };

        if let Some(p) = pid {
            self.set_process_pid(&proc_exec_id, p);
        }

        if !success {
            self.set_process_status(&proc_exec_id, ProcessStatus::Failed);
            return Err(AgentTeamServiceError::AiError(
                "Claude CLI exited with error".to_string(),
            ));
        }
        self.complete_process(&proc_exec_id, &response);

        // Save assistant message (use first role as responder if available)
        let assistant_msg = match team_with_roles.roles.first() {
            Some(r) => TeamMessage::assistant_message(
                team.id.clone(),
                r.role.id.clone(),
                response.clone(),
            ),
            None => TeamMessage::new(
                team.id.clone(),
                None,
                response.clone(),
                crate::models::team::MessageType::Assistant,
            ),
        };
        if let Err(e) = self.team_service.add_message(assistant_msg) {
            tracing::warn!("[TeamMsg] Failed to save assistant message: {}", e);
        }

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
    pub fn build_team_context_pub(
        team: &crate::models::team::Team,
        roles: &[crate::services::team_service::RoleWithSkills],
    ) -> String {
        Self::build_team_context(team, roles)
    }

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
}

/// Build the full team dispatcher prompt (shared between PTY dispatch and fallback path).
/// This function is public so the routes layer can use it for PTY-first dispatch.
pub fn build_team_prompt(
    team_context: &str,
    memory_context: &str,
    user_task: &str,
    workspace_dir: Option<&str>,
    project_state: &str,
) -> String {
    let workspace_note = if let Some(dir) = workspace_dir {
        format!("\n\n## Current Workspace\nWorking directory: `{}`\nWhen generating documents, plans, or design files, ALWAYS save them as actual files in this directory (e.g., `{}/design.md`, `{}/architecture.md`). Use the Write tool or file creation to persist output.", dir, dir, dir)
    } else {
        String::new()
    };

    let project_section = if project_state.is_empty() {
        String::new()
    } else {
        format!("\n\n{}\n\nIMPORTANT: Do NOT repeat work that is already completed. Focus on pending or in-progress modules.", project_state)
    };

    if memory_context.is_empty() {
        format!(
            r#"You are the team dispatcher. Given the team context and user message, decide what to do.

## Team Context
{}{}
{}
## User Message
{}{}

## Your Decision
Read the user's message and the available skills in the team context.
- If a skill's trigger keywords match the user's message → use that skill
- If no skill matches → answer the user directly as a helpful AI assistant

## Output Format
Return your response directly. If using a skill, invoke it according to its execution instructions.
IMPORTANT: When asked to generate documents, reports, or design files, write them as real files to the current workspace directory — do not just print them as text."#,
            team_context, project_section, memory_context, user_task, workspace_note
        )
    } else {
        format!(
            r#"You are the team dispatcher. Given the team context and user message, decide what to do.

## Team Context
{}{}
{}
## User Message
{}{}

## Your Decision
Read the user's message and the available skills in the team context.
- If a skill's trigger keywords match the user's message → use that skill
- If no skill matches → answer the user directly as a helpful AI assistant

## Output Format
Return your response directly. If using a skill, invoke it according to its execution instructions.
IMPORTANT: When asked to generate documents, reports, or design files, write them as real files to the current workspace directory — do not just print them as text."#,
            team_context, project_section, memory_context, user_task, workspace_note
        )
    }
}

impl AgentTeamService {
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
    /// `stream_tx`: if Some, streams stdout lines as AgentExecutionEvent::Output events
    pub async fn execute_role_task(
        &self,
        request: ExecuteRoleTaskRequest,
        stream_tx: Option<(
            broadcast::Sender<crate::ws::agent_execution::AgentExecutionEvent>,
            String,
        )>,
    ) -> Result<ExecuteRoleTaskResponse, AgentTeamServiceError> {
        tracing::info!(
            "[AgentTeamService] execute_role_task called, role_id: {}",
            request.role_id
        );

        // Load role with skills
        let role_with_skills = self
            .team_service
            .get_role_with_skills(&request.role_id)
            .map_err(|e| AgentTeamServiceError::RoleNotFound(e.to_string()))?;

        // Register process for monitoring
        let execution_id = role_with_skills.role.id.clone();
        let team_id = role_with_skills.role.team_id.clone().unwrap_or_default();
        self.register_process(
            &execution_id,
            &request.role_id,
            &role_with_skills.role.name,
            &team_id,
            &request.task,
        );
        tracing::info!("[AgentTeamService-3] Got role_with_skills");

        let role = role_with_skills.role.clone();

        // Save user message immediately
        let user_msg = TeamMessage::user_message(team_id.clone(), request.task.clone());
        if let Err(e) = self.team_service.add_message(user_msg) {
            tracing::warn!("[TeamMsg] Failed to save user message (role task): {}", e);
        }

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
        let memory_context = request
            .context
            .get("memory_context")
            .cloned()
            .unwrap_or_default();
        tracing::info!(
            "[AgentTeam] role_id: {}, task: {}, memory_context length: {}",
            request.role_id,
            request.task,
            memory_context.len()
        );
        tracing::debug!("[AgentTeam] memory_context: {}", memory_context);

        // 获取当前工作区路径
        let working_dir = self.current_workspace_path.read().clone();
        tracing::info!(
            "[AgentTeamService] Working dir: {:?}, spawning Claude CLI...",
            working_dir
        );

        // Build the full prompt
        let workspace_note = if let Some(ref dir) = working_dir {
            format!("\n\nIMPORTANT: Working directory is `{}`. When generating documents, plans, reports, or design files, ALWAYS save them as actual files in this directory. Use file creation tools to persist output — do not just print content as text.", dir)
        } else {
            String::new()
        };

        let full_prompt = if memory_context.is_empty() {
            format!(
                "{}\n\n<system>\n{}{}\n</system>\n\n<user>\n{}\n</user>",
                auto_yes_prefix, prompt, workspace_note, request.task
            )
        } else {
            format!(
                "{}\n\n<system>\n{}\n\n{}{}\n</system>\n\n<user>\n{}\n</user>",
                auto_yes_prefix, prompt, memory_context, workspace_note, request.task
            )
        };

        tracing::info!(
            "[AgentTeam] Full prompt length: {}, memory_context included: {}",
            full_prompt.len(),
            !memory_context.is_empty()
        );

        crate::services::claude_cli::guard_claude_invocation(
            &full_prompt,
            working_dir.as_deref(),
            working_dir.as_deref(),
        )
        .map_err(AgentTeamServiceError::AiError)?;

        let mut prompt_arg_storage =
            crate::services::claude_cli::build_prompt_cli_args(true);
        prompt_arg_storage.push(full_prompt.clone());
        let cli_args: Vec<&str> = prompt_arg_storage.iter().map(String::as_str).collect();

        let pty_result = run_claude_interactive(
            &cli_args,
            working_dir.as_deref(),
            &stream_tx,
            None,  // execute_role_task doesn't support confirmations yet
            false, // auto_confirm
            1800,  // 30 min — complex coding tasks can take 25+ min
        )
        .await;

        let (pid, success, response) = match pty_result {
            Err(e) => {
                self.set_process_status(&execution_id, ProcessStatus::Failed);
                return Err(AgentTeamServiceError::AiError(e));
            }
            Ok(r) => r,
        };

        if let Some(p) = pid {
            self.set_process_pid(&execution_id, p);
            tracing::info!("[AgentTeam] Process PID: {}", p);
        }

        if !success {
            self.set_process_status(&execution_id, ProcessStatus::Failed);
            return Err(AgentTeamServiceError::AiError(
                "Claude CLI exited with error".to_string(),
            ));
        }

        tracing::info!("[AgentTeam] Claude CLI success");
        self.complete_process(&execution_id, &response);

        // Save assistant message
        let assistant_msg =
            TeamMessage::assistant_message(team_id.clone(), role.id.clone(), response.clone());
        if let Err(e) = self.team_service.add_message(assistant_msg) {
            tracing::warn!(
                "[TeamMsg] Failed to save assistant message (role task): {}",
                e
            );
        }

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
            let context = format!(
                "[{}] skill: {}",
                priority_str.to_uppercase(),
                skill.skill_id
            );
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
        let skill_contexts = self.build_skill_contexts(&role_with_skills.skills).await;

        // Build system prompt
        let system_prompt = self.build_system_prompt(role, &skill_contexts).await;

        // Execute AI
        let response = self
            .execute_role_ai(role, &system_prompt, &message.text)
            .await
            .map_err(|e| AgentTeamServiceError::AiError(e.to_string()))?;

        // Save conversation with telegram metadata
        let team_id_for_msg = role.team_id.clone().unwrap_or_default();
        let mut user_msg = TeamMessage::user_message(team_id_for_msg.clone(), message.text.clone());
        user_msg
            .metadata
            .insert("source".to_string(), "telegram".to_string());
        user_msg
            .metadata
            .insert("chat_id".to_string(), message.chat_id.to_string());
        user_msg
            .metadata
            .insert("role_id".to_string(), message.role_id.clone());
        if let Some(mid) = message.message_id {
            user_msg
                .metadata
                .insert("message_id".to_string(), mid.to_string());
        }
        if let Err(e) = self.team_service.add_message(user_msg) {
            tracing::warn!("[TeamMsg] Failed to save user message (telegram): {}", e);
        }

        let mut assistant_msg =
            TeamMessage::assistant_message(team_id_for_msg, role.id.clone(), response.clone());
        assistant_msg
            .metadata
            .insert("source".to_string(), "telegram".to_string());
        assistant_msg
            .metadata
            .insert("chat_id".to_string(), message.chat_id.to_string());
        if let Err(e) = self.team_service.add_message(assistant_msg) {
            tracing::warn!(
                "[TeamMsg] Failed to save assistant message (telegram): {}",
                e
            );
        }

        // Send response via Telegram (reply to the original message in groups)
        if let Some(chat_id) = &config.chat_id {
            self.telegram_service
                .send_message_with_reply(&config.bot_token, chat_id, &response, message.message_id)
                .await
                .map_err(|e| AgentTeamServiceError::Telegram(e.to_string()))?;
        } else {
            // No configured chat_id — reply to the chat the message came from
            self.telegram_service
                .send_message_with_reply(
                    &config.bot_token,
                    &message.chat_id.to_string(),
                    &response,
                    message.message_id,
                )
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
    async fn build_system_prompt(&self, role: &TeamRole, skill_contexts: &[String]) -> String {
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

        crate::services::claude_cli::guard_claude_invocation(
            &full_prompt,
            working_dir.as_deref(),
            working_dir.as_deref(),
        )
        .map_err(|e| AgentTeamServiceError::AiError(e))?;

        let (exe_path, prefix_args) = crate::services::claude_cli::get_claude_cli_executable()
            .ok_or_else(|| AgentTeamServiceError::AiError("Claude CLI not found".to_string()))?;
        let mut cmd = tokio::process::Command::new(&exe_path);
        for arg in &prefix_args {
            cmd.arg(arg);
        }
        let mut prompt_args = crate::services::claude_cli::build_prompt_cli_args(false);
        for arg in &prompt_args {
            cmd.arg(arg);
        }
        cmd.arg(&full_prompt);
        cmd.kill_on_drop(true);
        if let Some(ref dir) = working_dir {
            cmd.current_dir(dir);
            tracing::info!(
                "[AgentTeam] execute_role_ai 执行 Claude CLI，当前目录: {}",
                dir
            );
        }
        let output = cmd.output().await.map_err(|e| {
            AgentTeamServiceError::AiError(format!("Failed to execute Claude CLI: {}", e))
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let detail = if !stderr.is_empty() {
                stderr
            } else if !stdout.is_empty() {
                stdout
            } else {
                "no stderr/stdout captured".to_string()
            };
            return Err(AgentTeamServiceError::AiError(format!(
                "Claude CLI error ({}): {}",
                output.status, detail
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
            if let Ok(config) = self
                .team_service
                .get_telegram_config(&role_with_skills.role.id)
            {
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
