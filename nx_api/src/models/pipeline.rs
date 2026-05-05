//! Pipeline 数据模型 — 任务编排引擎核心类型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 开发阶段
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PipelinePhase {
    /// Phase 1: 串行前置
    RequirementsAnalysis,
    ArchitectureDesign,
    ProjectInit,
    /// Phase 2: 并行核心开发
    BackendDev,
    FrontendDev,
    /// Phase 3: 串行收尾
    ApiIntegration,
    Testing,
    Documentation,
    Packaging,
}

impl PipelinePhase {
    /// 返回当前阶段的执行模式
    pub fn execution_mode(&self) -> ExecutionMode {
        match self {
            Self::BackendDev | Self::FrontendDev => ExecutionMode::Parallel,
            _ => ExecutionMode::Serial,
        }
    }

    /// 返回阶段所属的大阶段编号 (1, 2, 3)
    pub fn phase_group(&self) -> u8 {
        match self {
            Self::RequirementsAnalysis | Self::ArchitectureDesign | Self::ProjectInit => 1,
            Self::BackendDev | Self::FrontendDev => 2,
            Self::ApiIntegration | Self::Testing | Self::Documentation | Self::Packaging => 3,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RequirementsAnalysis => "requirements_analysis",
            Self::ArchitectureDesign => "architecture_design",
            Self::ProjectInit => "project_init",
            Self::BackendDev => "backend_dev",
            Self::FrontendDev => "frontend_dev",
            Self::ApiIntegration => "api_integration",
            Self::Testing => "testing",
            Self::Documentation => "documentation",
            Self::Packaging => "packaging",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "requirements_analysis" => Some(Self::RequirementsAnalysis),
            "architecture_design" => Some(Self::ArchitectureDesign),
            "project_init" => Some(Self::ProjectInit),
            "backend_dev" => Some(Self::BackendDev),
            "frontend_dev" => Some(Self::FrontendDev),
            "api_integration" => Some(Self::ApiIntegration),
            "testing" => Some(Self::Testing),
            "documentation" => Some(Self::Documentation),
            "packaging" => Some(Self::Packaging),
            _ => None,
        }
    }
}

/// 阶段执行模式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionMode {
    /// Phase 1 & 3: 严格顺序
    Serial,
    /// Phase 2: 前后端并行
    Parallel,
}

/// 单个步骤 = 一次 CLI 调用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStep {
    pub id: String,
    pub pipeline_id: String,
    pub task_id: String,
    pub phase: PipelinePhase,
    /// 哪个角色执行
    pub role_id: String,
    /// 发给 CLI 的具体指令（小粒度）
    pub instruction: String,
    /// 前置步骤 ID
    pub depends_on: Vec<String>,
    pub status: StepStatus,
    /// CLI 返回结果
    pub output: Option<String>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// 步骤状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StepStatus {
    /// 等待前置完成
    Pending,
    /// 前置已完成，可调度
    Ready,
    /// CLI 正在执行
    Running,
    /// 执行成功
    Completed,
    /// 执行失败
    Failed,
    /// 跳过
    Skipped,
    /// 被阻塞（前置失败）
    Blocked,
}

impl StepStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Ready => "ready",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
            Self::Blocked => "blocked",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "ready" => Some(Self::Ready),
            "running" => Some(Self::Running),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "skipped" => Some(Self::Skipped),
            "blocked" => Some(Self::Blocked),
            _ => None,
        }
    }

    /// 是否为终态
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Skipped)
    }
}

/// Pipeline 状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PipelineStatus {
    Idle,
    Running,
    Paused,
    WaitingForApproval,
    Completed,
    Failed,
}

impl PipelineStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Running => "running",
            Self::Paused => "paused",
            Self::WaitingForApproval => "waiting_for_approval",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "idle" => Some(Self::Idle),
            "running" => Some(Self::Running),
            "paused" => Some(Self::Paused),
            "waiting_for_approval" => Some(Self::WaitingForApproval),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

/// 阶段门控策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseGatePolicy {
    /// Phase 1 所有步骤完成才能进入 Phase 2
    pub require_all_prereqs: bool,
    /// Phase 2 -> Phase 3 的门控条件
    pub parallel_completion_required: bool,
    /// 失败后是否自动重试
    pub auto_retry: bool,
    pub max_step_retries: u32,
}

impl Default for PhaseGatePolicy {
    fn default() -> Self {
        Self {
            require_all_prereqs: true,
            parallel_completion_required: true,
            auto_retry: true,
            max_step_retries: 3,
        }
    }
}

/// Pipeline 实例（一个项目一个 Pipeline）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub id: String,
    pub project_id: String,
    pub team_id: String,
    pub current_phase: PipelinePhase,
    pub status: PipelineStatus,
    pub phase_gate_policy: PhaseGatePolicy,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
