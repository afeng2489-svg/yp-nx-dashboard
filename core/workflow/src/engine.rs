//! 工作流引擎
//!
//! 工作流的核心执行引擎。

use parking_lot::RwLock;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;

use crate::events::{EventEmitter, WorkflowEvent};
use crate::parser::{OnFail, QualityGate, StageType, WorkflowError as ParserWorkflowError};
use crate::{
    AgentState, AgentStatus, QualityCheckResult, QualityGateResult, StageOutput,
    WorkflowDefinition, WorkflowState, WorkflowStatus,
};
use regex::Regex;

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
    model_router_fn: Option<Arc<dyn Fn(&str) -> Option<String> + Send + Sync>>,
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
        }
    }

    /// 注入 RAG provider
    pub fn set_rag_provider(&mut self, provider: Arc<dyn crate::watcher::RagProvider>) {
        self.rag_provider = Some(provider);
    }

    /// 注入模型路由回调
    pub fn set_model_router_fn(&mut self, f: Arc<dyn Fn(&str) -> Option<String> + Send + Sync>) {
        self.model_router_fn = Some(f);
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

        // ── 新执行循环：支持条件跳转、user_input 暂停、loop ──
        let mut current_stage_name: Option<String> =
            workflow.stages.first().map(|s| s.name.clone());

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

            // 根据 stage 类型分发执行
            let outputs = match stage.stage_type {
                StageType::UserInput => {
                    let question = stage.question.clone().unwrap_or_default();
                    let options = stage.options.clone();
                    let output_var = stage.output_var.clone().unwrap_or_default();

                    self.event_emitter.emit(WorkflowEvent::WorkflowPaused {
                        execution_id: state.read().execution_id,
                        stage_name: stage.name.clone(),
                        question: question.clone(),
                        options: options
                            .iter()
                            .map(|o| (o.label.clone(), o.value.clone()))
                            .collect(),
                    });

                    // 等待 resume_tx channel 收到用户选择
                    let chosen_value = if let Some(ref resume_rx) = self.resume_rx {
                        let mut rx = resume_rx.lock().await;
                        rx.recv().await.unwrap_or_default()
                    } else {
                        // 单元测试时没有 channel，用第一个选项的 value 作为默认
                        stage
                            .options
                            .first()
                            .map(|o| o.value.clone())
                            .unwrap_or_default()
                    };

                    if !output_var.is_empty() {
                        state
                            .write()
                            .set_var(&output_var, serde_json::Value::String(chosen_value.clone()));
                    }

                    vec![StageOutput {
                        path: format!("user_input://{}", stage.name),
                        content: Some(chosen_value),
                        agent_id: None,
                    }]
                }

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

                StageType::Agent => {
                    // 首次执行 stage
                    let initial_result = self.execute_stage(&state, &stage, &workflow.agents).await;

                    let (outputs, quality_gate_result) = match initial_result {
                        Ok(out) => {
                            self.run_quality_gate_loop(&state, &stage, &workflow.agents, out)
                                .await?
                        }
                        Err(e) => {
                            self.handle_stage_failure(e, &state, &stage, &workflow)
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
                let mut escalated_stage = stage.clone();
                escalated_stage.model = Some(escalate_model.clone());
                for attempt in 1..=policy.escalate_retries {
                    tracing::warn!(
                        "[FailRecovery] stage='{}' 升级模型 {} 重试 {}/{}",
                        stage.name,
                        escalate_model,
                        attempt,
                        policy.escalate_retries
                    );
                    self.emit_model_escalation(
                        state,
                        &stage.name,
                        Some(escalate_model),
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

                let rag_config = stage.rag.clone();
                let model = stage.model.clone();
                handles.push(tokio::spawn(async move {
                    engine
                        .execute_agent(
                            &state_clone,
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
                    .execute_agent(state, agent, stage.rag.as_ref(), stage.model.as_deref())
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

    /// 执行单个智能体
    async fn execute_agent(
        &self,
        state: &SharedState,
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

        // 构建 prompt（Claude CLI 格式）
        let full_prompt = format!(
            "{}\n\n<system>\n你扮演 {}. 请仔细遵循你的指示。\n</system>\n\n<user>\n{}{}\n</user>",
            auto_yes_prefix, agent.role, resolved_prompt, rag_context
        );

        // 模型路由：stage 未指定 model 时，用路由器自动选择
        let routed_model;
        let effective_model = if model_override.is_some() {
            model_override
        } else if let Some(ref router_fn) = self.model_router_fn {
            routed_model = router_fn(&resolved_prompt);
            if let Some(ref m) = routed_model {
                tracing::info!("[ModelRouter] agent='{}' → {}", agent.id, m);
            }
            routed_model.as_deref()
        } else {
            None
        };

        // 通过 Claude CLI 执行
        let output = self.call_claude_cli(&full_prompt, effective_model).await;

        match output {
            Ok(cli_result) => {
                let response = cli_result.text;
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
                if cli_result.input_tokens > 0 || cli_result.output_tokens > 0 {
                    self.event_emitter.emit(WorkflowEvent::AgentTokenUsage {
                        execution_id,
                        agent_id: agent.id.clone(),
                        input_tokens: cli_result.input_tokens,
                        output_tokens: cli_result.output_tokens,
                    });
                }

                self.event_emitter.emit(WorkflowEvent::AgentCompleted {
                    execution_id,
                    agent_id: agent.id.clone(),
                    output: response.clone(),
                });

                Ok(vec![StageOutput {
                    path: format!("agent://{}/output", agent.id),
                    content: Some(response),
                    agent_id: Some(agent.id.clone()),
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
        let claude_bin = std::env::var("CLAUDE_BIN")
            .or_else(|_| std::env::var("CLAUDE_CLI_PATH_OVERRIDE"))
            .or_else(|_| {
                let candidates = if cfg!(target_os = "windows") {
                    vec![
                        "claude.cmd".to_string(),
                        "claude.exe".to_string(),
                        "claude".to_string(),
                    ]
                } else {
                    vec![
                        "/opt/homebrew/bin/claude".to_string(),
                        "/usr/local/bin/claude".to_string(),
                        "claude".to_string(),
                    ]
                };
                for c in &candidates {
                    if std::path::Path::new(c).exists() {
                        return Ok(c.clone());
                    }
                }
                Err(std::env::VarError::NotPresent)
            })
            .unwrap_or_else(|_| "claude".to_string());
        let mut cmd = if cfg!(target_os = "windows") && claude_bin.ends_with(".js") {
            let mut c = Command::new("node");
            c.arg(&claude_bin);
            c
        } else {
            Command::new(&claude_bin)
        };
        cmd.args([
            "-p",
            "--dangerously-skip-permissions",
            "--output-format",
            "stream-json",
        ]);
        if let Some(m) = model {
            cmd.args(["--model", m]);
        }
        cmd.arg(prompt);

        if let Some(ref dir) = self.working_directory {
            cmd.current_dir(dir);
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let child = cmd.spawn().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                WorkflowError::Execution(format!(
                    "未找到 Claude Code CLI 可执行文件 (路径: {}).\n\
                    请先安装：npm install -g @anthropic-ai/claude-code\n\
                    或在「AI 设置」页面手动指定 Claude CLI 路径。\n\
                    底层错误: {}",
                    claude_bin, e
                ))
            } else {
                WorkflowError::Execution(format!("启动 Claude CLI 失败 ({}): {}", claude_bin, e))
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
