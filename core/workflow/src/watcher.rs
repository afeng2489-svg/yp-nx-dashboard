//! 工作流执行的扩展观察者
//!
//! 让外部（nx_api）能在每个 stage 执行前后做事，比如：
//! - 拍 working_dir 快照计算文件 diff
//! - 记录 token 用量
//! - 自定义指标采集
//!
//! 设计原则：core/workflow 不能依赖具体实现（如 SQLite、文件系统库），
//! 只暴露最小 trait，让 nx_api 这种上层注入实现。

use std::sync::Arc;

/// RAG 检索 provider trait（由 nx_api 注入实现）
#[async_trait::async_trait]
pub trait RagProvider: Send + Sync {
    /// 检索相关文本片段
    async fn retrieve(&self, kb_id: &str, query: &str, top_k: usize, threshold: f32)
        -> Vec<String>;
}

/// stage 执行的观察者
///
/// engine 在每个 stage 开始/结束时调用，**同步阻塞**。
/// 实现里要尽量快（< 100ms 通常 OK，几秒级会拖慢工作流）。
pub trait StageWatcher: Send + Sync {
    /// stage 开始前调用
    fn before_stage(&self, execution_id: &str, stage_name: &str);

    /// stage 完成后调用
    fn after_stage(&self, execution_id: &str, stage_name: &str);
}

/// 一组 watcher，按顺序通知
#[derive(Clone, Default)]
pub struct StageWatchers {
    watchers: Vec<Arc<dyn StageWatcher>>,
}

impl StageWatchers {
    pub fn new() -> Self {
        Self {
            watchers: Vec::new(),
        }
    }

    pub fn push(&mut self, watcher: Arc<dyn StageWatcher>) {
        self.watchers.push(watcher);
    }

    pub fn notify_before(&self, execution_id: &str, stage_name: &str) {
        for w in &self.watchers {
            w.before_stage(execution_id, stage_name);
        }
    }

    pub fn notify_after(&self, execution_id: &str, stage_name: &str) {
        for w in &self.watchers {
            w.after_stage(execution_id, stage_name);
        }
    }
}
