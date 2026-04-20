//! 执行服务

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::sync::broadcast;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use parking_lot::Mutex;

use nexus_workflow::{WorkflowDefinition, WorkflowEngine, InMemoryEventEmitter};
use nexus_ai::{AIProviderRegistry, AIModelManager, AIManagerConfig as NexusAIManagerConfig, ProviderType, APIConfig as NexusAPIConfig, ModelConfig};

pub use crate::services::events::{ExecutionEvent, ExecutionStatus};

/// 从环境变量加载 AI 配置
fn load_ai_config_from_env() -> NexusAIManagerConfig {
    let mut api_config = HashMap::new();

    // 加载 Anthropic API 配置
    if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
        if !api_key.is_empty() {
            api_config.insert(ProviderType::Anthropic, NexusAPIConfig {
                api_key,
                base_url: String::new(),
                organization_id: String::new(),
                timeout_secs: 120,
            });
        }
    }

    // 加载 OpenAI API 配置
    if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
        if !api_key.is_empty() {
            api_config.insert(ProviderType::OpenAI, NexusAPIConfig {
                api_key,
                base_url: String::new(),
                organization_id: String::new(),
                timeout_secs: 120,
            });
        }
    }

    // 加载 Google API 配置
    if let Ok(api_key) = std::env::var("GOOGLE_API_KEY") {
        if !api_key.is_empty() {
            api_config.insert(ProviderType::Google, NexusAPIConfig {
                api_key,
                base_url: String::new(),
                organization_id: String::new(),
                timeout_secs: 120,
            });
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

/// 执行服务
#[derive(Clone)]
pub struct ExecutionService {
    executions: Arc<Mutex<HashMap<String, Execution>>>,
    event_sender: broadcast::Sender<ExecutionEvent>,
    /// user_input pause/resume channel 注册表（execution_id → sender）
    resume_channels: Arc<Mutex<HashMap<String, tokio::sync::mpsc::Sender<String>>>>,
}

impl std::fmt::Debug for ExecutionService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecutionService").finish()
    }
}

impl ExecutionService {
    /// 创建新的执行服务
    pub fn new() -> Self {
        let (event_sender, _) = broadcast::channel(1000);
        Self {
            executions: Arc::new(Mutex::new(HashMap::new())),
            event_sender,
            resume_channels: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 订阅执行事件
    pub fn subscribe(&self) -> broadcast::Receiver<ExecutionEvent> {
        self.event_sender.subscribe()
    }

    /// 广播事件
    pub fn broadcast(&self, event: ExecutionEvent) {
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

    /// 获取执行状态
    pub fn get_execution(&self, id: &str) -> Option<Execution> {
        let executions = self.executions.lock();
        executions.get(id).cloned()
    }

    /// 获取所有执行
    pub fn get_all_executions(&self) -> Vec<Execution> {
        let executions = self.executions.lock();
        executions.values().cloned().collect()
    }

    /// 取消执行
    pub fn cancel_execution(&self, id: &str) -> bool {
        let mut executions = self.executions.lock();
        if let Some(execution) = executions.get_mut(id) {
            execution.cancel();
            let status = execution.status;
            drop(executions);
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
            let exec_id = execution.id.clone();
            drop(executions);
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
        let mut executions = self.executions.lock();
        if let Some(execution) = executions.get_mut(id) {
            execution.stage_results.push(StageResult {
                stage_name: stage_name.clone(),
                outputs: vec![output.clone()],
                completed_at: Some(chrono::Utc::now()),
            });
            let exec_id = execution.id.clone();
            drop(executions);
            self.broadcast(ExecutionEvent::StageCompleted {
                execution_id: exec_id,
                stage_name,
                output,
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
        let definition: WorkflowDefinition = serde_yaml::from_str(workflow_yaml)
            .map_err(|e| ExecutionError::ParseError(format!("YAML 解析失败: {}", e)))?;

        // 2. 创建 AI 管理器（保留用于其他可能的需求）
        let _ai_manager = ai_config
            .map(AIModelManager::from_config)
            .unwrap_or_else(|| {
                // 尝试从环境变量加载 AI 配置
                AIModelManager::from_config(load_ai_config_from_env())
            });

        // 3. 创建事件发射器（桥接到 ExecutionService）
        let event_emitter = Arc::new(WorkflowEventBridge::new(self.clone()));

        // 4. 创建 resume channel，支持 user_input 暂停/恢复
        let (resume_tx, resume_rx) = tokio::sync::mpsc::channel::<String>(1);

        // 5. 创建工作流引擎（使用 Claude CLI，附带 resume channel）
        let engine = WorkflowEngine::with_resume_channel(event_emitter, working_directory, resume_rx);

        // 6. 启动执行
        let execution = self.start_execution(workflow_id.clone(), variables);
        let exec_id = execution.id.clone();

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
                        result.execution_id, result.status
                    );
                    exec_service.resume_channels.lock().remove(&exec_id);
                    exec_service.update_status(&exec_id, ExecutionStatus::Completed);
                    exec_service.broadcast(ExecutionEvent::Completed {
                        execution_id: exec_id,
                    });
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_new() {
        let execution = Execution::new("workflow-1".to_string(), serde_json::json!({"key": "value"}));
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
        let execution = service.start_execution("workflow-1".to_string(), serde_json::json!({"var": 123}));

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
        assert_eq!(execution.stage_results.len(), 4); // 初始化, 规划, 执行, 完成
    }

    #[test]
    fn test_execution_error_display() {
        let error = ExecutionError::ParseError("YAML error".to_string());
        assert!(error.to_string().contains("YAML error"));

        let error = ExecutionError::ExecutionError("execution failed".to_string());
        assert!(error.to_string().contains("execution failed"));
    }
}