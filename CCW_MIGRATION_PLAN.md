# CCW 工作流迁移方案

> 将 Claude-Code-Workflow (CCW) 的所有工作流能力迁移到 yp-nx-dashboard 原生实现。
> 分阶段交付，每阶段独立可验证，不破坏任何现有功能。

---

## 阅读须知

**每个阶段的结构：**
1. 改哪些文件（列出完整路径）
2. 每个文件改什么（贴出"找到这段代码→替换成这段代码"）
3. 验证命令（运行什么命令证明改好了）
4. 每个工作流模块附带完整 AI 提示词，可直接粘贴给 Claude Code 执行

**路径约定：**
- 项目根目录 = `/Users/Zhuanz/Desktop/yp-nx-dashboard`
- CCW 参考目录 = `/Users/Zhuanz/Claude-Code-Workflow`

---

# P0：引擎扩展

> ⚠️ **必须先完成 P0，之后所有模块才能运行。P0 是一次性改动，之后不再动引擎代码。**
> P0 全程向后兼容——现有工作流一行不用改。

---

## P0 第 1 步：给 `parser.rs` 加三个新结构体和新字段

**文件路径：** `core/workflow/src/parser.rs`

**操作：找到下面这段代码（第 96 行附近）**

```rust
/// 智能体定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    /// 唯一智能体 ID
    pub id: String,
    /// 智能体角色
    pub role: String,
    /// 使用的模型
    pub model: String,
    /// 系统提示词
    pub prompt: String,
    /// 依赖 (其他智能体 ID)
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// 附加配置
    #[serde(default)]
    pub config: AgentConfig,
}
```

**替换成：**

```rust
/// 变量提取规则
/// agent 执行完后，用正则从输出中提取变量写入 WorkflowState
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarExtraction {
    /// 写入 state 的变量名
    pub name: String,
    /// 正则表达式，第一个捕获组为变量值
    /// 例：pattern: "EXTRACT:confidence=([0-9.]+)"
    pub pattern: String,
}

/// 智能体定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    /// 唯一智能体 ID
    pub id: String,
    /// 智能体角色
    pub role: String,
    /// 使用的模型
    pub model: String,
    /// 系统提示词
    pub prompt: String,
    /// 依赖 (其他智能体 ID)
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// 附加配置
    #[serde(default)]
    pub config: AgentConfig,
    /// 从输出中提取变量（为空则不提取，完全向后兼容）
    #[serde(default)]
    pub extract_vars: Vec<VarExtraction>,
}
```

---

**操作：找到下面这段代码（第 140 行附近）**

```rust
/// 阶段定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageDefinition {
    /// 阶段名称
    pub name: String,
    /// 此阶段的智能体
    #[serde(default)]
    pub agents: Vec<String>,
    /// 是否并行运行智能体
    #[serde(default = "default_false")]
    pub parallel: bool,
    /// 此阶段的预期输出
    #[serde(default)]
    pub output: Vec<OutputDefinition>,
    /// 即使智能体失败也继续
    #[serde(default)]
    pub continue_on_error: bool,
}
```

**替换成：**

```rust
/// Stage 类型
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StageType {
    /// 原有类型：运行 agents（默认，向后兼容）
    #[default]
    Agent,
    /// 新增：暂停等待用户在前端做选择
    UserInput,
    /// 新增：循环执行 body_stages 直到 break_condition 为 true
    Loop,
}

/// 阶段跳转规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageTransition {
    /// 跳转条件表达式，引用 state 变量
    /// 格式：  "变量名 == '字符串'"  或  "变量名 >= 数字"
    /// 为空时作为兜底 fallback，无条件跳转
    #[serde(default)]
    pub condition: Option<String>,
    /// 跳转目标 stage 的 name 字段值
    pub goto: String,
}

/// 用户输入选项（配合 stage_type: user_input 使用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInputOption {
    /// 展示给用户的文字
    pub label: String,
    /// 写入 output_var 的值
    pub value: String,
    /// 选项说明（可选）
    #[serde(default)]
    pub description: Option<String>,
}

/// 阶段定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageDefinition {
    /// 阶段名称（在 next.goto 中通过此名称引用）
    pub name: String,
    /// Stage 类型（默认 agent，向后兼容）
    #[serde(default)]
    pub stage_type: StageType,
    /// 此阶段的智能体（stage_type=agent 时使用）
    #[serde(default)]
    pub agents: Vec<String>,
    /// 是否并行运行智能体
    #[serde(default = "default_false")]
    pub parallel: bool,
    /// 此阶段的预期输出
    #[serde(default)]
    pub output: Vec<OutputDefinition>,
    /// 即使智能体失败也继续
    #[serde(default)]
    pub continue_on_error: bool,
    /// 条件跳转规则（为空时按 stages 数组顺序执行，向后兼容）
    #[serde(default)]
    pub next: Vec<StageTransition>,

    // ---- user_input 专用字段 ----
    /// 展示给用户的问题文本
    #[serde(default)]
    pub question: Option<String>,
    /// 选项列表
    #[serde(default)]
    pub options: Vec<UserInputOption>,
    /// 用户选择结果写入的变量名
    #[serde(default)]
    pub output_var: Option<String>,

    // ---- loop 专用字段 ----
    /// 循环退出条件（引用 state 变量，格式同 StageTransition.condition）
    #[serde(default)]
    pub break_condition: Option<String>,
    /// 每次循环执行的 stage 名称列表
    #[serde(default)]
    pub body_stages: Vec<String>,
    /// 最大循环次数（超出后工作流 failed）
    #[serde(default = "default_max_loop")]
    pub max_iterations: usize,
}

fn default_max_loop() -> usize { 10 }
```

---

**验证 P0 第 1 步：**

```bash
cd /Users/Zhuanz/Desktop/yp-nx-dashboard
cargo check -p nexus-workflow 2>&1
```

期望输出：无 error，只有 warning（新字段未使用的 warning 正常）。

---

## P0 第 2 步：给 `Cargo.toml` 加 `regex` 依赖

**文件路径：** `core/workflow/Cargo.toml`

**操作：找到：**

```toml
nexus-ai = { path = "../ai" }
```

**在这一行下面插入：**

```toml
regex = "1"
```

**验证：**

```bash
cd /Users/Zhuanz/Desktop/yp-nx-dashboard
cargo check -p nexus-workflow 2>&1
```

---

## P0 第 3 步：改造 `engine.rs` 的执行循环

这是 P0 最核心的改动。改完后引擎支持：条件跳转、变量提取、用户输入暂停、循环。

**文件路径：** `core/workflow/src/engine.rs`

**操作 3-A：在文件顶部 `use` 区域找到：**

```rust
use crate::{WorkflowDefinition, WorkflowState, WorkflowStatus, StageOutput, AgentState, AgentStatus};
use crate::events::{EventEmitter, WorkflowEvent};
use crate::parser::WorkflowError as ParserWorkflowError;
use nexus_ai::ChatMessage;
```

**替换成：**

```rust
use crate::{WorkflowDefinition, WorkflowState, WorkflowStatus, StageOutput, AgentState, AgentStatus};
use crate::events::{EventEmitter, WorkflowEvent};
use crate::parser::{WorkflowError as ParserWorkflowError, StageType, StageDefinition};
use nexus_ai::ChatMessage;
use regex::Regex;
```

---

**操作 3-B：找到 `execute` 方法中的阶段循环（大约第 50 行）：**

```rust
        // 顺序执行阶段
        for (stage_idx, stage) in workflow.stages.iter().enumerate() {
            if state.read().should_stop() {
                break;
            }

            {
                let s = state.read();
                self.event_emitter.emit(WorkflowEvent::StageStarted {
                    execution_id: s.execution_id,
                    stage_name: stage.name.clone(),
                    stage_index: stage_idx,
                });
            }

            // 执行阶段（带 on_error 重试逻辑）
            let outputs = match self.execute_stage(&state, stage, &workflow.agents).await {
```

**把整个 `for` 循环（从 `// 顺序执行阶段` 到 `s.stage_results.last().unwrap().outputs.clone(),` 之后的 `});` ）全部替换成下面的代码：**

```rust
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
                    return Err(WorkflowError::Execution(
                        format!("找不到 stage: {}", stage_name)
                    ));
                }
            };

            {
                let s = state.read();
                self.event_emitter.emit(WorkflowEvent::StageStarted {
                    execution_id: s.execution_id,
                    stage_name: stage.name.clone(),
                    stage_index: stage_idx.unwrap_or(0),
                });
            }

            // 根据 stage 类型分发执行
            let outputs = match stage.stage_type {
                StageType::UserInput => {
                    // 通过 event 通知前端暂停，等待用户输入
                    let question = stage.question.clone().unwrap_or_default();
                    let options = stage.options.clone();
                    let output_var = stage.output_var.clone().unwrap_or_default();

                    self.event_emitter.emit(WorkflowEvent::WorkflowPaused {
                        execution_id: state.read().execution_id,
                        stage_name: stage.name.clone(),
                        question: question.clone(),
                        options: options.iter().map(|o| (o.label.clone(), o.value.clone())).collect(),
                    });

                    // 等待 resume_tx channel 收到用户选择
                    let chosen_value = if let Some(ref resume_rx) = self.resume_rx {
                        let mut rx = resume_rx.lock().await;
                        rx.recv().await.unwrap_or_default()
                    } else {
                        // 单元测试时没有 channel，用第一个选项的 value 作为默认
                        stage.options.first().map(|o| o.value.clone()).unwrap_or_default()
                    };

                    // 写入变量
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
                    }]
                }

                StageType::Loop => {
                    // 循环执行 body_stages 直到 break_condition 满足
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

                        // 执行 body_stages 中的每个 stage
                        for body_stage_name in &stage.body_stages {
                            let body_idx = workflow.stages.iter().position(|s| &s.name == body_stage_name);
                            let body_stage = match body_idx {
                                Some(idx) => workflow.stages[idx].clone(),
                                None => return Err(WorkflowError::Execution(
                                    format!("Loop body 找不到 stage: {}", body_stage_name)
                                )),
                            };
                            let body_outputs = self.execute_stage(&state, &body_stage, &workflow.agents).await?;
                            loop_outputs.extend(body_outputs);
                        }

                        // 检查退出条件
                        if let Some(ref cond) = stage.break_condition {
                            if Self::evaluate_condition(cond, &state.read().variables) {
                                break;
                            }
                        } else {
                            break; // 没有条件则只跑一次
                        }
                    }
                    loop_outputs
                }

                StageType::Agent => {
                    // 原有执行逻辑（带 on_error 重试）
                    match self.execute_stage(&state, &stage, &workflow.agents).await {
                        Ok(outputs) => outputs,
                        Err(e) => {
                            if let Some(ref error_handler) = workflow.on_error {
                                if error_handler.retry {
                                    let mut last_err = e;
                                    let mut retry_result = None;
                                    for attempt in 1..=error_handler.max_retries {
                                        tracing::warn!(
                                            "Stage '{}' 失败，重试 {}/{}",
                                            stage.name, attempt, error_handler.max_retries
                                        );
                                        match self.execute_stage(&state, &stage, &workflow.agents).await {
                                            Ok(outputs) => {
                                                retry_result = Some(outputs);
                                                break;
                                            }
                                            Err(e) => { last_err = e; }
                                        }
                                    }
                                    match retry_result {
                                        Some(outputs) => outputs,
                                        None => return Err(last_err),
                                    }
                                } else {
                                    return Err(e);
                                }
                            } else {
                                return Err(e);
                            }
                        }
                    }
                }
            };

            {
                let mut s = state.write();
                s.record_stage(&stage.name, outputs.clone());
            }

            {
                let s = state.read();
                self.event_emitter.emit(WorkflowEvent::StageCompleted {
                    execution_id: s.execution_id,
                    stage_name: stage.name.clone(),
                    outputs: outputs.clone(),
                });
            }

            // ── 计算下一个 stage ──
            // loop stage 执行完后继续往下走（不再回到自身）
            if stage.stage_type == StageType::Loop {
                current_stage_name = Self::next_after(&workflow.stages, &stage.name);
            } else if stage.next.is_empty() {
                // 没有 next 规则：按数组顺序走（向后兼容）
                current_stage_name = Self::next_after(&workflow.stages, &stage.name);
            } else {
                // 有 next 规则：按条件跳转
                let vars = state.read().variables.clone();
                let mut jumped = false;
                for transition in &stage.next {
                    let should_jump = match &transition.condition {
                        None => true, // 无条件 fallback
                        Some(cond) => Self::evaluate_condition(cond, &vars),
                    };
                    if should_jump {
                        current_stage_name = Some(transition.goto.clone());
                        jumped = true;
                        break;
                    }
                }
                if !jumped {
                    // 所有条件都不满足，按顺序走
                    current_stage_name = Self::next_after(&workflow.stages, &stage.name);
                }
            }
        }
```

---

**操作 3-C：在 `execute` 方法结束的 `Ok(WorkflowResult { ... })` 之前，`WorkflowEngine` 的 `impl` 块里添加两个辅助方法：**

找到：

```rust
    /// 执行单个阶段
    async fn execute_stage(
```

在这一行**之前**插入：

```rust
    /// 返回 stages 数组中 current_name 之后的下一个 stage 名（没有则 None 表示结束）
    fn next_after(stages: &[crate::parser::StageDefinition], current_name: &str) -> Option<String> {
        stages.iter()
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

        // == 字符串比较
        if let Some(idx) = cond.find(" == ") {
            let var_name = cond[..idx].trim();
            let expected = cond[idx + 4..].trim().trim_matches('\'').trim_matches('"');
            return vars.get(var_name)
                .and_then(|v| v.as_str())
                .map(|v| v == expected)
                .unwrap_or(false);
        }

        // != 字符串比较
        if let Some(idx) = cond.find(" != ") {
            let var_name = cond[..idx].trim();
            let expected = cond[idx + 4..].trim().trim_matches('\'').trim_matches('"');
            return vars.get(var_name)
                .and_then(|v| v.as_str())
                .map(|v| v != expected)
                .unwrap_or(true);
        }

        // >= 数字比较
        if let Some(idx) = cond.find(" >= ") {
            let var_name = cond[..idx].trim();
            let threshold: f64 = cond[idx + 4..].trim().parse().unwrap_or(0.0);
            return vars.get(var_name)
                .and_then(|v| v.as_str().and_then(|s| s.parse::<f64>().ok())
                    .or_else(|| v.as_f64()))
                .map(|v| v >= threshold)
                .unwrap_or(false);
        }

        // <= 数字比较
        if let Some(idx) = cond.find(" <= ") {
            let var_name = cond[..idx].trim();
            let threshold: f64 = cond[idx + 4..].trim().parse().unwrap_or(0.0);
            return vars.get(var_name)
                .and_then(|v| v.as_str().and_then(|s| s.parse::<f64>().ok())
                    .or_else(|| v.as_f64()))
                .map(|v| v <= threshold)
                .unwrap_or(false);
        }

        // 布尔变量直接判断：变量名 == 'true'
        if let Some(v) = vars.get(cond) {
            return v.as_str().map(|s| s == "true").unwrap_or(false)
                || v.as_bool().unwrap_or(false);
        }

        false
    }

```

---

**操作 3-D：给 `execute_agent` 方法加变量提取逻辑**

找到 `execute_agent` 方法中的：

```rust
            Ok(response) => {
                agent_state.status = AgentStatus::Completed;
                agent_state.last_message = Some(response.clone());
                agent_state.updated_at = chrono::Utc::now();

                // 写回完成状态
                state.write().update_agent(&agent.id, agent_state);

                self.event_emitter.emit(WorkflowEvent::AgentCompleted {
```

**替换成：**

```rust
            Ok(response) => {
                agent_state.status = AgentStatus::Completed;
                agent_state.last_message = Some(response.clone());
                agent_state.updated_at = chrono::Utc::now();

                // ── 变量提取：从输出中提取变量写入 state ──
                for extraction in &agent.extract_vars {
                    if let Ok(re) = Regex::new(&extraction.pattern) {
                        if let Some(cap) = re.captures(&response) {
                            if let Some(val) = cap.get(1) {
                                state.write().set_var(
                                    &extraction.name,
                                    serde_json::Value::String(val.as_str().to_string()),
                                );
                                tracing::debug!(
                                    "变量提取: {} = {}",
                                    extraction.name,
                                    val.as_str()
                                );
                            }
                        }
                    }
                }

                // 写回完成状态
                state.write().update_agent(&agent.id, agent_state);

                self.event_emitter.emit(WorkflowEvent::AgentCompleted {
```

---

**操作 3-E：给 `WorkflowEngine` 结构体加 `resume_rx` 字段**

找到：

```rust
/// 工作流执行引擎
pub struct WorkflowEngine {
    /// 事件发射器
    event_emitter: Arc<dyn EventEmitter>,
    /// 工作目录（用于 Claude CLI --project 参数）
    working_directory: Option<String>,
}
```

**替换成：**

```rust
/// 工作流执行引擎
pub struct WorkflowEngine {
    /// 事件发射器
    event_emitter: Arc<dyn EventEmitter>,
    /// 工作目录（用于 Claude CLI --project 参数）
    working_directory: Option<String>,
    /// user_input stage 用：前端通过此 channel 发回用户选择的值
    resume_rx: Option<Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<String>>>>,
}
```

找到两个构造方法并各加一行 `resume_rx: None,`：

```rust
    pub fn new(event_emitter: Arc<dyn EventEmitter>) -> Self {
        Self {
            event_emitter,
            working_directory: None,
            resume_rx: None,   // ← 加这一行
        }
    }

    pub fn with_working_directory(event_emitter: Arc<dyn EventEmitter>, working_directory: Option<String>) -> Self {
        Self {
            event_emitter,
            working_directory,
            resume_rx: None,   // ← 加这一行
        }
    }
```

再加一个带 resume channel 的构造方法（放在 `with_working_directory` 之后）：

```rust
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
        }
    }
```

同时更新 `Clone` 实现（文件末尾）：

找到：

```rust
impl Clone for WorkflowEngine {
    fn clone(&self) -> Self {
        Self {
            event_emitter: self.event_emitter.clone(),
            working_directory: self.working_directory.clone(),
        }
    }
}
```

**替换成：**

```rust
impl Clone for WorkflowEngine {
    fn clone(&self) -> Self {
        Self {
            event_emitter: self.event_emitter.clone(),
            working_directory: self.working_directory.clone(),
            resume_rx: self.resume_rx.clone(),
        }
    }
}
```

---

## P0 第 4 步：给 `events.rs` 加两个新事件

**文件路径：** `core/workflow/src/events.rs`

找到 `WorkflowEvent` 枚举的最后一个变体（在 `}` 之前），加入：

```rust
    /// user_input stage 触发：工作流暂停等待用户选择
    WorkflowPaused {
        execution_id: uuid::Uuid,
        stage_name: String,
        question: String,
        /// Vec<(展示文字, 值)>
        options: Vec<(String, String)>,
    },
    /// 工作流从暂停中恢复
    WorkflowResumed {
        execution_id: uuid::Uuid,
        stage_name: String,
        chosen_value: String,
    },
```

---

## P0 验证（全部改完后运行）

```bash
cd /Users/Zhuanz/Desktop/yp-nx-dashboard
cargo build -p nexus-workflow 2>&1
```

**期望**：`Finished` 无 error。

**额外验证——写一个测试 YAML，确认旧格式仍然正常：**

```bash
cd /Users/Zhuanz/Desktop/yp-nx-dashboard
cargo test -p nexus-workflow 2>&1
```

**期望**：所有现有测试通过（test_parse_valid_workflow、test_validate_workflow 等）。

---

# P1：迁移 `investigate`（根因调查工作流）

> **依赖：P0 完成**
> **参考文件：** `/Users/Zhuanz/Claude-Code-Workflow/.claude/skills/investigate/phases/`

## 第 1 步：创建工作流 YAML

**新建文件：** `config/workflows/investigate.yaml`

```yaml
name: investigate
version: "1.0"
description: "根因调查 - 5阶段 Iron Law 方法论。复现 bug、收集证据、验证假设、实施修复、生成报告。"

triggers:
  - type: manual
    inputs:
      target:
        type: string
        required: true
        description: "要调查的 bug 描述或相关文件路径"

variables:
  confidence: "0"
  root_cause: ""

agents:
  - id: investigator
    role: 根因调查员
    model: claude-sonnet-4-6
    prompt: |
      你是根因调查员，使用 Iron Law 方法论分析 bug。

      调查目标：{{target}}

      执行步骤：
      1. 解析目标，提取症状/期望行为/触发上下文
      2. 用 Grep、Read 工具定位问题代码
      3. 尝试找到触发条件
      4. 列出所有收集到的证据
      5. 提出假设并评估置信度

      ⚠️ 必须在回复最后两行输出（格式严格）：
      EXTRACT:root_cause=<一句话描述根因>
      EXTRACT:confidence=<0到1之间的小数，例如0.85>
    extract_vars:
      - name: root_cause
        pattern: "EXTRACT:root_cause=(.+)"
      - name: confidence
        pattern: "EXTRACT:confidence=([0-9.]+)"

  - id: pattern-analyzer
    role: 模式分析师
    model: claude-sonnet-4-6
    depends_on: [investigator]
    prompt: |
      基于调查结果进行模式分析。

      当前根因：{{root_cause}}
      置信度：{{confidence}}

      执行：
      1. 分析这类 bug 的常见根因模式
      2. 检查受影响范围（哪些模块可能受到影响）
      3. 判断是否是系统性问题还是单点问题
      4. 如果发现置信度应该调整，在末尾输出新值

      ⚠️ 如果需要调整置信度，最后一行输出：
      EXTRACT:confidence=<新值>
    extract_vars:
      - name: confidence
        pattern: "EXTRACT:confidence=([0-9.]+)"

  - id: hypothesis-tester
    role: 假设验证员
    model: claude-sonnet-4-6
    depends_on: [pattern-analyzer]
    prompt: |
      验证根因假设。

      根因：{{root_cause}}
      置信度：{{confidence}}

      执行：
      1. 设计最小化验证步骤
      2. 通过代码分析确认或推翻假设
      3. 如果推翻，提出新假设

      ⚠️ 最后输出：
      EXTRACT:confidence=<最终置信度>
    extract_vars:
      - name: confidence
        pattern: "EXTRACT:confidence=([0-9.]+)"

  - id: implementer
    role: 修复实施者
    model: claude-sonnet-4-6
    depends_on: [hypothesis-tester]
    prompt: |
      实施修复。

      已验证根因：{{root_cause}}

      执行：
      1. 确定最小化修复范围
      2. 实施代码修改（调用 Edit 工具）
      3. 确保修复不引入新问题
      4. 更新受影响的测试

  - id: reporter
    role: 验证报告员
    model: claude-sonnet-4-6
    depends_on: [implementer]
    prompt: |
      生成最终验证报告。

      执行：
      1. 运行相关测试验证修复
      2. 检查无回归
      3. 输出报告：
         - 根因描述
         - 修复方案
         - 测试验证结果
         - 预防建议

stages:
  - name: investigation
    agents: [investigator]
    next:
      - condition: "confidence >= 0.7"
        goto: pattern-analysis
      - goto: re-investigation

  - name: re-investigation
    agents: [investigator]
    next:
      - goto: pattern-analysis

  - name: pattern-analysis
    agents: [pattern-analyzer]

  - name: hypothesis-testing
    agents: [hypothesis-tester]
    next:
      - condition: "confidence >= 0.5"
        goto: implementation
      - goto: investigation

  - name: implementation
    agents: [implementer]

  - name: verification
    agents: [reporter]
```

## 第 2 步：注册到数据库

启动后端后在终端运行：

```bash
curl -X POST http://localhost:8080/api/v1/workflows \
  -H "Content-Type: application/json" \
  -d @- << 'EOF'
{
  "name": "investigate",
  "version": "1.0",
  "description": "根因调查 - 5阶段 Iron Law 方法论",
  "definition": $(cat /Users/Zhuanz/Desktop/yp-nx-dashboard/config/workflows/investigate.yaml | python3 -c "import sys,json,yaml; print(json.dumps(yaml.safe_load(sys.stdin.read())))")
}
EOF
```

## 第 3 步：验证

1. 打开 Dashboard → Workflows 页面，确认 `investigate` 出现在列表
2. 点击执行，填入 `target = "描述一个真实 bug"`
3. 执行日志应显示：investigation → （若 confidence < 0.7 则 re-investigation →）pattern-analysis → hypothesis-testing → implementation → verification

---

## P1 AI 提示词（直接粘贴给 Claude Code 执行）

```
你需要在 yp-nx-dashboard 项目中实现 investigate 工作流。

项目路径：/Users/Zhuanz/Desktop/yp-nx-dashboard

第 1 步：读取以下文件了解上下文
- 读取 core/workflow/src/parser.rs（了解 WorkflowDefinition / StageDefinition 结构）
- 读取 core/workflow/src/engine.rs（了解执行逻辑）
- 读取 nx_api/src/routes/workflows.rs（了解 API 接口）
- 读取所有 CCW 参考文件：
  /Users/Zhuanz/Claude-Code-Workflow/.claude/skills/investigate/phases/01-root-cause-investigation.md
  /Users/Zhuanz/Claude-Code-Workflow/.claude/skills/investigate/phases/02-pattern-analysis.md
  /Users/Zhuanz/Claude-Code-Workflow/.claude/skills/investigate/phases/03-hypothesis-testing.md
  /Users/Zhuanz/Claude-Code-Workflow/.claude/skills/investigate/phases/04-implementation.md
  /Users/Zhuanz/Claude-Code-Workflow/.claude/skills/investigate/phases/05-verification-report.md

第 2 步：创建 config/workflows/investigate.yaml
- 5 个阶段对应 CCW 的 5 个 phase 文件
- investigation stage 的 agent 末尾必须输出 EXTRACT:confidence=<值>
- 条件：confidence >= 0.7 才进入 pattern-analysis，否则重新 investigation
- hypothesis-testing 阶段：confidence >= 0.5 进入 implementation，否则重回 investigation

第 3 步：确认 config/ 目录存在，如不存在则创建 config/workflows/ 目录

第 4 步：通过后端 API 注册工作流（GET /api/v1/workflows 确认已出现）

验收标准：
- cargo build 无 error
- Dashboard Workflows 页面出现 investigate
- 执行一次，传入 target 参数，能看到阶段依次执行
- confidence 变量在阶段间正确传递（在执行日志中可见）
```

---

# P2：迁移 `review-cycle`（代码审查循环）

> **依赖：P0 完成（需要 user_input stage）**

## AI 提示词

```
你需要在 yp-nx-dashboard 项目中实现 review-cycle 工作流。

项目路径：/Users/Zhuanz/Desktop/yp-nx-dashboard

第 1 步：读取参考文件
- /Users/Zhuanz/Claude-Code-Workflow/.claude/workflow-skills/review-cycle/SKILL.md
- nx_api/src/ws/agent_execution.rs（了解现有 WebSocket 事件结构）
- core/workflow/src/events.rs（了解 WorkflowPaused 事件）

第 2 步：创建 config/workflows/review-cycle.yaml
结构如下：
- stage 1（user_input 类型）：
    question: "请选择审查模式"
    options:
      - { label: "Session 审查（审查当前会话所有变更）", value: "session" }
      - { label: "模块审查（审查指定文件/目录）", value: "module" }
      - { label: "修复审查（审查最近一次修复）", value: "fix" }
    output_var: review_mode
    next:
      - condition: "review_mode == 'session'"  →  goto: session-review
      - condition: "review_mode == 'module'"   →  goto: module-review
      - condition: "review_mode == 'fix'"      →  goto: fix-review

- stage session-review（agent 类型）：
    agent prompt：用 git diff HEAD~1 获取最近变更，逐文件审查
    按 CRITICAL/HIGH/MEDIUM/LOW 分级输出问题
    末尾：EXTRACT:issue_count=<数字>

- stage module-review（agent 类型）：
    agent prompt：读取 {{target_path}} 下所有文件，全面审查
    末尾：EXTRACT:issue_count=<数字>

- stage fix-review（agent 类型）：
    agent prompt：对比修复前后，验证修复是否完整、是否引入新问题
    末尾：EXTRACT:issue_count=<数字>

- stage summary（3个 review stage 都 goto 到这里）：
    agent prompt：汇总所有问题，按优先级排序，生成可操作的修复清单

第 3 步：在 nx_api 侧确认 WorkflowPaused 事件能通过 WebSocket 推送到前端
- 查看 nx_api/src/ws/ 中现有的 WebSocket handler
- 确认 WorkflowEvent::WorkflowPaused 被 handler 处理并序列化发送给客户端

第 4 步：在前端 WorkflowsPage.tsx 中
- 监听 WebSocket 的 workflow_pause 消息
- 弹出选择框展示 options
- 用户选择后发送 workflow_resume 消息回服务端

验收标准：
- 执行 review-cycle，前端弹出选择框
- 选择任意模式，对应的 review stage 执行
- 最终输出汇总报告
```

---

# P3：迁移 `brainstorm`（多角色头脑风暴）

> **依赖：P0 全部完成**

## AI 提示词

```
你需要在 yp-nx-dashboard 项目中实现 brainstorm 工作流。

项目路径：/Users/Zhuanz/Desktop/yp-nx-dashboard

第 1 步：读取所有 CCW 参考文件（必须全部读完再动手）
- /Users/Zhuanz/Claude-Code-Workflow/.claude/skills/brainstorm/phases/01-mode-routing.md
- /Users/Zhuanz/Claude-Code-Workflow/.claude/skills/brainstorm/phases/02-artifacts.md
- /Users/Zhuanz/Claude-Code-Workflow/.claude/skills/brainstorm/phases/03-role-analysis.md
- /Users/Zhuanz/Claude-Code-Workflow/.claude/skills/brainstorm/phases/04-synthesis.md

第 2 步：创建 config/workflows/brainstorm.yaml
完整结构：

stages:
  1. ask-mode（user_input）
     question: "请选择头脑风暴模式"
     options: auto / single-role
     output_var: execution_mode
     next: auto → artifacts，single-role → ask-role

  2. ask-role（user_input）
     question: "请选择分析角色"
     options: system-architect / product-manager / ux-expert / ui-designer / data-architect
     output_var: selected_role
     next: → single-role-analysis

  3. artifacts（agent，auto 模式）
     参考 02-artifacts.md 的逻辑
     生成话题分析框架、选定角色列表
     末尾：EXTRACT:selected_roles=system-architect,product-manager,ux-expert
     next: → parallel-analysis

  4. parallel-analysis（agent，parallel: true）
     包含 5 个 agent（每个角色一个）：
       system-architect-agent / product-manager-agent / ux-expert-agent /
       ui-designer-agent / data-architect-agent
     每个 agent 的 prompt 按对应角色的专业视角深度分析 {{topic}}
     parallel: true（5个 agent 同时执行）
     next: → synthesis

  5. single-role-analysis（agent，single-role 模式）
     agent prompt：以 {{selected_role}} 的专业视角深度分析 {{topic}}
     next: → synthesis

  6. synthesis（agent）
     参考 04-synthesis.md 的逻辑
     整合所有角色输出，识别共识/冲突/互补点
     生成最终建议报告（Markdown 格式，带优先级排序）

第 3 步：验证并行执行
确认 parallel-analysis stage 中 5 个 agent 在执行日志中几乎同时开始（时间戳相差 < 1 秒）

第 4 步：验证变量传递
artifacts stage 输出的内容应该在 synthesis stage 的 prompt 中可见（通过 state.variables）

验收标准：
- auto 模式：6 个 stage 按顺序执行，parallel-analysis 中 5 agent 并行
- single-role 模式：ask-role → single-role-analysis → synthesis
- synthesis 输出完整的多视角分析报告
```

---

# P4：迁移 TDD 工作流

> **依赖：P0 全部完成（需要 loop stage）**

## AI 提示词

```
你需要在 yp-nx-dashboard 中实现两个 TDD 工作流。

项目路径：/Users/Zhuanz/Desktop/yp-nx-dashboard

第 1 步：读取所有参考文件
workflow-tdd-plan 的 7 个 phase 文件：
/Users/Zhuanz/Claude-Code-Workflow/.claude/skills/workflow-tdd-plan/phases/（全部读取）

workflow-test-fix 的 5 个 phase 文件：
/Users/Zhuanz/Claude-Code-Workflow/.claude/skills/workflow-test-fix/phases/（全部读取）

第 2 步：创建 config/workflows/workflow-tdd-plan.yaml
7个阶段：
1. session-discovery：发现当前会话目标，提取要测试的功能点
2. context-gathering：收集代码上下文（相关文件、依赖、现有测试）
3. test-coverage-analysis：分析当前测试覆盖，找出空白
4. conflict-resolution：解决可能的测试冲突（mock vs 真实依赖）
5. tdd-task-generation：生成 TDD 任务列表，按 Red→Green→Refactor 组织
   末尾：EXTRACT:all_tests_pass=false（初始为 false，循环后变 true）
6. tdd-structure-validation：验证测试结构合理性
7. tdd-cycle（loop 类型）：
   break_condition: "all_tests_pass == 'true'"
   max_iterations: 10
   body_stages: [run-tests, analyze-failures, fix-code]

额外 stages（body_stages 内）：
- run-tests：运行测试套件，末尾：EXTRACT:all_tests_pass=true/false
- analyze-failures：分析失败用例，定位原因
- fix-code：修复代码或测试

第 3 步：创建 config/workflows/workflow-test-fix.yaml
5阶段（L0~L3 渐进）：
- L0-static：静态分析（类型错误、编译错误）
  末尾：EXTRACT:l0_pass=true/false
- L1-unit（loop，max_iterations:5）：
  break_condition: "l1_pass == 'true'"
  body_stages: [run-unit-tests, fix-unit]
  run-unit-tests 末尾：EXTRACT:l1_pass=true/false
- L2-integration（loop，max_iterations:3）：
  同上，变量名 l2_pass
- L3-e2e（loop，max_iterations:3）：
  同上，变量名 l3_pass
- coverage-report：生成覆盖率报告，确认 >= 80%

验收标准：
- 测试全部通过时 loop 正确退出
- 超过 max_iterations 时工作流状态变为 failed，错误信息说明超出次数
- L0~L3 按顺序执行，前一层失败不进入下一层
```

---

# P5：Issue 完整闭环

> **依赖：P0 全部完成。工作量最大，分 4 个子步骤独立实施。**

## P5-A：数据模型

## AI 提示词

```
在 yp-nx-dashboard 中添加 Issue 数据模型。

项目路径：/Users/Zhuanz/Desktop/yp-nx-dashboard

第 1 步：读取现有模型了解命名规范
- nx_api/src/models/（读取所有文件了解结构）
- nx_api/src/routes/sessions.rs（参考 session 的 CRUD 实现方式）

第 2 步：创建 nx_api/src/models/issue.rs
字段：
  id: String（UUID）
  title: String
  description: String
  status: IssueStatus（枚举：discovered/planned/queued/executing/completed/failed）
  priority: IssuePriority（枚举：critical/high/medium/low）
  perspectives: Vec<String>（发现此 issue 的视角，如 "bug"/"security"）
  solution: Option<String>（plan 阶段生成）
  depends_on: Vec<String>（issue id 列表，用于 DAG 排序）
  created_at: DateTime<Utc>
  updated_at: DateTime<Utc>

第 3 步：在 SQLite 中创建 issues 表
找到现有的数据库 migration 文件（搜索 CREATE TABLE），按同样方式添加 issues 表。

第 4 步：创建 nx_api/src/routes/issues.rs
实现：
  GET  /api/v1/issues           → 列表（支持 status/priority 过滤）
  POST /api/v1/issues           → 创建
  GET  /api/v1/issues/:id       → 详情
  PUT  /api/v1/issues/:id       → 更新（status/solution/priority）
  DELETE /api/v1/issues/:id     → 删除

第 5 步：在 nx_api/src/routes/mod.rs 注册新路由

验收标准：
- cargo build 无 error
- curl POST /api/v1/issues 创建一条记录成功
- curl GET /api/v1/issues 能查到刚创建的记录
```

---

## P5-B：Issue Discover 工作流

## AI 提示词

```
在 yp-nx-dashboard 中实现 issue-discover 工作流。

项目路径：/Users/Zhuanz/Desktop/yp-nx-dashboard

第 1 步：读取参考文件
/Users/Zhuanz/Claude-Code-Workflow/.claude/commands/issue/discover.md（完整读取）

第 2 步：创建 config/workflows/issue/issue-discover.yaml
结构：
- stage 1（user_input）：选择要扫描的视角（多选逻辑：先显示 8 个视角，用户确认）
  视角：bug / ux / security / performance / test-gaps / code-quality / maintainability / best-practices

- stage 2（agent，parallel: true）：8 个视角 agent 并行扫描目标路径 {{target_path}}
  每个 agent 专注自己的视角，输出 JSON 格式问题列表：
  [{"title":"...","description":"...","severity":"high","file":"...","line":0}]

- stage 3（agent）：汇总所有视角的发现，去重，生成统一的 issue 列表

- stage 4（agent）：将 issue 列表通过 API 写入数据库
  调用 POST /api/v1/issues 为每个 issue 创建记录

验收标准：
- 传入 target_path，执行后在 Dashboard Tasks 页面出现新发现的 issues
- 每个 issue 有正确的 status=discovered、priority 和 perspectives 字段
```

---

## P5-C：Issue Plan + Queue + Execute 工作流

## AI 提示词

```
在 yp-nx-dashboard 中实现 issue-plan、issue-queue、issue-execute 三个工作流。

项目路径：/Users/Zhuanz/Desktop/yp-nx-dashboard

第 1 步：读取参考文件
/Users/Zhuanz/Claude-Code-Workflow/.claude/commands/issue/plan.md
/Users/Zhuanz/Claude-Code-Workflow/.claude/commands/issue/queue.md
/Users/Zhuanz/Claude-Code-Workflow/.claude/commands/issue/execute.md

第 2 步：创建 config/workflows/issue/issue-plan.yaml
- 读取所有 status=discovered 的 issues（GET /api/v1/issues?status=discovered）
- 为每个 issue 调用 planner agent 生成解决方案（solution 字段）
- 更新每个 issue：status=planned，solution=<生成的方案>

第 3 步：创建 config/workflows/issue/issue-queue.yaml
- 读取所有 status=planned 的 issues
- queue-agent 分析 depends_on 关系，用拓扑排序生成执行顺序
- 将排序结果写入 nx_api/src/models/issue_queue.rs（新建此文件）
  queue 字段：groups: Vec<Vec<String>>（每组内的 issue 可并行，组间顺序执行）

第 4 步：创建 config/workflows/issue/issue-execute.yaml
- 读取 issue_queue，按 groups 顺序执行
- 同一 group 内的 issues 并行执行（parallel: true）
- 每个 issue 对应一个 executor agent，执行 solution 中的修复方案
- 执行完更新 status=completed 或 failed

第 5 步：在 nx_dashboard/src/pages/TasksPage.tsx 中
- 展示 issue 列表（按 status 分组）
- 顶部四个按钮：Discover / Plan / Queue / Execute
- 每个按钮点击后触发对应工作流执行
- 实时显示执行进度

验收标准：
- 完整跑一遍：Discover → Plan → Queue → Execute
- 最终所有 issues status=completed
- 有依赖关系的 issues 按正确顺序执行
```

---

# P6：UI 设计工作流

> **依赖：P0.1 P0.2（条件跳转+变量提取）**

## AI 提示词

```
在 yp-nx-dashboard 中实现 6 个 UI 设计工作流。

项目路径：/Users/Zhuanz/Desktop/yp-nx-dashboard

第 1 步：读取所有参考文件
/Users/Zhuanz/Claude-Code-Workflow/.claude/commands/workflow/ui-design/（全部 .md 文件）

第 2 步：创建 6 个工作流 YAML（存放在 config/workflows/ui-design/）

① style-extract.yaml
输入：image_path 或 code_path
agent 提取：主色/辅色/背景色（HEX）、字体族/字重/字号、间距系统（4px/8px等）、圆角值
输出 JSON 存入 state 变量 style_spec：
{ colors: {primary,secondary,bg}, typography: {family,weights,sizes}, spacing: [], radius: [] }

② layout-extract.yaml
输入：image_path 或 html_path
agent 提取：网格列数、对齐规则、组件层级关系、响应式断点
输出变量 layout_spec

③ animation-extract.yaml
输入：css_path 或 image_path（含动效说明）
agent 提取：动画时长、缓动函数、触发条件、关键帧
输出变量 animation_spec

④ generate.yaml
输入：style_spec（可引用 style-extract 的输出）+ component_description
agent 根据设计规格生成对应组件代码（React + Tailwind）

⑤ codify-style.yaml
输入：style_spec
agent 将规格写入代码库：
- 创建/更新 src/styles/tokens.css（CSS 变量）
- 创建/更新 tailwind.config.js（颜色/字体/间距扩展）

⑥ design-sync.yaml
输入：reference_path（设计稿目录）+ source_path（代码目录）
agent 对比设计稿和代码实现的差异，生成同步报告

第 3 步：在 SkillsPage.tsx 或 WorkflowsPage.tsx 中
添加 "UI 设计" 分类，在该分类下展示这 6 个工作流

验收标准：
- Dashboard 中出现 UI 设计分类和 6 个工作流
- style-extract 传入图片路径，输出包含正确颜色/字体 JSON
- codify-style 能正确生成/更新 CSS 变量文件
```

---

# 整体进度跟踪

| 阶段 | 状态 | 验证命令 |
|------|------|---------|
| P0-parser.rs 扩展 | ⬜ | `cargo check -p nexus-workflow` |
| P0-Cargo.toml 加 regex | ⬜ | `cargo check -p nexus-workflow` |
| P0-engine.rs 执行循环 | ⬜ | `cargo test -p nexus-workflow` |
| P0-events.rs 新事件 | ⬜ | `cargo build -p nexus-workflow` |
| P1 investigate | ⬜ | Dashboard 可执行 + 阶段跳转正确 |
| P2 review-cycle | ⬜ | 前端弹出选择框 + 结果正确 |
| P3 brainstorm | ⬜ | 并行 agents 同时执行 |
| P4 TDD 工作流 | ⬜ | loop 退出条件正确 |
| P5-A Issue 数据模型 | ⬜ | CRUD API 正常 |
| P5-B Issue Discover | ⬜ | issues 写入数据库 |
| P5-C Issue Plan/Queue/Execute | ⬜ | 完整闭环跑通 |
| P6 UI 设计工作流 | ⬜ | 6 个工作流在 Dashboard 可见 |

---

*文档版本：v2.0 | 项目：yp-nx-dashboard | 参考：Claude-Code-Workflow v7.3.3*
