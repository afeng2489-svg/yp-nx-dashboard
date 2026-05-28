//! 工作流引擎
//!
//! 工作流的核心执行引擎。

use parking_lot::RwLock;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;

type ModelRouterFn = Arc<dyn Fn(&str, &str) -> Option<String> + Send + Sync>;

use crate::artifacts::PageManifest;
use crate::events::{EventEmitter, WorkflowEvent};
use crate::parser::{OnFail, QualityGate, StageType, WorkflowError as ParserWorkflowError};
use crate::watchers::page_generate::PageGenerateWatcher;
use crate::{
    AgentState, AgentStatus, QualityCheckResult, QualityGateResult, StageOutput,
    WorkflowDefinition, WorkflowState, WorkflowStatus,
};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

/// Claude CLI 调用结果
struct ClaudeCliResult {
    text: String,
    input_tokens: u64,
    output_tokens: u64,
}

/// 质量门单次运行结果（内部用）
struct GateRunResult {
    passed: bool,
    checks: Vec<QualityCheckResult>,
}

/// 共享工作流状态
type SharedState = Arc<RwLock<WorkflowState>>;

/// 工作流执行引擎
pub struct WorkflowEngine {
    /// 事件发射器
    event_emitter: Arc<dyn EventEmitter>,
    /// 工作目录（用于 Claude CLI --project 参数）
    working_directory: Option<String>,
    /// user_input stage 用：前端通过此 channel 发回用户选择的值
    resume_rx: Option<Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<String>>>>,
    /// stage 执行前后的观察者（产物追踪、token 监控等扩展点）
    stage_watchers: crate::watcher::StageWatchers,
    /// RAG 检索 provider（可选，由 nx_api 注入）
    rag_provider: Option<Arc<dyn crate::watcher::RagProvider>>,
    /// 模型路由回调（prompt → model name，stage.model 为 None 时调用）
    model_router_fn: Option<ModelRouterFn>,
    /// API 车道执行器（AF-04b，由 nx_api 注入 AIModelManager）
    api_executor: Option<Arc<dyn crate::executor::ApiExecutor>>,
}

impl WorkflowEngine {
    /// 创建新的工作流引擎
    pub fn new(event_emitter: Arc<dyn EventEmitter>) -> Self {
        Self {
            event_emitter,
            working_directory: None,
            resume_rx: None,
            stage_watchers: crate::watcher::StageWatchers::new(),
            rag_provider: None,
            model_router_fn: None,
            api_executor: None,
        }
    }

    /// 创建带工作目录的工作流引擎
    pub fn with_working_directory(
        event_emitter: Arc<dyn EventEmitter>,
        working_directory: Option<String>,
    ) -> Self {
        Self {
            event_emitter,
            working_directory,
            resume_rx: None,
            stage_watchers: crate::watcher::StageWatchers::new(),
            rag_provider: None,
            model_router_fn: None,
            api_executor: None,
        }
    }

    /// 创建支持 user_input pause/resume 的引擎
    pub fn with_resume_channel(
        event_emitter: Arc<dyn EventEmitter>,
        working_directory: Option<String>,
        resume_rx: tokio::sync::mpsc::Receiver<String>,
    ) -> Self {
        Self {
            event_emitter,
            working_directory,
            resume_rx: Some(Arc::new(tokio::sync::Mutex::new(resume_rx))),
            stage_watchers: crate::watcher::StageWatchers::new(),
            rag_provider: None,
            model_router_fn: None,
            api_executor: None,
        }
    }

    /// 注入 RAG provider
    pub fn set_rag_provider(&mut self, provider: Arc<dyn crate::watcher::RagProvider>) {
        self.rag_provider = Some(provider);
    }

    /// 注入模型路由回调
    pub fn set_model_router_fn(&mut self, f: ModelRouterFn) {
        self.model_router_fn = Some(f);
    }

    /// 注入 API 车道执行器
    pub fn set_api_executor(&mut self, executor: Arc<dyn crate::executor::ApiExecutor>) {
        self.api_executor = Some(executor);
    }

    /// 注册 stage 观察者（用于产物追踪、token 监控等）
    pub fn add_stage_watcher(&mut self, watcher: Arc<dyn crate::watcher::StageWatcher>) {
        self.stage_watchers.push(watcher);
    }

    /// 执行工作流
    pub async fn execute(
        &self,
        workflow: &WorkflowDefinition,
    ) -> Result<WorkflowResult, WorkflowError> {
        let state: SharedState = Arc::new(RwLock::new(WorkflowState::new(&workflow.name)));

        {
            let s = state.read();
            self.event_emitter.emit(WorkflowEvent::WorkflowStarted {
                execution_id: s.execution_id,
                workflow_id: workflow.name.clone(),
            });
        }

        state.write().start();

        // 从工作流定义初始化变量
        for (key, value) in &workflow.variables {
            state.write().set_var(key, value.clone());
        }

        // AF-UX-09：从指定阶段重试（跳过已完成阶段）
        let retry_from = state
            .read()
            .get_var("retry_from_stage")
            .and_then(|v| v.as_str().map(String::from));
        if let Some(ref from) = retry_from {
            if let Some(idx) = workflow.stages.iter().position(|s| &s.name == from) {
                for prior in workflow.stages.iter().take(idx) {
                    state.write().record_stage(
                        &prior.name,
                        vec![],
                        None,
                    );
                }
            }
        }

        // ── 新执行循环：支持条件跳转、user_input 暂停、loop ──
        let mut current_stage_name: Option<String> = retry_from.or_else(|| {
            workflow.stages.first().map(|s| s.name.clone())
        });

        while let Some(ref stage_name) = current_stage_name.clone() {
            if state.read().should_stop() {
                break;
            }

            // 找到当前要执行的 stage
            let stage_idx = workflow.stages.iter().position(|s| &s.name == stage_name);
            let stage = match stage_idx {
                Some(idx) => workflow.stages[idx].clone(),
                None => {
                    return Err(WorkflowError::Execution(format!(
                        "找不到 stage: {}",
                        stage_name
                    )));
                }
            };

            let exec_id_str = state.read().execution_id.to_string();
            {
                let s = state.read();
                self.event_emitter.emit(WorkflowEvent::StageStarted {
                    execution_id: s.execution_id,
                    stage_name: stage.name.clone(),
                    stage_index: stage_idx.unwrap_or(0),
                });
            }

            // 通知所有观察者：stage 开始（用于产物追踪等）
            self.stage_watchers.notify_before(&exec_id_str, &stage.name);

            // approval stage：暂停 → resolve → approve 继续 / reject 重跑
            if stage.stage_type == StageType::Approval {
                let mut approval_stage = stage.clone();
                if approval_stage.options.is_empty() {
                    approval_stage.options = vec![
                        crate::parser::UserInputOption {
                            label: "批准".to_string(),
                            value: "approved".to_string(),
                            description: None,
                        },
                        crate::parser::UserInputOption {
                            label: "驳回".to_string(),
                            value: "rejected".to_string(),
                            description: None,
                        },
                    ];
                }
                if approval_stage.question.is_none() {
                    approval_stage.question =
                        Some("请审批当前阶段产出，批准后继续，驳回将返回修改。".to_string());
                }

                let approval_policy = {
                    let s = state.read();
                    s.get_var("approval_policy")
                        .and_then(|v| v.as_str())
                        .unwrap_or("approve_all")
                        .to_string()
                };
                let approval_stages: Vec<_> = workflow
                    .stages
                    .iter()
                    .filter(|s| s.stage_type == StageType::Approval)
                    .collect();
                let is_final_approval = approval_stages
                    .last()
                    .map(|s| s.name == approval_stage.name)
                    .unwrap_or(true);

                let chosen_value = if approval_policy == "trust_gates_final_only"
                    && !is_final_approval
                {
                    "approved".to_string()
                } else {
                    self.wait_for_pause_response(&state, &approval_stage, "approval")
                        .await
                };

                let is_rejected = chosen_value.starts_with("rejected");
                if let Some(ref output_var) = approval_stage.output_var {
                    state.write().set_var(
                        output_var,
                        serde_json::Value::String(chosen_value.clone()),
                    );
                }

                let outputs = vec![StageOutput {
                    path: format!("approval://{}", approval_stage.name),
                    content: Some(chosen_value.clone()),
                    agent_id: None,
                    summary: None,
                    files_changed: vec![],
                }];

                {
                    let mut s = state.write();
                    s.record_stage(&approval_stage.name, outputs.clone(), None);
                }
                self.emit_stage_completed(&state, &approval_stage, &outputs, &None);
                self.stage_watchers
                    .notify_after(&exec_id_str, &approval_stage.name);

                current_stage_name = if is_rejected {
                    approval_stage
                        .on_reject_goto
                        .clone()
                        .or_else(|| Self::next_after(&workflow.stages, &approval_stage.name))
                } else {
                    self.compute_next_stage(&approval_stage, &workflow.stages, &state)
                };
                continue;
            }

            // 根据 stage 类型分发执行
            #[allow(deprecated)] // PageGenerate stage deprecated AF-00b; match kept for compat
            let outputs = match stage.stage_type {
                StageType::UserInput => {
                    let output_var = stage.output_var.clone().unwrap_or_default();
                    let chosen_value = self
                        .wait_for_pause_response(&state, &stage, "user_input")
                        .await;

                    if !output_var.is_empty() {
                        state.write().set_var(
                            &output_var,
                            serde_json::Value::String(chosen_value.clone()),
                        );
                    }

                    vec![StageOutput {
                        path: format!("user_input://{}", stage.name),
                        content: Some(chosen_value),
                        agent_id: None,
                        summary: None,
                        files_changed: vec![],
                    }]
                }

                StageType::Approval => unreachable!("handled above"),

                StageType::Loop => {
                    let mut loop_outputs = Vec::new();
                    let mut iteration = 0usize;

                    loop {
                        iteration += 1;
                        if iteration > stage.max_iterations {
                            return Err(WorkflowError::Execution(format!(
                                "Loop stage '{}' 超过最大循环次数 {}",
                                stage.name, stage.max_iterations
                            )));
                        }

                        for body_stage_name in &stage.body_stages {
                            let body_idx = workflow
                                .stages
                                .iter()
                                .position(|s| &s.name == body_stage_name);
                            let body_stage = match body_idx {
                                Some(idx) => workflow.stages[idx].clone(),
                                None => {
                                    return Err(WorkflowError::Execution(format!(
                                        "Loop body 找不到 stage: {}",
                                        body_stage_name
                                    )))
                                }
                            };
                            let body_outputs = self
                                .execute_stage(&state, &body_stage, &workflow.agents)
                                .await?;
                            loop_outputs.extend(body_outputs);
                        }

                        if let Some(ref cond) = stage.break_condition {
                            if Self::evaluate_condition(cond, &state.read().variables) {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    loop_outputs
                }

                StageType::PageGenerate {
                    manifest_template: _,
                    ref output_dir,
                } => {
                    let project_dir = PathBuf::from(output_dir)
                        .parent()
                        .map(|p| p.to_path_buf())
                        .unwrap_or_else(|| PathBuf::from("."));
                    let staging_dir = project_dir
                        .join(".nexus-staging")
                        .join(state.read().execution_id.to_string());
                    fs::create_dir_all(&staging_dir)?;

                    // 复制 package.json/tsconfig.json 到暂存目录
                    for config_file in &["package.json", "tsconfig.json"] {
                        let src = project_dir.join(config_file);
                        if src.exists() {
                            fs::copy(&src, staging_dir.join(config_file))?;
                        }
                    }

                    // 1. 运行 requirement-analyst → manifest.json
                    let analyst_stage = crate::parser::StageDefinition {
                        name: format!("{}_analyst", stage.name),
                        agents: vec!["requirement-analyst".into()],
                        stage_type: StageType::Agent,
                        ..Default::default()
                    };
                    let _analyst_outputs = self
                        .execute_stage(&state, &analyst_stage, &workflow.agents)
                        .await?;

                    // 2. 提取 manifest JSON (从 agent output state variable)
                    let manifest_json = {
                        let s = state.read();
                        s.get_var("requirement-analyst_output")
                            .cloned()
                            .unwrap_or_default()
                    };
                    let manifest_str = match &manifest_json {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    let manifest: PageManifest =
                        serde_json::from_str(&manifest_str).map_err(|e| {
                            WorkflowError::Execution(format!("Invalid PageManifest JSON: {}", e))
                        })?;
                    fs::write(staging_dir.join("manifest.json"), &manifest_str)?;

                    // 3. 并行运行 route/component/data agents
                    let parallel_stage = crate::parser::StageDefinition {
                        name: format!("{}_parallel", stage.name),
                        agents: vec![
                            "route-generator".into(),
                            "component-generator".into(),
                            "data-generator".into(),
                        ],
                        stage_type: StageType::Agent,
                        parallel: true,
                        ..Default::default()
                    };
                    self.execute_stage(&state, &parallel_stage, &workflow.agents)
                        .await?;

                    // 4. 本机 StageWatcher 执行 R1-R9 验证
                    let mut review = PageGenerateWatcher::validate(&staging_dir, &manifest);
                    let mut review_attempts = 0usize;
                    while review.verdict == "MANIFEST_MISMATCH" && review_attempts < 2 {
                        state.write().set_var(
                            "page_generate_watcher_failures",
                            serde_json::to_value(&review.failures).unwrap_or_default(),
                        );
                        let review_stage = crate::parser::StageDefinition {
                            name: format!("{}_review_{}", stage.name, review_attempts),
                            agents: vec!["code-reviewer".into()],
                            stage_type: StageType::Agent,
                            ..Default::default()
                        };
                        self.execute_stage(&state, &review_stage, &workflow.agents)
                            .await?;
                        review = PageGenerateWatcher::validate(&staging_dir, &manifest);
                        review_attempts += 1;
                    }

                    // 5. 质量门
                    let outputs = vec![StageOutput {
                        path: staging_dir.to_string_lossy().to_string(),
                        content: Some(manifest_str),
                        agent_id: None,
                        summary: Some(format!(
                            "PageGenerate: {} (review attempts: {})",
                            manifest.page_name, review_attempts
                        )),
                        files_changed: vec![],
                    }];
                    let (outputs, _quality_gate_result) = self
                        .run_quality_gate_loop(&state, &stage, &workflow.agents, outputs.clone())
                        .await?;

                    // 6. 原子性移动 (含回滚)
                    atomic_move_staging_to_src(&staging_dir, output_dir)?;

                    // 7. 清理
                    cleanup_old_staging_dirs(&project_dir, 5, 7)?;

                    outputs
                }

                StageType::Agent => {
                    // 首次执行 stage
                    let initial_result = self.execute_stage(&state, &stage, &workflow.agents).await;

                    let (outputs, quality_gate_result) = match initial_result {
                        Ok(out) => {
                            self.run_quality_gate_loop(&state, &stage, &workflow.agents, out)
                                .await?
                        }
                        Err(e) => {
                            self.handle_stage_failure(e, &state, &stage, workflow)
                                .await?
                        }
                    };

                    // 传递 quality_gate_result 给 record_stage
                    {
                        let mut s = state.write();
                        s.record_stage(&stage.name, outputs.clone(), quality_gate_result.clone());
                    }

                    // 直接跳到 stage 完成通知（跳过外层的 record_stage）
                    self.emit_stage_completed(&state, &stage, &outputs, &quality_gate_result);
                    self.stage_watchers.notify_after(&exec_id_str, &stage.name);

                    // 计算 next 并直接 continue（跳过外层的 record_stage + notify）
                    current_stage_name = self.compute_next_stage(&stage, &workflow.stages, &state);
                    continue;
                }
            };

            {
                let mut s = state.write();
                s.record_stage(&stage.name, outputs.clone(), None);
            }

            {
                let s = state.read();
                self.event_emitter.emit(WorkflowEvent::StageCompleted {
                    execution_id: s.execution_id,
                    stage_name: stage.name.clone(),
                    outputs: outputs.clone(),
                    quality_gate_result: None,
                });
            }

            // 通知所有观察者：stage 完成（用于产物 diff 计算等）
            self.stage_watchers.notify_after(&exec_id_str, &stage.name);

            // ── 计算下一个 stage ──
            if stage.stage_type == StageType::Loop || stage.next.is_empty() {
                current_stage_name = Self::next_after(&workflow.stages, &stage.name);
            } else {
                let vars = state.read().variables.clone();
                let mut jumped = false;
                for transition in &stage.next {
                    let should_jump = match &transition.condition {
                        None => true,
                        Some(cond) => Self::evaluate_condition(cond, &vars),
                    };
                    if should_jump {
                        current_stage_name = Some(transition.goto.clone());
                        jumped = true;
                        break;
                    }
                }
                if !jumped {
                    current_stage_name = Self::next_after(&workflow.stages, &stage.name);
                }
            }
        }

        let mut s = state.write();
        if s.status == WorkflowStatus::Running {
            s.complete();
            self.event_emitter.emit(WorkflowEvent::WorkflowCompleted {
                execution_id: s.execution_id,
                final_state: serde_json::to_string(&s.variables).unwrap_or_default(),
            });
        }

        Ok(WorkflowResult {
            execution_id: s.execution_id,
            status: s.status,
            variables: s.variables.clone(),
            stage_results: s.stage_results.clone(),
        })
    }

    /// 返回 stages 数组中 current_name 之后的下一个 stage 名（没有则 None 表示结束）
    fn next_after(stages: &[crate::parser::StageDefinition], current_name: &str) -> Option<String> {
        stages
            .iter()
            .position(|s| s.name == current_name)
            .and_then(|idx| stages.get(idx + 1))
            .map(|s| s.name.clone())
    }

    /// user_input / approval：发 WorkflowPaused，等待 resume channel，发 WorkflowResumed
    async fn wait_for_pause_response(
        &self,
        state: &SharedState,
        stage: &crate::parser::StageDefinition,
        pause_kind: &str,
    ) -> String {
        let question = stage
            .question
            .clone()
            .unwrap_or_else(|| "请选择".to_string());
        let options = stage.options.clone();

        self.event_emitter.emit(WorkflowEvent::WorkflowPaused {
            execution_id: state.read().execution_id,
            stage_name: stage.name.clone(),
            question: question.clone(),
            options: options
                .iter()
                .map(|o| (o.label.clone(), o.value.clone()))
                .collect(),
            pause_kind: pause_kind.to_string(),
        });

        let chosen_value = if let Some(ref resume_rx) = self.resume_rx {
            let mut rx = resume_rx.lock().await;
            rx.recv().await.unwrap_or_default()
        } else {
            stage
                .options
                .first()
                .map(|o| o.value.clone())
                .unwrap_or_default()
        };

        self.event_emitter.emit(WorkflowEvent::WorkflowResumed {
            execution_id: state.read().execution_id,
            stage_name: stage.name.clone(),
            chosen_value: chosen_value.clone(),
        });

        chosen_value
    }

    /// 求值条件表达式
    /// 支持：  变量名 == '值'  |  变量名 != '值'  |  变量名 >= 数字  |  变量名 <= 数字
    fn evaluate_condition(
        condition: &str,
        vars: &std::collections::HashMap<String, serde_json::Value>,
    ) -> bool {
        let cond = condition.trim();

        if let Some(idx) = cond.find(" == ") {
            let var_name = cond[..idx].trim();
            let expected = cond[idx + 4..].trim().trim_matches('\'').trim_matches('"');
            return vars
                .get(var_name)
                .and_then(|v| v.as_str())
                .map(|v| v == expected)
                .unwrap_or(false);
        }

        if let Some(idx) = cond.find(" != ") {
            let var_name = cond[..idx].trim();
            let expected = cond[idx + 4..].trim().trim_matches('\'').trim_matches('"');
            return vars
                .get(var_name)
                .and_then(|v| v.as_str())
                .map(|v| v != expected)
                .unwrap_or(true);
        }

        if let Some(idx) = cond.find(" >= ") {
            let var_name = cond[..idx].trim();
            let threshold: f64 = cond[idx + 4..].trim().parse().unwrap_or(0.0);
            return vars
                .get(var_name)
                .and_then(|v| {
                    v.as_str()
                        .and_then(|s| s.parse::<f64>().ok())
                        .or_else(|| v.as_f64())
                })
                .map(|v| v >= threshold)
                .unwrap_or(false);
        }

        if let Some(idx) = cond.find(" <= ") {
            let var_name = cond[..idx].trim();
            let threshold: f64 = cond[idx + 4..].trim().parse().unwrap_or(0.0);
            return vars
                .get(var_name)
                .and_then(|v| {
                    v.as_str()
                        .and_then(|s| s.parse::<f64>().ok())
                        .or_else(|| v.as_f64())
                })
                .map(|v| v <= threshold)
                .unwrap_or(false);
        }

        if let Some(v) = vars.get(cond) {
            return v.as_str().map(|s| s == "true").unwrap_or(false)
                || v.as_bool().unwrap_or(false);
        }

        false
    }

    /// 质量门循环：执行 stage → 跑检查 → 失败重试 → 返回 (outputs, quality_gate_result)
    async fn run_quality_gate_loop(
        &self,
        state: &SharedState,
        stage: &crate::parser::StageDefinition,
        agents: &[crate::parser::AgentDefinition],
        initial_outputs: Vec<StageOutput>,
    ) -> Result<(Vec<StageOutput>, Option<QualityGateResult>), WorkflowError> {
        let gate = match &stage.quality_gate {
            Some(g) => g,
            None => return Ok((initial_outputs, None)),
        };

        let resolved_gate = self.resolve_quality_gate(gate);
        let mut current_outputs = initial_outputs;
        let mut retry_count = 0usize;

        // AF-UX-09：用户选择跳过此质量门
        {
            let skip = state
                .read()
                .get_var("skip_quality_gate_for_stage")
                .and_then(|v| v.as_str())
                .map(|s| s == stage.name)
                .unwrap_or(false);
            if skip {
                tracing::info!(
                    "Stage '{}' 质量门已按用户请求跳过",
                    stage.name
                );
                return Ok((
                    current_outputs,
                    Some(QualityGateResult {
                        passed: true,
                        checks: vec![],
                        retry_count: 0,
                    }),
                ));
            }
        }

        loop {
            let gate_result = self
                .run_quality_gate(&resolved_gate, self.working_directory.as_deref())
                .await;

            if gate_result.passed {
                // 发射质量门通过事件
                {
                    let s = state.read();
                    let checks_summary = gate_result
                        .checks
                        .iter()
                        .map(|c| format!("{}: PASS", c.cmd))
                        .collect::<Vec<_>>()
                        .join(", ");
                    self.event_emitter.emit(WorkflowEvent::QualityGateChecked {
                        execution_id: s.execution_id,
                        stage_name: stage.name.clone(),
                        passed: true,
                        retry_count,
                        checks_summary,
                    });
                }
                return Ok((
                    current_outputs,
                    Some(QualityGateResult {
                        passed: true,
                        checks: gate_result.checks,
                        retry_count,
                    }),
                ));
            }

            // 质量门失败
            retry_count += 1;
            let can_retry = matches!(resolved_gate.on_fail, OnFail::Retry)
                && retry_count <= resolved_gate.max_retries;

            tracing::warn!(
                "Stage '{}' 质量门失败 (重试 {}/{})",
                stage.name,
                retry_count,
                resolved_gate.max_retries,
            );

            // 发射质量门检查事件
            {
                let s = state.read();
                let checks_summary = gate_result
                    .checks
                    .iter()
                    .map(|c| format!("{}: {}", c.cmd, if c.passed { "PASS" } else { "FAIL" }))
                    .collect::<Vec<_>>()
                    .join(", ");
                self.event_emitter.emit(WorkflowEvent::QualityGateChecked {
                    execution_id: s.execution_id,
                    stage_name: stage.name.clone(),
                    passed: false,
                    retry_count,
                    checks_summary,
                });
            }

            if !can_retry {
                return Err(WorkflowError::Execution(format!(
                    "Stage '{}' 质量门重试 {} 次后仍未通过",
                    stage.name, retry_count
                )));
            }

            // 构建错误反馈并重新执行 stage
            let error_summary = self.format_gate_errors(&gate_result);
            self.inject_gate_error_to_state(state, &stage.name, &error_summary);

            current_outputs = self.execute_stage(state, stage, agents).await?;
        }
    }

    /// 解析质量门（处理 template 引用）
    fn resolve_quality_gate(&self, gate: &QualityGate) -> QualityGate {
        if let Some(ref template_name) = gate.template {
            // 从内置模板解析 checks
            if let Some(template_checks) = Self::load_quality_gate_template(template_name) {
                return QualityGate {
                    checks: template_checks,
                    on_fail: gate.on_fail.clone(),
                    max_retries: gate.max_retries,
                    template: None,
                };
            }
            tracing::warn!("未找到质量门模板 '{}', 使用内联 checks", template_name);
        }
        gate.clone()
    }

    /// 加载内置质量门模板
    fn load_quality_gate_template(name: &str) -> Option<Vec<crate::parser::QualityCheck>> {
        match name {
            "rust_default" => Some(vec![
                crate::parser::QualityCheck {
                    cmd: "cargo build".to_string(),
                    timeout: 300,
                },
                crate::parser::QualityCheck {
                    cmd: "cargo test".to_string(),
                    timeout: 300,
                },
                crate::parser::QualityCheck {
                    cmd: "cargo clippy -- -D warnings".to_string(),
                    timeout: 300,
                },
            ]),
            "typescript_default" => Some(vec![
                crate::parser::QualityCheck {
                    cmd: "npx tsc --noEmit".to_string(),
                    timeout: 300,
                },
                crate::parser::QualityCheck {
                    cmd: "npm test".to_string(),
                    timeout: 300,
                },
            ]),
            "python_default" => Some(vec![
                crate::parser::QualityCheck {
                    cmd: "python -m pytest".to_string(),
                    timeout: 300,
                },
                crate::parser::QualityCheck {
                    cmd: "mypy .".to_string(),
                    timeout: 300,
                },
            ]),
            "go_default" => Some(vec![
                crate::parser::QualityCheck {
                    cmd: "go build ./...".to_string(),
                    timeout: 300,
                },
                crate::parser::QualityCheck {
                    cmd: "go test ./...".to_string(),
                    timeout: 300,
                },
            ]),
            "docker_default" => Some(vec![crate::parser::QualityCheck {
                cmd: "docker build .".to_string(),
                timeout: 600,
            }]),
            _ => None,
        }
    }

    /// 执行质量门检查命令
    async fn run_quality_gate(
        &self,
        gate: &QualityGate,
        working_dir: Option<&str>,
    ) -> GateRunResult {
        let mut checks = Vec::new();
        let mut all_passed = true;

        for check in &gate.checks {
            if let Err(e) = nexus_sandbox::validate_shell_command(&check.cmd) {
                tracing::warn!("质量门命令被 blocklist 拒绝: {} — {}", check.cmd, e);
                all_passed = false;
                checks.push(QualityCheckResult {
                    cmd: check.cmd.clone(),
                    passed: false,
                    exit_code: None,
                    stdout: String::new(),
                    stderr: e.to_string(),
                    duration_ms: 0,
                });
                continue;
            }

            let start = std::time::Instant::now();

            let mut cmd = tokio::process::Command::new("sh");
            cmd.arg("-c").arg(&check.cmd);

            if let Some(dir) = working_dir {
                cmd.current_dir(dir);
            }

            cmd.stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());

            let result =
                tokio::time::timeout(std::time::Duration::from_secs(check.timeout), cmd.output())
                    .await;

            let duration_ms = start.elapsed().as_millis() as u64;

            let check_result = match result {
                Ok(Ok(output)) => {
                    let passed = output.status.success();
                    if !passed {
                        all_passed = false;
                    }
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    QualityCheckResult {
                        cmd: check.cmd.clone(),
                        passed,
                        exit_code: output.status.code(),
                        stdout: truncate_str(&stdout, 2000),
                        stderr: truncate_str(&stderr, 2000),
                        duration_ms,
                    }
                }
                Ok(Err(e)) => {
                    all_passed = false;
                    QualityCheckResult {
                        cmd: check.cmd.clone(),
                        passed: false,
                        exit_code: None,
                        stdout: String::new(),
                        stderr: e.to_string(),
                        duration_ms,
                    }
                }
                Err(_) => {
                    all_passed = false;
                    QualityCheckResult {
                        cmd: check.cmd.clone(),
                        passed: false,
                        exit_code: None,
                        stdout: String::new(),
                        stderr: format!("超时 ({}s)", check.timeout),
                        duration_ms,
                    }
                }
            };

            tracing::info!(
                "质量门检查 '{}' → {} ({:?}ms)",
                check.cmd,
                if check_result.passed {
                    "通过"
                } else {
                    "失败"
                },
                duration_ms,
            );

            checks.push(check_result);
        }

        GateRunResult {
            passed: all_passed,
            checks,
        }
    }

    /// 格式化质量门错误信息（注入给 AI 重试）
    fn format_gate_errors(&self, result: &GateRunResult) -> String {
        let mut summary = String::from("质量门检查失败：\n");
        for check in &result.checks {
            if !check.passed {
                summary.push_str(&format!("\n❌ 命令: {}\n", check.cmd));
                if !check.stdout.is_empty() {
                    summary.push_str(&format!("stdout:\n{}\n", check.stdout));
                }
                if !check.stderr.is_empty() {
                    summary.push_str(&format!("stderr:\n{}\n", check.stderr));
                }
            }
        }
        summary
    }

    /// 将质量门错误信息注入到 state 变量中，供 agent 下次执行时读取
    fn inject_gate_error_to_state(
        &self,
        state: &SharedState,
        stage_name: &str,
        error_summary: &str,
    ) {
        let var_key = format!("{}_quality_gate_error", stage_name);
        state.write().set_var(
            &var_key,
            serde_json::Value::String(error_summary.to_string()),
        );
    }

    /// 发射 StageCompleted 事件
    fn emit_stage_completed(
        &self,
        state: &SharedState,
        stage: &crate::parser::StageDefinition,
        outputs: &[StageOutput],
        quality_gate_result: &Option<QualityGateResult>,
    ) {
        let s = state.read();
        self.event_emitter.emit(WorkflowEvent::StageCompleted {
            execution_id: s.execution_id,
            stage_name: stage.name.clone(),
            outputs: outputs.to_vec(),
            quality_gate_result: quality_gate_result.clone(),
        });
    }

    /// 计算下一个 stage
    fn compute_next_stage(
        &self,
        stage: &crate::parser::StageDefinition,
        stages: &[crate::parser::StageDefinition],
        state: &SharedState,
    ) -> Option<String> {
        if stage.stage_type == StageType::Loop || stage.next.is_empty() {
            Self::next_after(stages, &stage.name)
        } else {
            let vars = state.read().variables.clone();
            for transition in &stage.next {
                let should_jump = match &transition.condition {
                    None => true,
                    Some(cond) => Self::evaluate_condition(cond, &vars),
                };
                if should_jump {
                    return Some(transition.goto.clone());
                }
            }
            Self::next_after(stages, &stage.name)
        }
    }

    /// stage 失败后的自愈逻辑：同模型重试 → 换模型重试 → then 动作
    async fn handle_stage_failure(
        &self,
        initial_err: WorkflowError,
        state: &SharedState,
        stage: &crate::parser::StageDefinition,
        workflow: &crate::parser::WorkflowDefinition,
    ) -> Result<(Vec<StageOutput>, Option<QualityGateResult>), WorkflowError> {
        // 优先用 stage 级 on_fail，否则降级到 workflow 级 on_error
        if let Some(ref policy) = stage.on_fail {
            let mut last_err = initial_err;

            // 1. 同模型重试
            for attempt in 1..=policy.retry {
                tracing::warn!(
                    "[FailRecovery] stage='{}' 同模型重试 {}/{}",
                    stage.name,
                    attempt,
                    policy.retry
                );
                self.emit_model_escalation(state, &stage.name, None, attempt, policy.retry);
                match self.execute_stage(state, stage, &workflow.agents).await {
                    Ok(out) => {
                        return self
                            .run_quality_gate_loop(state, stage, &workflow.agents, out)
                            .await
                    }
                    Err(e) => last_err = e,
                }
            }

            // 2. 换强模型重试
            if let Some(ref escalate_model) = policy.escalate_model {
                let resolved_model = state.read().resolve_template(escalate_model);
                let mut escalated_stage = stage.clone();
                escalated_stage.model = Some(resolved_model.clone());
                for attempt in 1..=policy.escalate_retries {
                    tracing::warn!(
                        "[FailRecovery] stage='{}' 升级模型 {} 重试 {}/{}",
                        stage.name,
                        resolved_model,
                        attempt,
                        policy.escalate_retries
                    );
                    self.emit_model_escalation(
                        state,
                        &stage.name,
                        Some(&resolved_model),
                        attempt,
                        policy.escalate_retries,
                    );
                    match self
                        .execute_stage(state, &escalated_stage, &workflow.agents)
                        .await
                    {
                        Ok(out) => {
                            return self
                                .run_quality_gate_loop(
                                    state,
                                    &escalated_stage,
                                    &workflow.agents,
                                    out,
                                )
                                .await
                        }
                        Err(e) => last_err = e,
                    }
                }
            }

            // 3. then 动作
            match policy.then.as_str() {
                "continue" => {
                    tracing::warn!(
                        "[FailRecovery] stage='{}' 全部重试失败，continue_on_error",
                        stage.name
                    );
                    Ok((vec![], None))
                }
                _ => Err(last_err), // "fail" | "rollback"（rollback 由上层 git 处理）
            }
        } else if let Some(ref error_handler) = workflow.on_error {
            // 降级到 workflow 级重试
            if error_handler.retry {
                let mut last_err = initial_err;
                for attempt in 1..=error_handler.max_retries {
                    tracing::warn!(
                        "Stage '{}' 失败，重试 {}/{}",
                        stage.name,
                        attempt,
                        error_handler.max_retries
                    );
                    match self.execute_stage(state, stage, &workflow.agents).await {
                        Ok(out) => {
                            return self
                                .run_quality_gate_loop(state, stage, &workflow.agents, out)
                                .await
                        }
                        Err(e) => last_err = e,
                    }
                }
                Err(last_err)
            } else {
                Err(initial_err)
            }
        } else {
            Err(initial_err)
        }
    }

    fn emit_model_escalation(
        &self,
        state: &SharedState,
        stage_name: &str,
        escalate_model: Option<&str>,
        attempt: usize,
        max: usize,
    ) {
        let execution_id = state.read().execution_id;
        let msg = match escalate_model {
            Some(m) => format!("已升级模型 {} 重试 {}/{}", m, attempt, max),
            None => format!("同模型重试 {}/{}", attempt, max),
        };
        self.event_emitter.emit(WorkflowEvent::AgentMessage {
            execution_id,
            agent_id: stage_name.to_string(),
            message: format!("[FailRecovery] {}", msg),
        });
    }

    /// 执行单个阶段
    async fn execute_stage(
        &self,
        state: &SharedState,
        stage: &crate::parser::StageDefinition,
        agents: &[crate::parser::AgentDefinition],
    ) -> Result<Vec<StageOutput>, WorkflowError> {
        if stage.parallel {
            // 并行执行智能体
            let mut handles = Vec::new();
            for agent_id in &stage.agents {
                let agent = agents.iter().find(|a| &a.id == agent_id).ok_or_else(|| {
                    WorkflowError::Validation(format!("未找到智能体: {}", agent_id))
                })?;

                // 检查依赖
                if !self.check_dependencies(agent, state)? {
                    continue;
                }

                let state_clone = Arc::clone(state);
                let agent_clone = agent.clone();
                let engine = self.clone();

                let stage_clone = stage.clone();
                let rag_config = stage.rag.clone();
                let model = stage.model.clone();
                handles.push(tokio::spawn(async move {
                    engine
                        .execute_agent(
                            &state_clone,
                            &stage_clone,
                            &agent_clone,
                            rag_config.as_ref(),
                            model.as_deref(),
                        )
                        .await
                }));
            }

            let mut outputs = Vec::new();
            let mut errors = Vec::new();
            for handle in handles {
                match handle.await {
                    Ok(Ok(agent_outputs)) => outputs.extend(agent_outputs),
                    Ok(Err(e)) => errors.push(e),
                    Err(e) => errors.push(WorkflowError::Execution(format!("任务 panic: {}", e))),
                }
            }

            if !errors.is_empty() && !stage.continue_on_error {
                return Err(errors.into_iter().next().unwrap());
            }

            Ok(outputs)
        } else {
            // 顺序执行智能体
            let mut outputs = Vec::new();
            for agent_id in &stage.agents {
                let agent = agents.iter().find(|a| &a.id == agent_id).ok_or_else(|| {
                    WorkflowError::Validation(format!("未找到智能体: {}", agent_id))
                })?;

                // 检查依赖
                if !self.check_dependencies(agent, state)? {
                    continue;
                }

                match self
                    .execute_agent(state, stage, agent, stage.rag.as_ref(), stage.model.as_deref())
                    .await
                {
                    Ok(agent_outputs) => outputs.extend(agent_outputs),
                    Err(e) => {
                        if stage.continue_on_error {
                            tracing::warn!("智能体 {} 失败但继续执行: {}", agent_id, e);
                        } else {
                            return Err(e);
                        }
                    }
                }
            }

            Ok(outputs)
        }
    }

    /// 检查所有依赖是否满足
    fn check_dependencies(
        &self,
        agent: &crate::parser::AgentDefinition,
        state: &SharedState,
    ) -> Result<bool, WorkflowError> {
        if agent.depends_on.is_empty() {
            return Ok(true);
        }
        let state_read = state.read();
        for dep_id in &agent.depends_on {
            if let Some(dep_state) = state_read.agent_states.get(dep_id) {
                if dep_state.status != AgentStatus::Completed {
                    return Ok(false);
                }
            } else {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// 从 AI 回复中提取结构化 JSON 摘要
    fn parse_structured_summary(response: &str) -> (Option<String>, Vec<String>) {
        // 匹配 ```json { ... } ``` 代码块
        let json_block_re = Regex::new(r"```json\s*(\{[^`]+\})\s*```").unwrap();
        if let Some(cap) = json_block_re.captures(response) {
            if let Some(json_str) = cap.get(1) {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str.as_str()) {
                    let summary = parsed
                        .get("summary")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let files = parsed
                        .get("files_changed")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default();
                    return (summary, files);
                }
            }
        }
        (None, vec![])
    }

    /// 执行单个智能体
    async fn execute_agent(
        &self,
        state: &SharedState,
        stage: &crate::parser::StageDefinition,
        agent: &crate::parser::AgentDefinition,
        rag_config: Option<&crate::parser::RagConfig>,
        model_override: Option<&str>,
    ) -> Result<Vec<StageOutput>, WorkflowError> {
        let execution_id = state.read().execution_id;

        let mut agent_state = AgentState {
            agent_id: agent.id.clone(),
            role: agent.role.clone(),
            status: AgentStatus::Running,
            last_message: None,
            updated_at: chrono::Utc::now(),
        };

        // 写入 Running 状态
        state.write().update_agent(&agent.id, agent_state.clone());

        self.event_emitter.emit(WorkflowEvent::AgentStarted {
            execution_id,
            agent_id: agent.id.clone(),
            role: agent.role.clone(),
        });

        // 使用解析后的变量构建提示词
        let resolved_prompt = state.read().resolve_template(&agent.prompt);

        // RAG 注入：检索相关知识并追加到 prompt
        let rag_context = if let (Some(rag), Some(provider)) = (rag_config, &self.rag_provider) {
            let texts = provider
                .retrieve(
                    &rag.knowledge_base_id,
                    &resolved_prompt,
                    rag.top_k,
                    rag.threshold,
                )
                .await;
            if texts.is_empty() {
                String::new()
            } else {
                format!(
                    "\n\n<knowledge>\n以下是与当前任务相关的参考知识：\n\n{}\n</knowledge>",
                    texts.join("\n\n---\n\n")
                )
            }
        } else {
            String::new()
        };

        // Auto-yes prefix to skip confirmation prompts
        let auto_yes_prefix = "You are operating in auto-yes mode. If you ask any question requiring confirmation (yes/no, continue?, proceed?, etc.), always assume the answer is YES and proceed automatically. Never ask for confirmation.";

        // 结构化输出要求
        let structured_output_instruction = "\n\n---\n## 输出格式要求（必须遵守）\n\
            在完成所有工作后，你必须在回复的最后添加一个 JSON 摘要块，格式如下：\n\
            ```json\n\
            {\n\
              \"summary\": \"用中文简要描述你完成了什么（2-3句话）\",\n\
              \"files_changed\": [\"变更的文件路径1\", \"变更的文件路径2\"]\n\
            }\n\
            ```\n\
            注意：\n\
            - summary 必须用中文，简洁描述你实际做了什么\n\
            - files_changed 列出你创建或修改的所有文件路径\n\
            - 这个 JSON 块必须放在回复的最后";

        // 如果 agent 有自定义 output_format，用它替换默认 JSON 摘要指令
        let format_instruction = if let Some(ref fmt) = agent.output_format {
            format!("\n\n输出格式要求: {}\n", state.read().resolve_template(fmt))
        } else {
            structured_output_instruction.to_string()
        };

        // 构建 prompt（Claude CLI 格式）
        let full_prompt = format!(
            "{}\n\n<system>\n你扮演 {}. 请仔细遵循你的指示。\n</system>\n\n<user>\n{}{}{}\n</user>",
            auto_yes_prefix, agent.role, resolved_prompt, rag_context, format_instruction
        );

        // 模型路由：stage 未指定 model 时，用路由器自动选择
        let routed_model;
        let effective_model = if model_override.is_some() {
            model_override
        } else if let Some(ref router_fn) = self.model_router_fn {
            routed_model = router_fn(&resolved_prompt, &agent.id);
            if let Some(ref m) = routed_model {
                tracing::info!("[ModelRouter] agent='{}' → {}", agent.id, m);
            }
            routed_model.as_deref()
        } else {
            None
        };

        // 通过 Claude CLI 或 API 执行
        let executor_kind = crate::executor::resolve_executor(stage, agent);
        tracing::info!(
            "[Executor] stage='{}' agent='{}' → {}",
            stage.name,
            agent.id,
            executor_kind.as_str()
        );

        let run_result: Result<(String, u64, u64, String, String, f64), WorkflowError> =
            if executor_kind == crate::executor::ExecutorKind::Api {
                let api = self.api_executor.as_ref().ok_or_else(|| {
                    WorkflowError::Execution(
                        "executor=api 但未注入 API 执行器（检查 AI Provider 配置）".into(),
                    )
                })?;
                let system_prompt = format!("你扮演 {}. 请仔细遵循指示。", agent.role);
                let user_message = format!(
                    "{}{}{}",
                    resolved_prompt, rag_context, format_instruction
                );
                let model = effective_model.unwrap_or(agent.model.as_str());
                let cost_mode = state
                    .read()
                    .get_var("text_lane_cost_mode")
                    .and_then(|v| v.as_str().map(String::from));
                match api
                    .complete(crate::executor::ApiCompletionRequest {
                        system_prompt,
                        user_message,
                        model: model.to_string(),
                        max_tokens: agent.config.max_tokens,
                        temperature: agent.config.temperature,
                        stage_name: Some(stage.name.clone()),
                        cost_mode,
                    })
                    .await
                {
                    Ok(r) => Ok((
                        r.text,
                        r.input_tokens,
                        r.output_tokens,
                        "api".into(),
                        r.provider,
                        r.estimated_cost_usd,
                    )),
                    Err(e) => Err(e.into()),
                }
            } else {
                match self.call_claude_cli(&full_prompt, effective_model).await {
                    Ok(cli) => {
                        let cost = (cli.input_tokens as f64 * 3.0 / 1_000_000.0)
                            + (cli.output_tokens as f64 * 15.0 / 1_000_000.0);
                        Ok((
                            cli.text,
                            cli.input_tokens,
                            cli.output_tokens,
                            "claude_cli".into(),
                            "anthropic".into(),
                            cost,
                        ))
                    }
                    Err(e) => Err(e),
                }
            };

        match run_result {
            Ok((response, input_tokens, output_tokens, executor, provider, estimated_cost_usd)) => {
                agent_state.status = AgentStatus::Completed;
                agent_state.last_message = Some(response.clone());
                agent_state.updated_at = chrono::Utc::now();

                // ── 自动注入：将 agent 输出写入 {agent_id}_output 变量，供后续 agent 引用 ──
                state.write().set_var(
                    &format!("{}_output", agent.id),
                    serde_json::Value::String(response.clone()),
                );

                // ── 变量提取：从输出中提取变量写入 state ──
                for extraction in &agent.extract_vars {
                    if let Ok(re) = Regex::new(&extraction.pattern) {
                        if let Some(cap) = re.captures(&response) {
                            if let Some(val) = cap.get(1) {
                                state.write().set_var(
                                    &extraction.name,
                                    serde_json::Value::String(val.as_str().to_string()),
                                );
                                tracing::debug!("变量提取: {} = {}", extraction.name, val.as_str());
                            }
                        }
                    }
                }

                // 写回完成状态
                state.write().update_agent(&agent.id, agent_state);

                // 发出 token 用量事件
                if input_tokens > 0 || output_tokens > 0 {
                    self.event_emitter.emit(WorkflowEvent::AgentTokenUsage {
                        execution_id,
                        agent_id: agent.id.clone(),
                        input_tokens,
                        output_tokens,
                        executor,
                        provider,
                        estimated_cost_usd,
                    });
                }

                self.event_emitter.emit(WorkflowEvent::AgentCompleted {
                    execution_id,
                    agent_id: agent.id.clone(),
                    output: response.clone(),
                });

                // 解析结构化摘要
                let (summary, files_changed) = Self::parse_structured_summary(&response);

                Ok(vec![StageOutput {
                    path: format!("agent://{}/output", agent.id),
                    content: Some(response),
                    agent_id: Some(agent.id.clone()),
                    summary,
                    files_changed,
                }])
            }
            Err(e) => {
                agent_state.status = AgentStatus::Failed;
                agent_state.updated_at = chrono::Utc::now();

                // 写回失败状态
                state.write().update_agent(&agent.id, agent_state);

                self.event_emitter.emit(WorkflowEvent::AgentFailed {
                    execution_id,
                    agent_id: agent.id.clone(),
                    error: e.to_string(),
                });

                Err(WorkflowError::Execution(format!(
                    "智能体 {} 失败: {}",
                    agent.id, e
                )))
            }
        }
    }

    /// 调用 Claude CLI（stream-json 模式，解析 token usage）
    async fn call_claude_cli(
        &self,
        prompt: &str,
        model: Option<&str>,
    ) -> Result<ClaudeCliResult, WorkflowError> {
        if let Err(e) = nexus_sandbox::check_blocklist(prompt) {
            return Err(WorkflowError::Execution(e.to_string()));
        }
        if let Some(ref dir) = self.working_directory {
            if let Err(e) =
                nexus_sandbox::validate_working_directory(dir, Some(dir.as_str()))
            {
                return Err(WorkflowError::Execution(e.to_string()));
            }
        }

        let (claude_bin, prefix_args) = nexus_sandbox::claude_cli_spawn_spec().map_err(|e| {
            WorkflowError::Execution(format!(
                "{}\n\
                提示：在应用内打开「设置 → AI」检测/配置 Claude CLI 路径；\
                本地开发可设 NEXUS_PERMISSIONS_MODE=trusted 以启用 agent 工具权限。",
                e
            ))
        })?;

        let mut cmd = Command::new(&claude_bin);
        for arg in &prefix_args {
            cmd.arg(arg);
        }
        for arg in nexus_sandbox::build_workflow_cli_args(model) {
            cmd.arg(arg);
        }
        cmd.arg(prompt);

        if let Some(ref dir) = self.working_directory {
            cmd.current_dir(dir);
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let child = cmd.spawn().map_err(|e| {
            let display = if prefix_args.is_empty() {
                claude_bin.clone()
            } else {
                format!("{} {}", claude_bin, prefix_args.join(" "))
            };
            if e.kind() == std::io::ErrorKind::NotFound {
                WorkflowError::Execution(format!(
                    "未找到 Claude Code CLI 可执行文件 (路径: {}).\n\
                    请先安装：npm install -g @anthropic-ai/claude-code\n\
                    或在「设置 → AI」手动指定 Claude CLI 路径。\n\
                    底层错误: {}",
                    display, e
                ))
            } else {
                WorkflowError::Execution(format!("启动 Claude CLI 失败 ({}): {}", display, e))
            }
        })?;

        let output = child
            .wait_with_output()
            .await
            .map_err(|e| WorkflowError::Execution(format!("Claude CLI error: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(WorkflowError::Execution(format!(
                "Claude CLI error: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut text_parts: Vec<String> = Vec::new();
        let mut total_input_tokens: u64 = 0;
        let mut total_output_tokens: u64 = 0;

        for line in stdout.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) {
                // 提取文本内容
                if let Some(content) = json.get("content").and_then(|c| c.as_str()) {
                    text_parts.push(content.to_string());
                } else if let Some(result) = json.get("result").and_then(|r| r.as_str()) {
                    text_parts.push(result.to_string());
                }
                // 提取 token usage
                if let Some(usage) = json.get("usage") {
                    if let Some(it) = usage.get("input_tokens").and_then(|v| v.as_u64()) {
                        total_input_tokens += it;
                    }
                    if let Some(ot) = usage.get("output_tokens").and_then(|v| v.as_u64()) {
                        total_output_tokens += ot;
                    }
                }
            } else {
                // 非 JSON 行，可能是纯文本残留
                text_parts.push(trimmed.to_string());
            }
        }

        let text = text_parts.join("\n").trim().to_string();

        Ok(ClaudeCliResult {
            text,
            input_tokens: total_input_tokens,
            output_tokens: total_output_tokens,
        })
    }
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let mut end = max_len;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}... (截断)", &s[..end])
    }
}

impl Clone for WorkflowEngine {
    fn clone(&self) -> Self {
        Self {
            event_emitter: self.event_emitter.clone(),
            working_directory: self.working_directory.clone(),
            resume_rx: self.resume_rx.clone(),
            stage_watchers: self.stage_watchers.clone(),
            rag_provider: self.rag_provider.clone(),
            model_router_fn: self.model_router_fn.clone(),
            api_executor: self.api_executor.clone(),
        }
    }
}

/// 工作流执行结果
#[derive(Debug, Clone)]
pub struct WorkflowResult {
    pub execution_id: uuid::Uuid,
    pub status: WorkflowStatus,
    pub variables: std::collections::HashMap<String, serde_json::Value>,
    pub stage_results: Vec<crate::StageResult>,
}

/// 工作流执行错误
#[derive(Debug, thiserror::Error)]
pub enum WorkflowError {
    #[error("解析错误: {0}")]
    Parse(String),

    #[error("验证错误: {0}")]
    Validation(String),

    #[error("执行错误: {0}")]
    Execution(String),

    #[error("IO 错误: {0}")]
    Io(String),
}

impl From<ParserWorkflowError> for WorkflowError {
    fn from(e: ParserWorkflowError) -> Self {
        match e {
            ParserWorkflowError::Parse(s) => WorkflowError::Parse(s),
            ParserWorkflowError::Validation(s) => WorkflowError::Validation(s),
            ParserWorkflowError::Io(s) => WorkflowError::Io(s),
        }
    }
}

impl From<nexus_ai::AIError> for WorkflowError {
    fn from(e: nexus_ai::AIError) -> Self {
        WorkflowError::Execution(e.to_string())
    }
}

impl From<std::io::Error> for WorkflowError {
    fn from(e: std::io::Error) -> Self {
        WorkflowError::Io(e.to_string())
    }
}

/// 将暂存目录中的文件原子性移动到目标 src 目录
///
/// 如有同名文件：先创建 .backup 副本。
/// 如果任一步骤失败：已移动的文件从目标移回暂存目录，从 .backup 恢复被覆盖的文件。
fn atomic_move_staging_to_src(staging_dir: &Path, output_dir: &str) -> Result<(), WorkflowError> {
    let dest_dir = Path::new(output_dir);
    let mut moved: Vec<(PathBuf, Option<PathBuf>)> = Vec::new(); // (dest, backup)

    let walk_dir = |dir: &Path| -> Result<Vec<PathBuf>, std::io::Error> {
        let mut files = Vec::new();
        visit_dir(dir, &mut files)?;
        Ok(files)
    };

    let staging_files =
        walk_dir(staging_dir).map_err(|e| WorkflowError::Io(format!("遍历暂存目录失败: {}", e)))?;

    for src_path in &staging_files {
        let rel = src_path
            .strip_prefix(staging_dir)
            .map_err(|e| WorkflowError::Io(format!("计算相对路径失败: {}", e)))?;
        let dest_path = dest_dir.join(rel);

        // 创建目标父目录
        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| WorkflowError::Io(format!("创建目录失败: {}", e)))?;
        }

        // 如有同名文件，备份
        let backup = if dest_path.exists() {
            let backup_path = dest_path.with_extension(format!(
                "{}.backup",
                dest_path
                    .extension()
                    .map(|e| format!(".{}", e.to_string_lossy()))
                    .unwrap_or_default()
            ));
            fs::rename(&dest_path, &backup_path).map_err(|e| {
                // 回滚已移动的文件
                rollback_moved(&moved);
                WorkflowError::Io(format!("备份文件失败: {}", e))
            })?;
            Some(backup_path)
        } else {
            None
        };

        // 移动文件
        if let Err(e) = fs::rename(src_path, &dest_path) {
            rollback_moved(&moved);
            return Err(WorkflowError::Io(format!("移动文件失败: {}", e)));
        }
        moved.push((dest_path, backup));
    }

    Ok(())
}

fn visit_dir(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), std::io::Error> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            visit_dir(&path, files)?;
        } else {
            files.push(path);
        }
    }
    Ok(())
}

fn rollback_moved(moved: &[(PathBuf, Option<PathBuf>)]) {
    for (dest, backup) in moved.iter().rev() {
        // 将已移动的文件移回暂存（忽略错误，尽最大努力）
        let _ = fs::remove_file(dest);
        if let Some(ref backup_path) = backup {
            let original = backup_path.with_extension(
                backup_path
                    .extension()
                    .map(|e| {
                        let s = e.to_string_lossy();
                        if s == "backup" {
                            String::new()
                        } else {
                            format!(".{}", s)
                        }
                    })
                    .unwrap_or_default(),
            );
            let _ = fs::rename(backup_path, &original);
        }
    }
}

/// 清理旧的暂存目录
///
/// 保留最近 `keep_last` 个暂存目录，删除 `older_than_days` 天前的。
#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::InMemoryEventEmitter;
    use crate::parser::{OnFail, QualityCheck, QualityGate};
    use crate::QualityCheckResult;
    use std::collections::HashMap;

    // ── evaluate_condition tests ──

    fn make_vars(pairs: &[(&str, &str)]) -> HashMap<String, serde_json::Value> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), serde_json::Value::String(v.to_string())))
            .collect()
    }

    #[test]
    fn eval_eq_match() {
        let vars = make_vars(&[("status", "done")]);
        assert!(WorkflowEngine::evaluate_condition("status == 'done'", &vars));
    }

    #[test]
    fn eval_eq_no_match() {
        let vars = make_vars(&[("status", "pending")]);
        assert!(!WorkflowEngine::evaluate_condition("status == 'done'", &vars));
    }

    #[test]
    fn eval_eq_missing_var() {
        let vars = make_vars(&[]);
        assert!(!WorkflowEngine::evaluate_condition("missing == 'x'", &vars));
    }

    #[test]
    fn eval_ne_match() {
        let vars = make_vars(&[("status", "pending")]);
        assert!(WorkflowEngine::evaluate_condition("status != 'done'", &vars));
    }

    #[test]
    fn eval_ne_no_match() {
        let vars = make_vars(&[("status", "done")]);
        assert!(!WorkflowEngine::evaluate_condition("status != 'done'", &vars));
    }

    #[test]
    fn eval_ne_missing_var_true() {
        let vars = make_vars(&[]);
        assert!(WorkflowEngine::evaluate_condition("missing != 'x'", &vars));
    }

    #[test]
    fn eval_ge_string_number() {
        let vars = make_vars(&[("count", "5")]);
        assert!(WorkflowEngine::evaluate_condition("count >= 3", &vars));
        assert!(!WorkflowEngine::evaluate_condition("count >= 10", &vars));
    }

    #[test]
    fn eval_ge_json_number() {
        let mut vars = HashMap::new();
        vars.insert("count".into(), serde_json::Value::Number(serde_json::Number::from(5)));
        assert!(WorkflowEngine::evaluate_condition("count >= 3", &vars));
        assert!(!WorkflowEngine::evaluate_condition("count >= 10", &vars));
    }

    #[test]
    fn eval_le_string_number() {
        let vars = make_vars(&[("count", "3")]);
        assert!(WorkflowEngine::evaluate_condition("count <= 5", &vars));
        assert!(!WorkflowEngine::evaluate_condition("count <= 1", &vars));
    }

    #[test]
    fn eval_boolean_true() {
        let mut vars = HashMap::new();
        vars.insert("flag".into(), serde_json::Value::Bool(true));
        assert!(WorkflowEngine::evaluate_condition("flag", &vars));
    }

    #[test]
    fn eval_boolean_false() {
        let mut vars = HashMap::new();
        vars.insert("flag".into(), serde_json::Value::Bool(false));
        assert!(!WorkflowEngine::evaluate_condition("flag", &vars));
    }

    #[test]
    fn eval_string_true_literal() {
        let vars = make_vars(&[("flag", "true")]);
        assert!(WorkflowEngine::evaluate_condition("flag", &vars));
    }

    #[test]
    fn eval_unknown_operator_returns_false() {
        let vars = make_vars(&[("x", "1")]);
        assert!(!WorkflowEngine::evaluate_condition("x > 0", &vars));
    }

    #[test]
    fn eval_ge_invalid_threshold() {
        let vars = make_vars(&[("count", "5")]);
        assert!(WorkflowEngine::evaluate_condition(
            "count >= notanumber",
            &vars
        ));
    }

    // ── next_after tests ──

    fn make_stage(name: &str) -> crate::parser::StageDefinition {
        crate::parser::StageDefinition {
            name: name.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn next_after_first() {
        let stages = vec![make_stage("a"), make_stage("b"), make_stage("c")];
        assert_eq!(
            WorkflowEngine::next_after(&stages, "a"),
            Some("b".to_string())
        );
    }

    #[test]
    fn next_after_middle() {
        let stages = vec![make_stage("a"), make_stage("b"), make_stage("c")];
        assert_eq!(
            WorkflowEngine::next_after(&stages, "b"),
            Some("c".to_string())
        );
    }

    #[test]
    fn next_after_last_returns_none() {
        let stages = vec![make_stage("a"), make_stage("b")];
        assert_eq!(WorkflowEngine::next_after(&stages, "b"), None);
    }

    #[test]
    fn next_after_not_found_returns_none() {
        let stages = vec![make_stage("a")];
        assert_eq!(WorkflowEngine::next_after(&stages, "z"), None);
    }

    #[test]
    fn next_after_empty_stages() {
        let stages: Vec<crate::parser::StageDefinition> = vec![];
        assert_eq!(WorkflowEngine::next_after(&stages, "a"), None);
    }

    // ── truncate_str tests ──

    #[test]
    fn truncate_short_string() {
        assert_eq!(truncate_str("hello", 100), "hello");
    }

    #[test]
    fn truncate_long_string() {
        let result = truncate_str("abcdefghij", 5);
        assert!(result.starts_with("abcde"));
        assert!(result.contains("截断"));
    }

    #[test]
    fn truncate_at_boundary() {
        let s = "hello";
        assert_eq!(truncate_str(s, 5), "hello");
    }

    #[test]
    fn truncate_empty_string() {
        assert_eq!(truncate_str("", 10), "");
    }

    // ── parse_structured_summary tests ──

    #[test]
    fn parse_valid_json_summary() {
        let response = "some text\n```json\n{\"summary\": \"done work\", \"files_changed\": [\"a.rs\", \"b.rs\"]}\n```\nmore text";
        let (summary, files) = WorkflowEngine::parse_structured_summary(response);
        assert_eq!(summary, Some("done work".to_string()));
        assert_eq!(files, vec!["a.rs", "b.rs"]);
    }

    #[test]
    fn parse_no_json_block() {
        let (summary, files) = WorkflowEngine::parse_structured_summary("plain text");
        assert_eq!(summary, None);
        assert!(files.is_empty());
    }

    #[test]
    fn parse_json_missing_fields() {
        let response = "```json\n{\"other\": \"value\"}\n```";
        let (summary, files) = WorkflowEngine::parse_structured_summary(response);
        assert_eq!(summary, None);
        assert!(files.is_empty());
    }

    // ── format_gate_errors tests ──

    #[test]
    fn format_gate_errors_all_passed() {
        let engine = WorkflowEngine::new(Arc::new(InMemoryEventEmitter::new()));
        let result = GateRunResult {
            passed: true,
            checks: vec![QualityCheckResult {
                cmd: "cargo build".into(),
                passed: true,
                exit_code: Some(0),
                stdout: "Compiling...".into(),
                stderr: String::new(),
                duration_ms: 100,
            }],
        };
        let summary = engine.format_gate_errors(&result);
        assert!(!summary.contains('❌'));
    }

    #[test]
    fn format_gate_errors_some_failed() {
        let engine = WorkflowEngine::new(Arc::new(InMemoryEventEmitter::new()));
        let result = GateRunResult {
            passed: false,
            checks: vec![QualityCheckResult {
                cmd: "cargo test".into(),
                passed: false,
                exit_code: Some(1),
                stdout: String::new(),
                stderr: "test failed".into(),
                duration_ms: 200,
            }],
        };
        let summary = engine.format_gate_errors(&result);
        assert!(summary.contains('❌'));
        assert!(summary.contains("cargo test"));
        assert!(summary.contains("test failed"));
    }

    // ── resolve_quality_gate tests ──

    #[test]
    fn resolve_quality_gate_with_template() {
        let engine = WorkflowEngine::new(Arc::new(InMemoryEventEmitter::new()));
        let gate = QualityGate {
            checks: vec![],
            on_fail: OnFail::Retry,
            max_retries: 3,
            template: Some("rust_default".into()),
        };
        let resolved = engine.resolve_quality_gate(&gate);
        assert_eq!(resolved.checks.len(), 3);
        assert!(resolved.checks.iter().any(|c| c.cmd == "cargo build"));
        assert!(resolved.checks.iter().any(|c| c.cmd == "cargo test"));
        assert!(resolved.checks.iter().any(|c| c.cmd == "cargo clippy -- -D warnings"));
        assert!(resolved.template.is_none());
    }

    #[test]
    fn resolve_quality_gate_unknown_template_falls_back() {
        let engine = WorkflowEngine::new(Arc::new(InMemoryEventEmitter::new()));
        let gate = QualityGate {
            checks: vec![QualityCheck {
                cmd: "echo hi".into(),
                timeout: 10,
            }],
            on_fail: OnFail::Fail,
            max_retries: 1,
            template: Some("nonexistent".into()),
        };
        let resolved = engine.resolve_quality_gate(&gate);
        assert_eq!(resolved.checks.len(), 1);
        assert_eq!(resolved.checks[0].cmd, "echo hi");
    }

    #[test]
    fn resolve_quality_gate_no_template() {
        let engine = WorkflowEngine::new(Arc::new(InMemoryEventEmitter::new()));
        let gate = QualityGate {
            checks: vec![QualityCheck {
                cmd: "make test".into(),
                timeout: 60,
            }],
            on_fail: OnFail::Retry,
            max_retries: 2,
            template: None,
        };
        let resolved = engine.resolve_quality_gate(&gate);
        assert_eq!(resolved.checks.len(), 1);
        assert_eq!(resolved.checks[0].cmd, "make test");
    }

    // ── load_quality_gate_template tests ──

    #[test]
    fn load_known_templates() {
        assert!(WorkflowEngine::load_quality_gate_template("rust_default").is_some());
        assert!(WorkflowEngine::load_quality_gate_template("typescript_default").is_some());
        assert!(WorkflowEngine::load_quality_gate_template("python_default").is_some());
        assert!(WorkflowEngine::load_quality_gate_template("go_default").is_some());
        assert!(WorkflowEngine::load_quality_gate_template("docker_default").is_some());
    }

    #[test]
    fn load_unknown_template() {
        assert!(WorkflowEngine::load_quality_gate_template("made_up_template").is_none());
    }

    // ── atomic_move_staging_to_src tests ──

    #[test]
    fn atomic_move_creates_dest_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let staging = tmp.path().join("staging");
        let dest = tmp.path().join("output");
        fs::create_dir_all(&staging).unwrap();
        fs::write(staging.join("hello.txt"), "world").unwrap();

        atomic_move_staging_to_src(&staging, dest.to_str().unwrap()).unwrap();
        assert!(dest.join("hello.txt").exists());
        assert_eq!(
            fs::read_to_string(dest.join("hello.txt")).unwrap(),
            "world"
        );
    }

    #[test]
    fn atomic_move_backs_up_existing_file() {
        let tmp = tempfile::tempdir().unwrap();
        let staging = tmp.path().join("staging");
        let dest = tmp.path().join("output");
        fs::create_dir_all(&staging).unwrap();
        fs::create_dir_all(&dest).unwrap();
        fs::write(staging.join("file.txt"), "new").unwrap();
        fs::write(dest.join("file.txt"), "old").unwrap();

        atomic_move_staging_to_src(&staging, dest.to_str().unwrap()).unwrap();
        assert_eq!(fs::read_to_string(dest.join("file.txt")).unwrap(), "new");

        // 查找备份文件（由原子移动创建）
        let files: Vec<_> = fs::read_dir(&dest)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        let backup = files.iter().find(|n| n.contains("backup"));
        assert!(backup.is_some(), "no backup file found among: {files:?}");
        let backup_path = dest.join(backup.unwrap());
        assert_eq!(fs::read_to_string(&backup_path).unwrap(), "old");
    }

    #[test]
    fn atomic_move_empty_staging() {
        let tmp = tempfile::tempdir().unwrap();
        let staging = tmp.path().join("staging");
        let dest = tmp.path().join("output");
        fs::create_dir_all(&staging).unwrap();

        atomic_move_staging_to_src(&staging, dest.to_str().unwrap()).unwrap();
        assert!(!dest.exists() || fs::read_dir(&dest).unwrap().next().is_none());
    }

    #[test]
    fn cleanup_old_staging_skips_nonexistent() {
        let tmp = tempfile::tempdir().unwrap();
        let result = cleanup_old_staging_dirs(tmp.path(), 5, 7);
        assert!(result.is_ok());
    }

    #[test]
    fn cleanup_keeps_recent_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let staging_root = tmp.path().join(".nexus-staging");
        fs::create_dir_all(staging_root.join("keep1")).unwrap();
        fs::create_dir_all(staging_root.join("keep2")).unwrap();

        // keep_last=5 + older_than_days=365 → keeps everything
        cleanup_old_staging_dirs(tmp.path(), 5, 365).unwrap();
        assert!(staging_root.join("keep1").exists());
        assert!(staging_root.join("keep2").exists());
    }

    // ── WorkflowEngine constructor tests ──

    #[test]
    fn engine_new_with_collector() {
        let collector = Arc::new(InMemoryEventEmitter::new());
        let engine = WorkflowEngine::new(collector.clone());
        assert!(engine.working_directory.is_none());
        assert!(engine.resume_rx.is_none());
    }

    #[test]
    fn engine_with_working_directory() {
        let collector = Arc::new(InMemoryEventEmitter::new());
        let engine =
            WorkflowEngine::with_working_directory(collector, Some("/tmp/test".into()));
        assert_eq!(engine.working_directory, Some("/tmp/test".to_string()));
    }

    #[test]
    fn engine_with_resume_channel() {
        let collector = Arc::new(InMemoryEventEmitter::new());
        let (_tx, rx) = tokio::sync::mpsc::channel::<String>(1);
        let engine =
            WorkflowEngine::with_resume_channel(collector, None, rx);
        assert!(engine.resume_rx.is_some());
    }

    #[test]
    fn engine_clone_preserves_state() {
        let collector = Arc::new(InMemoryEventEmitter::new());
        let engine =
            WorkflowEngine::with_working_directory(collector, Some("/tmp/test".into()));
        let cloned = engine.clone();
        assert_eq!(cloned.working_directory, Some("/tmp/test".to_string()));
    }

    // ── Condition edge case tests ──

    #[test]
    fn eval_with_double_quoted_value() {
        let vars = make_vars(&[("name", "alice")]);
        assert!(WorkflowEngine::evaluate_condition(
            "name == \"alice\"",
            &vars
        ));
    }

    #[test]
    fn eval_eq_integer_values() {
        let vars = make_vars(&[("iteration", "3")]);
        assert!(WorkflowEngine::evaluate_condition("iteration == '3'", &vars));
        assert!(!WorkflowEngine::evaluate_condition("iteration == '5'", &vars));
    }

    #[test]
    fn eval_with_whitespace_in_condition() {
        let vars = make_vars(&[("x", "hello")]);
        assert!(WorkflowEngine::evaluate_condition("  x   ==   'hello'  ", &vars));
    }

    // ── inject_gate_error_to_state test ──

    #[test]
    fn inject_gate_error_populates_var() {
        let engine = WorkflowEngine::new(Arc::new(InMemoryEventEmitter::new()));
        let state: SharedState = Arc::new(RwLock::new(WorkflowState::new("test_wf")));
        engine.inject_gate_error_to_state(&state, "build", "something broke");
        let s = state.read();
        let val = s.get_var("build_quality_gate_error").unwrap();
        assert_eq!(val.as_str().unwrap(), "something broke");
    }
}

fn cleanup_old_staging_dirs(
    project_dir: &Path,
    keep_last: usize,
    older_than_days: u64,
) -> Result<(), WorkflowError> {
    let staging_root = project_dir.join(".nexus-staging");
    if !staging_root.exists() {
        return Ok(());
    }

    let mut dirs: Vec<_> = fs::read_dir(&staging_root)
        .map_err(|e| WorkflowError::Io(format!("读取暂存目录失败: {}", e)))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();

    // 按修改时间降序排列（最新的在前）
    dirs.sort_by(|a, b| {
        let ta = a.metadata().ok().and_then(|m| m.modified().ok());
        let tb = b.metadata().ok().and_then(|m| m.modified().ok());
        tb.cmp(&ta)
    });

    let cutoff =
        std::time::SystemTime::now() - std::time::Duration::from_secs(older_than_days * 86400);

    for (i, entry) in dirs.iter().enumerate() {
        let should_delete = i >= keep_last
            || entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .map(|t| t < cutoff)
                .unwrap_or(false);
        if should_delete {
            let _ = fs::remove_dir_all(entry.path());
        }
    }

    Ok(())
}
