//! Feature Flag 数据模型 — 三态功能开关 + 熔断器

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 功能开关状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FeatureFlagState {
    /// 完全启用
    On,
    /// 只读模式（可查询，不可写入/修改）
    ReadOnly,
    /// 完全禁用
    Off,
}

impl FeatureFlagState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::On => "on",
            Self::ReadOnly => "readonly",
            Self::Off => "off",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "on" => Some(Self::On),
            "readonly" => Some(Self::ReadOnly),
            "off" => Some(Self::Off),
            _ => None,
        }
    }
}

/// 功能开关
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlag {
    /// 唯一标识: "pipeline", "snapshot", "crash_resume", "file_watch" 等
    pub key: String,
    /// 当前状态
    pub state: FeatureFlagState,
    /// 熔断器是否已触发
    pub circuit_breaker: bool,
    /// 连续错误计数
    pub error_count: u32,
    /// 连续错误阈值，超过自动 Off
    pub error_threshold: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl FeatureFlag {
    /// 功能是否启用（On 状态且未熔断）
    pub fn is_enabled(&self) -> bool {
        self.state == FeatureFlagState::On && !self.circuit_breaker
    }

    /// 功能是否至少可读（On 或 ReadOnly，且未熔断）
    pub fn is_readable(&self) -> bool {
        (self.state == FeatureFlagState::On || self.state == FeatureFlagState::ReadOnly)
            && !self.circuit_breaker
    }

    /// 是否应触发熔断
    pub fn should_trip(&self) -> bool {
        self.error_count >= self.error_threshold
    }
}

/// 已知的 feature flag key 常量
pub mod keys {
    pub const PIPELINE: &str = "pipeline";
    pub const SNAPSHOT: &str = "snapshot";
    pub const CRASH_RESUME: &str = "crash_resume";
    pub const FILE_WATCH: &str = "file_watch";
    pub const PROCESS_LIFECYCLE: &str = "process_lifecycle";
}
