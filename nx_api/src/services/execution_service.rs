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
        ],
    }
}
use crate::services::execution_bridge::WorkflowEventBridge;
use crate::services::execution_repository::SqliteExecutionRepository;
use crate::services::model_router::{default_rules, ModelRouter, TaskContext};

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
        }
    }

    /// 创建带持久化的执行服务
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
        }
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
            } => {
                let mut executions = self.executions.lock();
                if let Some(ex) = executions.get_mut(execution_id.as_str()) {
                    ex.pending_pause = Some(PendingPause {
                        stage_name: stage_name.clone(),
                        question: question.clone(),
                        options: options.clone(),
                    });
                }
            }
            ExecutionEvent::WorkflowResumed { execution_id, .. } => {
                let mut executions = self.executions.lock();
                if let Some(ex) = executions.get_mut(execution_id.as_str()) {
                    ex.pending_pause = None;
                }
            }
            _ => {}
        }
        let _ = self.event_sender.send(event);
    }

    /// 启动新执行
    pub fn start_execution(&self, workflow_id: String, variables: serde_json::Value) -> Execution {
        let mut execution = Execution::new(workflow_id.clone(), variables);
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

    /// 获取执行状态（优先查内存，再查 DB）
    pub fn get_execution(&self, id: &str) -> Option<Execution> {
        let executions = self.executions.lock();
        if let Some(exec) = executions.get(id) {
            return Some(exec.clone());
        }
        drop(executions);
        // 内存没有，尝试从 DB 恢复
        self.repo
            .as_ref()
            .and_then(|repo| repo.find_by_id(id).ok())
            .flatten()
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
            } else {
                all.push(exec.clone());
            }
        }
        // 按 started_at 降序排列
        all.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        all
    }

    /// 取消执行
    pub fn cancel_execution(&self, id: &str) -> bool {
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
            true
        } else {
            false
        }
    }

    /// 更新执行状态
    pub fn update_status(&self, id: &str, status: ExecutionStatus) {
        let mut executions = self.executions.lock();
        if let Some(execution) = executions.get_mut(id) {
            execution.status = status;
            // 完成或失败时设置 finished_at
            if matches!(status, ExecutionStatus::Completed | ExecutionStatus::Failed) {
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

    /// 模拟执行（用于测试）
    pub fn simulate_execution(&self, workflow_id: String) -> Execution {
        let execution = self.start_execution(workflow_id, serde_json::json!({}));

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

        // 2. 创建 AI 管理器（保留用于其他可能的需求）
        let _ai_manager = ai_config
            .map(AIModelManager::from_config)
            .unwrap_or_else(|| {
                // 尝试从环境变量加载 AI 配置
                AIModelManager::from_config(load_ai_config_from_env())
            });

        // 3. 先启动执行，拿到 exec_id，再创建事件桥（桥需要 exec_id 来替换引擎内部 UUID）
        let execution = self.start_execution(workflow_id.clone(), variables);
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
            engine.set_model_router_fn(std::sync::Arc::new(move |prompt| {
                let ctx = TaskContext {
                    prompt,
                    task_type: None,
                };
                router.lock().route(&ctx)
            }));
        }

        // 7. 注册 resume channel
        {
            let mut channels = self.resume_channels.lock();
            channels.insert(exec_id.clone(), resume_tx);
        }

        // 8. 在后台执行工作流（不阻塞）
        let exec_service = self.clone();
        let workflow_def = definition.clone();

        tokio::spawn(async move {
            match engine.execute(&workflow_def).await {
                Ok(result) => {
                    tracing::info!(
                        "工作流执行完成: execution_id={}, status={:?}",
                        result.execution_id,
                        result.status
                    );
                    exec_service.resume_channels.lock().remove(&exec_id);
                    exec_service.update_status(&exec_id, ExecutionStatus::Completed);
                    exec_service.broadcast(ExecutionEvent::Completed {
                        execution_id: exec_id,
                    });

                    // 链式触发：检查 workflow triggers 中是否有 type=event 的触发器
                    if let Some(ref handler) = exec_service.chain_trigger_handler {
                        for trigger in &workflow_def.triggers {
                            if trigger.trigger_type == TriggerType::Event {
                                if let Some(ref target_name) = trigger.workflow_ref {
                                    let variables = if trigger.pass_output.unwrap_or(false) {
                                        // 将上游输出作为下游变量
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
                        execution_id: exec_id,
                        error: error_msg,
                    });
                }
            }
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
        let execution =
            service.start_execution("workflow-1".to_string(), serde_json::json!({"var": 123}));

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
        service.start_execution("workflow-1".to_string(), serde_json::json!({}));
        service.start_execution("workflow-2".to_string(), serde_json::json!({}));

        let executions = service.get_all_executions();
        assert_eq!(executions.len(), 2);
    }

    #[test]
    fn test_cancel_execution() {
        let service = ExecutionService::new();
        let execution = service.start_execution("workflow-1".to_string(), serde_json::json!({}));

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
        let execution = service.start_execution("workflow-1".to_string(), serde_json::json!({}));

        service.update_status(&execution.id, ExecutionStatus::Completed);

        let found = service.get_execution(&execution.id).unwrap();
        assert_eq!(found.status, ExecutionStatus::Completed);
        assert!(found.finished_at.is_some());
    }

    #[test]
    fn test_add_stage_output() {
        let service = ExecutionService::new();
        let execution = service.start_execution("workflow-1".to_string(), serde_json::json!({}));

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

        service.start_execution("workflow-1".to_string(), serde_json::json!({}));

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
