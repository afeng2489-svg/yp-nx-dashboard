//! 执行服务

use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::RwLock;

use nexus_ai::{
    AIManagerConfig as NexusAIManagerConfig, AIModelManager, AIProviderRegistry,
    APIConfig as NexusAPIConfig, ModelConfig, ProviderType,
};
use nexus_workflow::{InMemoryEventEmitter, TriggerType, WorkflowDefinition, WorkflowEngine};

pub use crate::services::events::{ExecutionEvent, ExecutionStatus};

/// 从环境变量加载 AI 配置
fn load_ai_config_from_env() -> NexusAIManagerConfig {
    let mut api_config = HashMap::new();

    // 加载 Anthropic API 配置
    if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
        if !api_key.is_empty() {
            api_config.insert(
                ProviderType::Anthropic,
                NexusAPIConfig {
                    api_key,
                    base_url: String::new(),
                    organization_id: String::new(),
                    timeout_secs: 120,
                },
            );
        }
    }

    // 加载 OpenAI API 配置
    if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
        if !api_key.is_empty() {
            api_config.insert(
                ProviderType::OpenAI,
                NexusAPIConfig {
                    api_key,
                    base_url: String::new(),
                    organization_id: String::new(),
                    timeout_secs: 120,
                },
            );
        }
    }

    // 加载 Google API 配置
    if let Ok(api_key) = std::env::var("GOOGLE_API_KEY") {
        if !api_key.is_empty() {
            api_config.insert(
                ProviderType::Google,
                NexusAPIConfig {
                    api_key,
                    base_url: String::new(),
                    organization_id: String::new(),
                    timeout_secs: 120,
                },
            );
        }
    }

    // 加载默认模型
    let default_model = if let Ok(model_id) = std::env::var("NEXUS_DEFAULT_MODEL") {
        ModelConfig {
            model_id,
            provider: ProviderType::Anthropic,
            max_tokens: 4096,
            temperature: 0.7,
            stop_sequences: vec![],
            extra_params: HashMap::new(),
        }
    } else {
        ModelConfig::default()
    };

    NexusAIManagerConfig {
        default_model,
        api_config,
        enabled_providers: vec![
            ProviderType::Anthropic,
            ProviderType::OpenAI,
            ProviderType::Google,
            ProviderType::Ollama,
            ProviderType::Codex,
            ProviderType::Qwen,
            ProviderType::OpenCode,
            ProviderType::MiniMax,
            ProviderType::ClaudeCli,
        ],
        default_escalate_model: None,
    }
}
use crate::services::execution_bridge::WorkflowEventBridge;
use crate::services::execution_repository::SqliteExecutionRepository;
use crate::services::model_router::{default_rules, ModelRouter, TaskContext};

/// 工厂台 / 调度等启动上下文（写入 execution 记录）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionLaunchContext {
    pub team_id: Option<String>,
    pub project_id: Option<String>,
    pub trigger_source: Option<String>,
}

/// 链式触发回调：接收下游工作流名和变量，返回下游 execution_id
pub type ChainTriggerCallback = Arc<
    dyn Fn(
            String,
            serde_json::Value,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<String, String>> + Send + Sync>,
        > + Send
        + Sync,
>;

/// 执行服务
#[derive(Clone)]
pub struct ExecutionService {
    executions: Arc<Mutex<HashMap<String, Execution>>>,
    event_sender: broadcast::Sender<ExecutionEvent>,
    /// user_input pause/resume channel 注册表（execution_id → sender）
    resume_channels: Arc<Mutex<HashMap<String, tokio::sync::mpsc::Sender<String>>>>,
    /// stage 观察者列表（产物追踪、token 监控等）
    stage_watchers: Arc<Mutex<Vec<Arc<dyn nexus_workflow::watcher::StageWatcher>>>>,
    /// RAG provider（可选，注入后 engine 在 stage 执行前自动检索知识）
    rag_provider: Arc<Mutex<Option<Arc<dyn nexus_workflow::watcher::RagProvider>>>>,
    /// 持久化仓储（重启后历史记录不丢失）
    repo: Option<Arc<SqliteExecutionRepository>>,
    /// 链式触发回调（工作流完成时触发下游）
    chain_trigger_handler: Option<ChainTriggerCallback>,
    /// 模型自动路由器
    model_router: Arc<Mutex<ModelRouter>>,
    /// A2UI 服务（可选，注入后 WorkflowPaused 事件自动写入消息）
    a2ui_service: Option<Arc<crate::a2ui::A2UIService>>,
    /// 产物追踪 watcher（单独保存以便注册 engine_id → api_id 映射）
    artifact_watcher: Option<Arc<crate::services::artifact_watcher::ArtifactStageWatcher>>,
    /// 运行中工作流的取消令牌（execution_id → token）
    cancel_tokens: Arc<Mutex<HashMap<String, tokio_util::sync::CancellationToken>>>,
    /// 与 AppState 共享的 AI 管理器（含设置页 API Key / Claude CLI）
    ai_manager: Arc<Mutex<Option<Arc<AIModelManager>>>>,
}

impl std::fmt::Debug for ExecutionService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecutionService").finish()
    }
}

impl ExecutionService {
    /// 创建新的执行服务（无持久化）
    pub fn new() -> Self {
        let (event_sender, _) = broadcast::channel(1000);
        Self {
            executions: Arc::new(Mutex::new(HashMap::new())),
            event_sender,
            resume_channels: Arc::new(Mutex::new(HashMap::new())),
            stage_watchers: Arc::new(Mutex::new(Vec::new())),
            rag_provider: Arc::new(Mutex::new(None)),
            repo: None,
            chain_trigger_handler: None,
            model_router: Arc::new(Mutex::new(ModelRouter::new(default_rules()))),
            a2ui_service: None,
            artifact_watcher: None,
            cancel_tokens: Arc::new(Mutex::new(HashMap::new())),
            ai_manager: Arc::new(Mutex::new(None)),
        }
    }

    pub fn with_repository(repo: Arc<SqliteExecutionRepository>) -> Self {
        let (event_sender, _) = broadcast::channel(1000);
        Self {
            executions: Arc::new(Mutex::new(HashMap::new())),
            event_sender,
            resume_channels: Arc::new(Mutex::new(HashMap::new())),
            stage_watchers: Arc::new(Mutex::new(Vec::new())),
            rag_provider: Arc::new(Mutex::new(None)),
            repo: Some(repo),
            chain_trigger_handler: None,
            model_router: Arc::new(Mutex::new(ModelRouter::new(default_rules()))),
            a2ui_service: None,
            artifact_watcher: None,
            cancel_tokens: Arc::new(Mutex::new(HashMap::new())),
            ai_manager: Arc::new(Mutex::new(None)),
        }
    }

    /// 注入与 AppState 共享的 AIModelManager（设置页 API Key 变更会同步生效）
    pub fn set_ai_manager(&mut self, manager: Arc<AIModelManager>) {
        *self.ai_manager.lock() = Some(manager);
    }
    pub fn set_artifact_watcher(
        &mut self,
        watcher: Arc<crate::services::artifact_watcher::ArtifactStageWatcher>,
    ) {
        self.stage_watchers.lock().push(watcher.clone());
        self.artifact_watcher = Some(watcher);
    }

    /// 注册引擎内部 UUID → API exec_id 映射（供产物追踪使用）
    pub fn register_engine_id(&self, engine_id: &str, api_id: &str) {
        if let Some(ref watcher) = self.artifact_watcher {
            watcher.register_id(engine_id, api_id);
            if let Some(exec) = self.get_execution(api_id) {
                if let Some(ref path) = exec.workspace_path {
                    watcher.register_workdir(api_id, path);
                }
            }
        }
    }

    /// 注入 A2UI 服务
    pub fn set_a2ui(&mut self, service: Arc<crate::a2ui::A2UIService>) {
        self.a2ui_service = Some(service);
    }

    /// 注册一个 stage 观察者（启动期调用一次即可，运行期共享）
    pub fn add_stage_watcher(&self, watcher: Arc<dyn nexus_workflow::watcher::StageWatcher>) {
        self.stage_watchers.lock().push(watcher);
    }

    /// 注入 RAG provider（启动期调用一次）
    pub fn set_rag_provider(&self, provider: Arc<dyn nexus_workflow::watcher::RagProvider>) {
        *self.rag_provider.lock() = Some(provider);
    }

    /// 获取路由规则（供 API 读取）
    pub fn get_routing_rules(&self) -> Vec<crate::services::model_router::RoutingRule> {
        self.model_router.lock().rules().to_vec()
    }

    /// 替换路由规则（供 API 写入）
    pub fn set_routing_rules(&self, rules: Vec<crate::services::model_router::RoutingRule>) {
        self.model_router.lock().set_rules(rules);
    }

    /// 注册链式触发回调
    pub fn set_chain_trigger_handler(&mut self, handler: ChainTriggerCallback) {
        self.chain_trigger_handler = Some(handler);
    }

    /// 订阅执行事件
    pub fn subscribe(&self) -> broadcast::Receiver<ExecutionEvent> {
        self.event_sender.subscribe()
    }

    /// 广播事件，并将状态变更持久化到 Execution（供新 WS 连接 catch-up）
    pub fn broadcast(&self, event: ExecutionEvent) {
        match &event {
            ExecutionEvent::Output { execution_id, line } => {
                let mut executions = self.executions.lock();
                if let Some(ex) = executions.get_mut(execution_id.as_str()) {
                    ex.output_log.push(line.clone());
                    // 最多保留 500 行，超出时丢弃最旧的
                    if ex.output_log.len() > 500 {
                        let excess = ex.output_log.len() - 500;
                        ex.output_log.drain(0..excess);
                    }
                }
            }
            ExecutionEvent::StageStarted {
                execution_id,
                stage_name,
            } => {
                let mut executions = self.executions.lock();
                if let Some(ex) = executions.get_mut(execution_id.as_str()) {
                    ex.current_stage = Some(stage_name.clone());
                }
            }
            ExecutionEvent::Completed { execution_id } => {
                let mut executions = self.executions.lock();
                if let Some(ex) = executions.get_mut(execution_id.as_str()) {
                    ex.current_stage = None;
                }
            }
            ExecutionEvent::Failed { execution_id, .. } => {
                let mut executions = self.executions.lock();
                if let Some(ex) = executions.get_mut(execution_id.as_str()) {
                    ex.current_stage = None;
                }
            }
            ExecutionEvent::WorkflowPaused {
                execution_id,
                stage_name,
                question,
                options,
                pause_kind,
            } => {
                let mut executions = self.executions.lock();
                if let Some(ex) = executions.get_mut(execution_id.as_str()) {
                    ex.status = ExecutionStatus::Paused;
                    ex.pending_pause = Some(PendingPause {
                        stage_name: stage_name.clone(),
                        question: question.clone(),
                        options: options.clone(),
                        pause_kind: pause_kind.clone(),
                    });
                }
                drop(executions);
                if let Some(ref repo) = self.repo {
                    let _ = repo.update_status(
                        execution_id,
                        status_to_db_str(ExecutionStatus::Paused),
                        None,
                        None,
                    );
                }
                if let Some(a2ui) = &self.a2ui_service {
                    let session = a2ui.get_or_create_session(execution_id.as_str());
                    let msg = crate::a2ui::InteractiveMessage {
                        id: uuid::Uuid::new_v4().to_string(),
                        session_id: session.id.clone(),
                        execution_id: execution_id.clone(),
                        content: if options.is_empty() {
                            crate::a2ui::A2UIMessage::Ask {
                                question: question.clone(),
                                context: Some(stage_name.clone()),
                            }
                        } else {
                            crate::a2ui::A2UIMessage::Select {
                                prompt: question.clone(),
                                options: options.iter().map(|o| o.label.clone()).collect(),
                            }
                        },
                        source: "workflow".to_string(),
                        timestamp: chrono::Utc::now(),
                        pending: true,
                        response: None,
                        responded_at: None,
                    };
                    let _ = a2ui.add_message(&session.id, msg);
                }
            }
            ExecutionEvent::WorkflowResumed { execution_id, .. } => {
                let mut executions = self.executions.lock();
                if let Some(ex) = executions.get_mut(execution_id.as_str()) {
                    ex.pending_pause = None;
                    if ex.status == ExecutionStatus::Paused {
                        ex.status = ExecutionStatus::Running;
                    }
                }
                drop(executions);
                if let Some(ref repo) = self.repo {
                    let _ = repo.update_status(
                        execution_id,
                        status_to_db_str(ExecutionStatus::Running),
                        None,
                        None,
                    );
                }
            }
            _ => {}
        }
        let _ = self.event_sender.send(event);
    }

    /// 启动新执行
    pub fn start_execution(
        &self,
        workflow_id: String,
        variables: serde_json::Value,
        workspace_path: Option<String>,
        launch: Option<ExecutionLaunchContext>,
    ) -> Execution {
        let mut execution = Execution::new(workflow_id.clone(), variables);
        execution.workspace_path = workspace_path;
        if let Some(ctx) = launch {
            execution.team_id = ctx.team_id;
            execution.project_id = ctx.project_id;
            execution.trigger_source = ctx.trigger_source;
        }
        execution.start(); // 设置为 Running 状态

        let exec_clone = execution.clone();
        let mut executions = self.executions.lock();
        executions.insert(execution.id.clone(), execution);
        drop(executions);

        // 写入数据库
        if let Some(ref repo) = self.repo {
            if let Err(e) = repo.insert(&exec_clone) {
                tracing::error!("持久化执行记录失败: {}", e);
            }
        }

        if let Some(ref path) = exec_clone.workspace_path {
            if let Some(ref watcher) = self.artifact_watcher {
                watcher.register_workdir(&exec_clone.id, path);
            }
        }

        // 广播事件
        self.broadcast(ExecutionEvent::Started {
            execution_id: exec_clone.id.clone(),
            workflow_id: workflow_id.clone(),
        });
        self.broadcast(ExecutionEvent::StatusChanged {
            execution_id: exec_clone.id.clone(),
            status: ExecutionStatus::Running,
        });

        exec_clone
    }

    /// 重试 Run：继承父 Run 已完成阶段、审批记录与变量链（AF-UX-09）
    pub fn seed_retry_lineage(
        &self,
        child_id: &str,
        parent_id: &str,
        stage_order: &[String],
        from_stage: Option<&str>,
    ) -> Result<(), String> {
        let parent = self
            .get_execution(parent_id)
            .ok_or_else(|| format!("父执行 {parent_id} 不存在"))?;

        let from_idx = from_stage
            .and_then(|s| stage_order.iter().position(|n| n == s))
            .unwrap_or(stage_order.len());

        let inherited: Vec<StageResult> = if from_idx == 0 {
            Vec::new()
        } else {
            stage_order
                .iter()
                .take(from_idx)
                .filter_map(|name| {
                    parent
                        .stage_results
                        .iter()
                        .find(|sr| sr.stage_name == *name)
                        .cloned()
                })
                .collect()
        };

        let mut child = self
            .get_execution(child_id)
            .ok_or_else(|| format!("子执行 {child_id} 不存在"))?;

        child.resumed_from = Some(parent_id.to_string());
        child.stage_results = inherited.clone();
        child.approval_events = parent.approval_events.clone();
        if let Some(obj) = child.variables.as_object_mut() {
            obj.insert(
                "resumed_from_execution_id".into(),
                serde_json::json!(parent_id),
            );
        }

        {
            let mut executions = self.executions.lock();
            if let Some(ex) = executions.get_mut(child_id) {
                ex.resumed_from = child.resumed_from.clone();
                ex.stage_results = child.stage_results.clone();
                ex.approval_events = child.approval_events.clone();
                ex.variables = child.variables.clone();
            }
        }

        if let Some(ref repo) = self.repo {
            for sr in &inherited {
                if let Err(e) = repo.insert_stage_result(child_id, sr) {
                    tracing::warn!("继承阶段结果失败: {}", e);
                }
            }
            if let Err(e) = repo.update_approval_events(child_id, &child.approval_events) {
                tracing::warn!("继承审批记录失败: {}", e);
            }
            if let Err(e) = repo.update_variables(child_id, &child.variables) {
                tracing::warn!("更新重试 variables 失败: {}", e);
            }
        }

        self.broadcast(ExecutionEvent::StatusChanged {
            execution_id: child_id.to_string(),
            status: ExecutionStatus::Running,
        });

        Ok(())
    }

    /// 获取执行状态（优先查内存，再查 DB）
    pub fn get_execution(&self, id: &str) -> Option<Execution> {
        let mut exec = {
            let executions = self.executions.lock();
            if let Some(exec) = executions.get(id) {
                Some(exec.clone())
            } else {
                None
            }
        };
        if exec.is_none() {
            exec = self
                .repo
                .as_ref()
                .and_then(|repo| repo.find_by_id(id).ok())
                .flatten();
        }
        exec.map(|mut e| {
            Self::hydrate_resumed_from(&mut e);
            e
        })
    }

    fn hydrate_resumed_from(exec: &mut Execution) {
        if exec.resumed_from.is_none() {
            exec.resumed_from = exec
                .variables
                .get("resumed_from_execution_id")
                .and_then(|v| v.as_str())
                .map(String::from);
        }
    }

    /// 获取所有执行（合并 DB 历史 + 内存中的活跃记录）
    pub fn get_all_executions(&self) -> Vec<Execution> {
        let mut all: Vec<Execution> = if let Some(ref repo) = self.repo {
            repo.find_all().unwrap_or_default()
        } else {
            Vec::new()
        };

        // 合并内存中的记录：DB 中没有的追加，DB 中有的用内存版本覆盖（状态更新）
        let executions = self.executions.lock();
        for (id, exec) in executions.iter() {
            if let Some(existing) = all.iter_mut().find(|e| e.id == *id) {
                // 用内存中更新的状态覆盖 DB 记录
                existing.status = exec.status;
                existing.error = exec.error.clone();
                existing.stage_results = exec.stage_results.clone();
                existing.started_at = exec.started_at;
                existing.finished_at = exec.finished_at;
                existing.output_log = exec.output_log.clone();
                existing.current_stage = exec.current_stage.clone();
                existing.running_agents = exec.running_agents.clone();
                existing.pending_pause = exec.pending_pause.clone();
                existing.approval_events = exec.approval_events.clone();
                existing.resumed_from = exec.resumed_from.clone();
            } else {
                all.push(exec.clone());
            }
        }
        drop(executions);
        for e in all.iter_mut() {
            Self::hydrate_resumed_from(e);
        }
        // 按 started_at 降序排列
        all.sort_by_key(|b| std::cmp::Reverse(b.started_at));
        all
    }

    /// 从内存中移除执行记录
    pub fn remove_from_memory(&self, id: &str) {
        self.executions.lock().remove(id);
    }

    /// 从数据库删除执行记录
    pub fn delete_from_db(&self, id: &str) -> Option<bool> {
        self.repo
            .as_ref()
            .map(|repo| repo.delete(id).unwrap_or(false))
    }

    /// 批量从数据库删除执行记录
    pub fn delete_many_from_db(&self, ids: &[String]) -> Option<usize> {
        self.repo
            .as_ref()
            .map(|repo| repo.delete_many(ids).unwrap_or(0))
    }

    /// 取消执行
    pub fn cancel_execution(&self, id: &str) -> bool {
        if let Some(token) = self.cancel_tokens.lock().remove(id) {
            token.cancel();
        }

        let mut executions = self.executions.lock();
        if let Some(execution) = executions.get_mut(id) {
            execution.cancel();
            let status = execution.status;
            let status_str = status_to_db_str(status);
            let finished_at = execution.finished_at.map(|t| t.to_rfc3339());
            drop(executions);

            // 同步到数据库
            if let Some(ref repo) = self.repo {
                if let Err(e) = repo.update_status(id, status_str, None, finished_at.as_deref()) {
                    tracing::error!("持久化取消状态失败: {}", e);
                }
            }

            self.broadcast(ExecutionEvent::StatusChanged {
                execution_id: id.to_string(),
                status,
            });
            return true;
        }
        drop(executions);

        // 内存中无记录（如 API 重启后的僵尸 Run）：直接更新数据库
        if let Some(ref repo) = self.repo {
            if let Ok(Some(exec)) = repo.find_by_id(id) {
                if matches!(
                    exec.status,
                    ExecutionStatus::Running | ExecutionStatus::Paused | ExecutionStatus::Pending
                ) {
                    let finished_at = chrono::Utc::now().to_rfc3339();
                    if repo
                        .update_status(id, "cancelled", None, Some(&finished_at))
                        .is_ok()
                    {
                        self.broadcast(ExecutionEvent::StatusChanged {
                            execution_id: id.to_string(),
                            status: ExecutionStatus::Cancelled,
                        });
                        return true;
                    }
                }
            }
        }

        false
    }

    /// 更新执行状态
    pub fn update_status(&self, id: &str, status: ExecutionStatus) {
        let mut executions = self.executions.lock();
        if let Some(execution) = executions.get_mut(id) {
            execution.status = status;
            // 终态时设置 finished_at
            if matches!(
                status,
                ExecutionStatus::Completed
                    | ExecutionStatus::Failed
                    | ExecutionStatus::Cancelled
            ) {
                execution.finished_at = Some(chrono::Utc::now());
            }
            let status_str = status_to_db_str(status);
            let error = execution.error.clone();
            let finished_at = execution.finished_at.map(|t| t.to_rfc3339());
            let exec_id = execution.id.clone();
            drop(executions);

            // 同步到数据库
            if let Some(ref repo) = self.repo {
                if let Err(e) = repo.update_status(
                    &exec_id,
                    status_str,
                    error.as_deref(),
                    finished_at.as_deref(),
                ) {
                    tracing::error!("持久化状态更新失败: {}", e);
                }
            }

            self.broadcast(ExecutionEvent::StatusChanged {
                execution_id: exec_id,
                status,
            });
        }
    }

    /// 设置执行错误
    pub fn set_error(&self, id: &str, error: String) {
        let mut executions = self.executions.lock();
        if let Some(execution) = executions.get_mut(id) {
            execution.error = Some(error);
        }
    }

    /// 添加阶段输出
    pub fn add_stage_output(&self, id: &str, stage_name: String, output: serde_json::Value) {
        self.add_stage_output_with_gate(id, stage_name, output, None);
    }

    /// 添加阶段输出（带质量门结果）
    pub fn add_stage_output_with_gate(
        &self,
        id: &str,
        stage_name: String,
        output: serde_json::Value,
        quality_gate_result: Option<serde_json::Value>,
    ) {
        let mut executions = self.executions.lock();
        if let Some(execution) = executions.get_mut(id) {
            let sr = StageResult {
                stage_name: stage_name.clone(),
                outputs: vec![output.clone()],
                completed_at: Some(chrono::Utc::now()),
                quality_gate_result: quality_gate_result.clone(),
            };
            execution.stage_results.push(sr.clone());
            let exec_id = execution.id.clone();
            drop(executions);

            // 同步阶段结果到数据库
            if let Some(ref repo) = self.repo {
                if let Err(e) = repo.insert_stage_result(&exec_id, &sr) {
                    tracing::error!("持久化阶段结果失败: {}", e);
                }
            }

            self.broadcast(ExecutionEvent::StageCompleted {
                execution_id: exec_id,
                stage_name,
                output,
                quality_gate_result,
            });
        }
    }

    /// 添加输出行
    pub fn add_output_line(&self, id: &str, line: String) {
        let executions = self.executions.lock();
        if let Some(execution) = executions.get(id) {
            let exec_id = execution.id.clone();
            drop(executions);
            self.broadcast(ExecutionEvent::Output {
                execution_id: exec_id,
                line,
            });
        }
    }

    /// 累加 token 消耗和费用，并检查预算
    pub fn add_token_usage(&self, id: &str, tokens: i64, cost_usd: f64) {
        self.add_token_usage_with_budget(id, tokens, cost_usd, None);
    }

    /// 累加 token 消耗和费用，带预算检查
    pub fn add_token_usage_with_budget(
        &self,
        id: &str,
        tokens: i64,
        cost_usd: f64,
        budget_limit_usd: Option<f64>,
    ) {
        let mut executions = self.executions.lock();
        if let Some(execution) = executions.get_mut(id) {
            execution.total_tokens += tokens;
            execution.total_cost_usd += cost_usd;
            let exec_id = execution.id.clone();
            let total_tokens = execution.total_tokens;
            let total_cost_usd = execution.total_cost_usd;
            drop(executions);

            if let Some(ref repo) = self.repo {
                if let Err(e) = repo.update_token_usage(&exec_id, total_tokens, total_cost_usd) {
                    tracing::error!("持久化 token 用量失败: {}", e);
                }
            }

            // 预算检查
            if let Some(limit) = budget_limit_usd {
                if limit > 0.0 {
                    let percentage = total_cost_usd / limit * 100.0;
                    if total_cost_usd > limit {
                        tracing::warn!(
                            "[Budget] 执行 {} 超预算: ${:.4} > ${:.4}",
                            exec_id,
                            total_cost_usd,
                            limit
                        );
                        self.broadcast(ExecutionEvent::BudgetExceeded {
                            execution_id: exec_id.clone(),
                            current_usd: total_cost_usd,
                            limit_usd: limit,
                        });
                        self.cancel_execution(&exec_id);
                    } else if percentage >= 80.0 {
                        tracing::warn!(
                            "[Budget] 执行 {} 接近预算上限: ${:.4}/${:.4} ({:.0}%)",
                            exec_id,
                            total_cost_usd,
                            limit,
                            percentage
                        );
                        self.broadcast(ExecutionEvent::BudgetWarning {
                            execution_id: exec_id,
                            current_usd: total_cost_usd,
                            limit_usd: limit,
                            percentage,
                        });
                    }
                }
            }
        }
    }

    /// 恢复暂停中的执行（user_input stage）
    pub fn resume_execution(&self, execution_id: &str, value: String) -> bool {
        let channels = self.resume_channels.lock();
        if let Some(tx) = channels.get(execution_id) {
            tx.try_send(value).is_ok()
        } else {
            false
        }
    }

    /// 审批 resolve：记录 audit + resume 引擎
    pub fn resolve_execution(
        &self,
        execution_id: &str,
        approved: bool,
        comment: Option<String>,
    ) -> Result<(), String> {
        let pause = {
            let executions = self.executions.lock();
            let ex = executions
                .get(execution_id)
                .ok_or_else(|| "执行不存在".to_string())?;
            ex.pending_pause
                .clone()
                .ok_or_else(|| "当前无待审批暂停".to_string())?
        };

        let trimmed_comment = comment.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        let value = if approved {
            "approved".to_string()
        } else if let Some(ref c) = trimmed_comment {
            format!("rejected:{}", c)
        } else {
            "rejected".to_string()
        };

        {
            let mut executions = self.executions.lock();
            if let Some(ex) = executions.get_mut(execution_id) {
                ex.approval_events.push(ApprovalEvent {
                    stage_name: pause.stage_name.clone(),
                    approved,
                    comment: trimmed_comment.clone(),
                    decided_at: Utc::now(),
                });
            }
        }

        if let Some(ref repo) = self.repo {
            if let Some(ex) = self.get_execution(execution_id) {
                let _ = repo.update_approval_events(execution_id, &ex.approval_events);
            }
        }

        if !self.resume_execution(execution_id, value.clone()) {
            return Err("无法恢复执行（resume channel 已关闭）".to_string());
        }

        {
            let mut executions = self.executions.lock();
            if let Some(ex) = executions.get_mut(execution_id) {
                ex.pending_pause = None;
                ex.status = ExecutionStatus::Running;
            }
        }
        if let Some(ref repo) = self.repo {
            let _ = repo.update_status(
                execution_id,
                status_to_db_str(ExecutionStatus::Running),
                None,
                None,
            );
        }

        self.broadcast(ExecutionEvent::WorkflowResumed {
            execution_id: execution_id.to_string(),
            stage_name: pause.stage_name,
            chosen_value: value,
        });

        Ok(())
    }

    /// 模拟执行（用于测试）
    pub fn simulate_execution(&self, workflow_id: String) -> Execution {
        let execution = self.start_execution(workflow_id, serde_json::json!({}), None, None);

        // 模拟阶段执行
        let exec_id = execution.id.clone();
        let stages = ["初始化", "规划", "执行", "完成"];

        for stage in stages {
            // 阶段开始
            self.broadcast(ExecutionEvent::StageStarted {
                execution_id: exec_id.clone(),
                stage_name: stage.to_string(),
            });

            // 模拟一些输出
            self.broadcast(ExecutionEvent::Output {
                execution_id: exec_id.clone(),
                line: format!("[{}] 开始执行...", stage),
            });

            // 模拟延迟后阶段完成
            let output = serde_json::json!({
                "status": "success",
                "stage": stage,
                "duration_ms": 100
            });
            self.broadcast(ExecutionEvent::StageCompleted {
                execution_id: exec_id.clone(),
                stage_name: stage.to_string(),
                output,
                quality_gate_result: None,
            });
        }

        // 完成执行
        self.update_status(&exec_id, ExecutionStatus::Completed);
        self.broadcast(ExecutionEvent::Completed {
            execution_id: exec_id.clone(),
        });

        // 返回更新后的执行状态
        self.get_execution(&exec_id).unwrap_or(execution)
    }

    /// 使用真实的 AI 执行工作流
    ///
    /// # Arguments
    /// * `workflow_id` - 工作流 ID（仅用于标识）
    /// * `workflow_yaml` - 工作流 YAML 定义
    /// * `variables` - 执行变量
    /// * `ai_config` - AI 提供商配置
    /// * `working_directory` - 工作目录（用于 Claude CLI --project 参数）
    ///
    /// # Returns
    /// 执行 ID
    pub async fn execute_workflow(
        &self,
        workflow_id: String,
        workflow_yaml: &str,
        variables: serde_json::Value,
        ai_config: Option<NexusAIManagerConfig>,
        working_directory: Option<String>,
        launch: Option<ExecutionLaunchContext>,
    ) -> Result<String, ExecutionError> {
        use std::sync::Arc;

        // 1. 解析工作流定义
        let mut definition: WorkflowDefinition = serde_yaml::from_str(workflow_yaml)
            .map_err(|e| ExecutionError::ParseError(format!("YAML 解析失败: {}", e)))?;

        // 将用户传入的变量覆盖到工作流定义（非空值才覆盖，保留 YAML 默认值）
        if let Some(vars) = variables.as_object() {
            for (key, value) in vars {
                let should_inject = match value {
                    serde_json::Value::String(s) => !s.is_empty(),
                    serde_json::Value::Null => false,
                    _ => true,
                };
                if should_inject {
                    definition.variables.insert(key.clone(), value.clone());
                }
            }
        }

        // 注入项目级模板变量（供页面生成等阶段使用）
        {
            // project_path: 从 working_directory 获取
            if let Some(ref wd) = working_directory {
                definition
                    .variables
                    .entry("project_path".to_string())
                    .or_insert_with(|| serde_json::Value::String(wd.clone()));
            }

            // escalate_model: 从 AI 配置获取
            let escalate_model = ai_config
                .as_ref()
                .and_then(|c| c.default_escalate_model.clone())
                .unwrap_or_else(|| "opus".to_string());
            definition
                .variables
                .entry("escalate_model".to_string())
                .or_insert_with(|| serde_json::Value::String(escalate_model));
        }

        // 2. 创建 AI 管理器（优先 AppState 共享实例，含设置页 API Key + Claude CLI）
        let ai_manager: Arc<AIModelManager> = self
            .ai_manager
            .lock()
            .clone()
            .unwrap_or_else(|| {
                Arc::new(
                    ai_config
                        .map(AIModelManager::from_config)
                        .unwrap_or_else(|| AIModelManager::from_config(load_ai_config_from_env())),
                )
            });

        // 3. 先启动执行，拿到 exec_id，再创建事件桥（桥需要 exec_id 来替换引擎内部 UUID）
        let execution = self.start_execution(
            workflow_id.clone(),
            variables,
            working_directory.clone(),
            launch,
        );
        let exec_id = execution.id.clone();

        // 4. 创建事件发射器（桥接到 ExecutionService，绑定 exec_id + 预算限制）
        let mut bridge = WorkflowEventBridge::new(self.clone(), exec_id.clone());
        if let Some(limit) = definition.budget_limit_usd {
            bridge = bridge.with_budget(limit);
        }
        let event_emitter = Arc::new(bridge);

        // 5. 创建 resume channel，支持 user_input 暂停/恢复
        let (resume_tx, resume_rx) = tokio::sync::mpsc::channel::<String>(1);

        // 6. 创建工作流引擎（使用 Claude CLI，附带 resume channel）
        let mut engine =
            WorkflowEngine::with_resume_channel(event_emitter, working_directory, resume_rx);

        // 6.1 注入注册过的 stage 观察者（产物追踪等）
        for watcher in self.stage_watchers.lock().iter() {
            engine.add_stage_watcher(watcher.clone());
        }

        // 6.2 注入 RAG provider（如果已配置）
        if let Some(provider) = self.rag_provider.lock().clone() {
            engine.set_rag_provider(provider);
        }

        // 6.3 注入模型路由回调
        {
            let router = self.model_router.clone();
            engine.set_model_router_fn(std::sync::Arc::new(move |prompt, stage_name| {
                let task_type = infer_task_type(stage_name);
                let ctx = TaskContext {
                    prompt,
                    task_type: task_type.as_deref(),
                };
                router.lock().route(&ctx)
            }));
        }

        // 6.4 注入 API 车道（AF-04b）
        engine.set_api_executor(
            crate::services::workflow_api_executor::WorkflowApiExecutor::new(ai_manager.clone()),
        );

        // 7. 注册 resume channel
        {
            let mut channels = self.resume_channels.lock();
            channels.insert(exec_id.clone(), resume_tx);
        }

        // 8. 在后台执行工作流（不阻塞）
        let cancel_token = tokio_util::sync::CancellationToken::new();
        {
            let mut tokens = self.cancel_tokens.lock();
            tokens.insert(exec_id.clone(), cancel_token.clone());
        }

        let exec_service = self.clone();
        let workflow_def = definition.clone();

        tokio::spawn(async move {
            let work = engine.execute(&workflow_def);
            tokio::pin!(work);

            tokio::select! {
                result = &mut work => {
                    let already_cancelled = exec_service
                        .get_execution(&exec_id)
                        .map(|e| e.status == ExecutionStatus::Cancelled)
                        .unwrap_or(false);
                    if already_cancelled {
                        exec_service.resume_channels.lock().remove(&exec_id);
                        exec_service.cancel_tokens.lock().remove(&exec_id);
                        return;
                    }

                    match result {
                        Ok(result) => {
                            tracing::info!(
                                "工作流执行完成: execution_id={}, status={:?}",
                                result.execution_id,
                                result.status
                            );
                            exec_service.resume_channels.lock().remove(&exec_id);
                            exec_service.update_status(&exec_id, ExecutionStatus::Completed);
                            exec_service.broadcast(ExecutionEvent::Completed {
                                execution_id: exec_id.clone(),
                            });

                            // 链式触发：检查 workflow triggers 中是否有 type=event 的触发器
                            if let Some(ref handler) = exec_service.chain_trigger_handler {
                                for trigger in &workflow_def.triggers {
                                    if trigger.trigger_type == TriggerType::Event {
                                        if let Some(ref target_name) = trigger.workflow_ref {
                                            let variables = if trigger.pass_output.unwrap_or(false) {
                                                serde_json::json!({
                                                    "upstream_execution_id": result.execution_id.to_string(),
                                                    "stages": result.stage_results.iter().map(|sr| {
                                                        serde_json::json!({
                                                            "stage": sr.stage_name,
                                                            "outputs": sr.outputs,
                                                        })
                                                    }).collect::<Vec<_>>(),
                                                })
                                            } else {
                                                serde_json::json!({})
                                            };

                                            match handler(target_name.clone(), variables).await {
                                                Ok(downstream_id) => {
                                                    tracing::info!(
                                                        "[ChainTrigger] 下游工作流 '{}' 已触发, downstream_execution_id={}",
                                                        target_name,
                                                        downstream_id,
                                                    );
                                                }
                                                Err(e) => {
                                                    tracing::warn!(
                                                        "[ChainTrigger] 下游工作流 '{}' 触发失败: {}",
                                                        target_name,
                                                        e,
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            let error_msg = e.to_string();
                            tracing::error!("工作流执行失败: {}", error_msg);
                            exec_service.resume_channels.lock().remove(&exec_id);
                            exec_service.set_error(&exec_id, error_msg.clone());
                            exec_service.update_status(&exec_id, ExecutionStatus::Failed);
                            exec_service.broadcast(ExecutionEvent::Failed {
                                execution_id: exec_id.clone(),
                                error: error_msg,
                            });
                        }
                    }
                }
                _ = cancel_token.cancelled() => {
                    tracing::info!("工作流已取消: execution_id={}", exec_id);
                    exec_service.resume_channels.lock().remove(&exec_id);
                    let needs_update = exec_service
                        .get_execution(&exec_id)
                        .map(|e| e.status != ExecutionStatus::Cancelled)
                        .unwrap_or(true);
                    if needs_update {
                        exec_service.update_status(&exec_id, ExecutionStatus::Cancelled);
                    }
                }
            }

            exec_service.cancel_tokens.lock().remove(&exec_id);
        });

        Ok(execution.id)
    }
}

fn status_to_db_str(status: ExecutionStatus) -> &'static str {
    match status {
        ExecutionStatus::Pending => "pending",
        ExecutionStatus::Running => "running",
        ExecutionStatus::Paused => "paused",
        ExecutionStatus::Completed => "completed",
        ExecutionStatus::Failed => "failed",
        ExecutionStatus::Cancelled => "cancelled",
    }
}

/// 执行错误
#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error("解析错误: {0}")]
    ParseError(String),

    #[error("执行错误: {0}")]
    ExecutionError(String),
}

impl Default for ExecutionService {
    fn default() -> Self {
        Self::new()
    }
}

/// 持久化的 pause 状态（供快照 catch-up）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingPause {
    pub stage_name: String,
    pub question: String,
    pub options: Vec<crate::services::events::WorkflowOption>,
    #[serde(default = "default_pause_kind")]
    pub pause_kind: String,
}

fn default_pause_kind() -> String {
    "user_input".to_string()
}

/// 审批审计记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalEvent {
    pub stage_name: String,
    pub approved: bool,
    pub comment: Option<String>,
    pub decided_at: DateTime<Utc>,
}

/// 工作流执行
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Execution {
    pub id: String,
    pub workflow_id: String,
    pub status: ExecutionStatus,
    pub variables: serde_json::Value,
    pub stage_results: Vec<StageResult>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
    /// 实时输出日志缓存（最近 500 行），供新 WS 连接 catch-up 用
    #[serde(default)]
    pub output_log: Vec<String>,
    /// 当前正在执行的阶段名
    #[serde(default)]
    pub current_stage: Option<String>,
    /// 当前正在执行的 agent id 列表
    #[serde(default)]
    pub running_agents: Vec<String>,
    /// 当前 pause 状态（user_input 阶段等待输入时）
    #[serde(default)]
    pub pending_pause: Option<PendingPause>,
    /// 累计 token 消耗
    #[serde(default)]
    pub total_tokens: i64,
    /// 累计费用（美元）
    #[serde(default)]
    pub total_cost_usd: f64,
    /// 执行时的工作区路径（用于产物预览）
    #[serde(default)]
    pub workspace_path: Option<String>,
    /// 启动该 Run 的团队（工厂台）
    #[serde(default)]
    pub team_id: Option<String>,
    /// 关联项目
    #[serde(default)]
    pub project_id: Option<String>,
    /// 触发来源：factory / scheduler / webhook 等
    #[serde(default)]
    pub trigger_source: Option<String>,
    /// 审批审计记录
    #[serde(default)]
    pub approval_events: Vec<ApprovalEvent>,
    /// 从哪次失败 Run 重试而来（AF-UX-09 阶段重试）
    #[serde(default)]
    pub resumed_from: Option<String>,
}

impl Execution {
    /// 创建新执行
    pub fn new(workflow_id: String, variables: serde_json::Value) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            workflow_id,
            status: ExecutionStatus::Pending,
            variables,
            stage_results: Vec::new(),
            started_at: None,
            finished_at: None,
            error: None,
            output_log: Vec::new(),
            current_stage: None,
            running_agents: Vec::new(),
            pending_pause: None,
            total_tokens: 0,
            total_cost_usd: 0.0,
            workspace_path: None,
            team_id: None,
            project_id: None,
            trigger_source: None,
            approval_events: Vec::new(),
            resumed_from: None,
        }
    }

    /// 标记为运行中
    pub fn start(&mut self) {
        self.status = ExecutionStatus::Running;
        self.started_at = Some(Utc::now());
    }

    /// 标记为完成
    pub fn complete(&mut self) {
        self.status = ExecutionStatus::Completed;
        self.finished_at = Some(Utc::now());
    }

    /// 标记为失败
    pub fn fail(&mut self, error: String) {
        self.status = ExecutionStatus::Failed;
        self.error = Some(error);
        self.finished_at = Some(Utc::now());
    }

    /// 取消执行
    pub fn cancel(&mut self) {
        self.status = ExecutionStatus::Cancelled;
        self.finished_at = Some(Utc::now());
    }
}

/// 阶段执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageResult {
    pub stage_name: String,
    pub outputs: Vec<serde_json::Value>,
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub quality_gate_result: Option<serde_json::Value>,
}

/// 从 stage 名称推断任务类型，用于模型路由
fn infer_task_type(stage_name: &str) -> Option<String> {
    let lower = stage_name.to_lowercase();
    if lower.contains("test") || lower.contains("qa") {
        Some("testing".to_string())
    } else if lower.contains("plan") || lower.contains("design") || lower.contains("arch") {
        Some("planning".to_string())
    } else if lower.contains("doc") || lower.contains("summary") || lower.contains("report") {
        Some("documentation".to_string())
    } else if lower.contains("review") || lower.contains("audit") {
        Some("review".to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_new() {
        let execution = Execution::new(
            "workflow-1".to_string(),
            serde_json::json!({"key": "value"}),
        );
        assert_eq!(execution.workflow_id, "workflow-1");
        assert_eq!(execution.status, ExecutionStatus::Pending);
        assert_eq!(execution.variables, serde_json::json!({"key": "value"}));
        assert!(execution.started_at.is_none());
        assert!(execution.finished_at.is_none());
        assert!(execution.error.is_none());
    }

    #[test]
    fn test_execution_start() {
        let mut execution = Execution::new("workflow-1".to_string(), serde_json::json!({}));
        execution.start();
        assert_eq!(execution.status, ExecutionStatus::Running);
        assert!(execution.started_at.is_some());
    }

    #[test]
    fn test_execution_complete() {
        let mut execution = Execution::new("workflow-1".to_string(), serde_json::json!({}));
        execution.start();
        execution.complete();
        assert_eq!(execution.status, ExecutionStatus::Completed);
        assert!(execution.finished_at.is_some());
    }

    #[test]
    fn test_execution_fail() {
        let mut execution = Execution::new("workflow-1".to_string(), serde_json::json!({}));
        execution.fail("test error".to_string());
        assert_eq!(execution.status, ExecutionStatus::Failed);
        assert_eq!(execution.error, Some("test error".to_string()));
        assert!(execution.finished_at.is_some());
    }

    #[test]
    fn test_execution_cancel() {
        let mut execution = Execution::new("workflow-1".to_string(), serde_json::json!({}));
        execution.cancel();
        assert_eq!(execution.status, ExecutionStatus::Cancelled);
        assert!(execution.finished_at.is_some());
    }

    #[test]
    fn test_stage_result() {
        let result = StageResult {
            stage_name: "test-stage".to_string(),
            outputs: vec![serde_json::json!({"result": "ok"})],
            completed_at: Some(Utc::now()),
            quality_gate_result: None,
        };
        assert_eq!(result.stage_name, "test-stage");
        assert_eq!(result.outputs.len(), 1);
    }

    #[test]
    fn test_execution_service_new() {
        let service = ExecutionService::new();
        assert_eq!(service.get_all_executions().len(), 0);
    }

    #[test]
    fn test_start_execution() {
        let service = ExecutionService::new();
        let execution = service.start_execution(
            "workflow-1".to_string(),
            serde_json::json!({"var": 123}),
            None,
            None,
        );

        assert_eq!(execution.workflow_id, "workflow-1");
        assert_eq!(execution.status, ExecutionStatus::Running);
        assert_eq!(execution.variables, serde_json::json!({"var": 123}));

        let found = service.get_execution(&execution.id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, execution.id);
    }

    #[test]
    fn test_get_all_executions() {
        let service = ExecutionService::new();
        service.start_execution("workflow-1".to_string(), serde_json::json!({}), None, None);
        service.start_execution("workflow-2".to_string(), serde_json::json!({}), None, None);

        let executions = service.get_all_executions();
        assert_eq!(executions.len(), 2);
    }

    #[test]
    fn test_cancel_execution() {
        let service = ExecutionService::new();
        let execution =
            service.start_execution("workflow-1".to_string(), serde_json::json!({}), None, None);

        let cancelled = service.cancel_execution(&execution.id);
        assert!(cancelled);

        let found = service.get_execution(&execution.id).unwrap();
        assert_eq!(found.status, ExecutionStatus::Cancelled);
    }

    #[test]
    fn test_cancel_execution_not_found() {
        let service = ExecutionService::new();
        let cancelled = service.cancel_execution("non-existent-id");
        assert!(!cancelled);
    }

    #[test]
    fn test_update_status() {
        let service = ExecutionService::new();
        let execution =
            service.start_execution("workflow-1".to_string(), serde_json::json!({}), None, None);

        service.update_status(&execution.id, ExecutionStatus::Completed);

        let found = service.get_execution(&execution.id).unwrap();
        assert_eq!(found.status, ExecutionStatus::Completed);
        assert!(found.finished_at.is_some());
    }

    #[test]
    fn test_add_stage_output() {
        let service = ExecutionService::new();
        let execution =
            service.start_execution("workflow-1".to_string(), serde_json::json!({}), None, None);

        service.add_stage_output(
            &execution.id,
            "test-stage".to_string(),
            serde_json::json!({"output": "value"}),
        );

        let found = service.get_execution(&execution.id).unwrap();
        assert_eq!(found.stage_results.len(), 1);
        assert_eq!(found.stage_results[0].stage_name, "test-stage");
    }

    #[tokio::test]
    async fn test_subscribe() {
        let service = ExecutionService::new();
        let mut rx = service.subscribe();

        service.start_execution("workflow-1".to_string(), serde_json::json!({}), None, None);

        // Should receive events (Started + StatusChanged)
        let result = tokio::time::timeout(std::time::Duration::from_millis(100), rx.recv()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_simulate_execution() {
        let service = ExecutionService::new();
        let execution = service.simulate_execution("workflow-1".to_string());

        assert_eq!(execution.workflow_id, "workflow-1");
        assert_eq!(execution.status, ExecutionStatus::Completed);
        // simulate_execution broadcasts StageCompleted events but does not
        // populate stage_results (that requires add_stage_output).
        // It does add output lines via the Output broadcast handler.
        assert_eq!(execution.output_log.len(), 4); // one line per stage
    }

    #[test]
    fn test_execution_error_display() {
        let error = ExecutionError::ParseError("YAML error".to_string());
        assert!(error.to_string().contains("YAML error"));

        let error = ExecutionError::ExecutionError("execution failed".to_string());
        assert!(error.to_string().contains("execution failed"));
    }
}
