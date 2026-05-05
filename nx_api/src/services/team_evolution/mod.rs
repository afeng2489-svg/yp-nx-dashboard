//! Team Evolution Module - 团队模块进化
//!
//! # 主控分离原则
//!
//! NexusFlow 平台是大脑，Claude CLI 是手脚：
//! - CLI **每次只执行一个小步骤**（一个文件、一个模块、一个接口）
//! - 平台负责：任务拆分、顺序编排、依赖检查、进度汇总、异常处理
//! - **禁止** 向 CLI 发送「完成整个后端」这样的全量指令
//!
//! # 任务分层 Pipeline
//!
//! - Phase 1（串行）：需求分析 → 架构设计 → 项目初始化
//! - Phase 2（并行）：前端项目 + 后端项目 双独立进程
//! - Phase 3（串行）：接口联调 → 测试 → 文档 → 打包
//!
//! # Feature Toggle
//!
//! 所有新模块入口通过 feature flag 守卫，支持三态切换和熔断降级。
//! 任何新功能出错不影响核心系统。

pub mod error;

// P1: Feature Flag
pub mod feature_flag_repository;
pub mod feature_flag_service;

// P1: Pipeline Engine
pub mod pipeline_repository;
pub mod pipeline_service;
pub mod pipeline_dispatcher;

// P2: Snapshots + Progress
pub mod snapshot_repository;
pub mod snapshot_service;

// P3: Process Isolation + Lifecycle
pub mod process_isolation;
pub mod process_lifecycle;

// P4: Breakpoint Resume + Crash Recovery
pub mod crash_detector;
pub mod resume_service;
pub mod temp_cleaner;

// P5: File Watch + Error Layering
pub mod file_watcher;

// Integration layer
pub mod integration;
pub mod quality_gate;

// Re-exports
pub use crash_detector::CrashDetector;
pub use error::TeamEvolutionError;
pub use feature_flag_service::FeatureFlagService;
pub use file_watcher::FileWatcher;
pub use integration::TeamEvolutionEventHandler;
pub use pipeline_service::PipelineService;
pub use process_isolation::{infer_process_type, IsolatedProcess, ProcessRegistry, ProcessType};
pub use process_lifecycle::{
    LifecycleConfig, ProcessLifecycleEvent, ProcessLifecycleManager, ProcessStats,
};
pub use resume_service::ResumeService;
pub use snapshot_service::SnapshotService;
pub use temp_cleaner::TempCleaner;
