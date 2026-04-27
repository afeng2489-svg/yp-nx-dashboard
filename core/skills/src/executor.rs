//! Skill Executor - 技能执行引擎
//!
//! 提供技能的执行、调度和结果追踪功能。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use parking_lot::RwLock;
use thiserror::Error;
use tracing::info;

use super::registry::SkillRegistry;
use super::skill::{SkillContext, SkillExecutionResult, SkillExecutor};

// ============================================================================
// Errors
// ============================================================================

/// 技能执行器错误
#[derive(Debug, Error)]
pub enum ExecutorError {
    #[error("技能未找到: {0}")]
    SkillNotFound(String),

    #[error("执行失败: {0}")]
    ExecutionFailed(String),

    #[error("注册表错误: {0}")]
    RegistryError(String),

    #[error("无效参数: {0}")]
    InvalidParams(String),

    #[error("超时: {0}")]
    Timeout(String),
}

// ============================================================================
// Executor Registry
// ============================================================================

/// 技能执行器注册表
///
/// 管理所有 `SkillExecutor` 实现的注册和查找。
#[derive(Default)]
pub struct ExecutorRegistry {
    executors: RwLock<HashMap<String, Arc<dyn SkillExecutor>>>,
}

impl ExecutorRegistry {
    /// 创建新的执行器注册表
    pub fn new() -> Self {
        Self {
            executors: RwLock::new(HashMap::new()),
        }
    }

    /// 注册技能执行器
    pub fn register(&self, executor: Arc<dyn SkillExecutor>) -> Result<(), ExecutorError> {
        let name = executor.name().to_string();
        let mut executors = self.executors.write();
        if executors.contains_key(&name) {
            return Err(ExecutorError::RegistryError(format!(
                "Executor '{}' already registered",
                name
            )));
        }
        executors.insert(name, executor);
        Ok(())
    }

    /// 批量注册执行器
    pub fn register_many(
        &self,
        executors: impl IntoIterator<Item = Arc<dyn SkillExecutor>>,
    ) -> usize {
        let mut count = 0;
        let mut executors_lock = self.executors.write();
        for executor in executors {
            let name = executor.name().to_string();
            if let std::collections::hash_map::Entry::Vacant(e) = executors_lock.entry(name) {
                e.insert(executor);
                count += 1;
            }
        }
        count
    }

    /// 通过名称获取执行器
    pub fn get(&self, name: &str) -> Option<Arc<dyn SkillExecutor>> {
        let executors = self.executors.read();
        executors.get(name).cloned()
    }

    /// 获取所有已注册的执行器
    pub fn all(&self) -> Vec<Arc<dyn SkillExecutor>> {
        let executors = self.executors.read();
        executors.values().cloned().collect()
    }

    /// 检查执行器是否已注册
    pub fn contains(&self, name: &str) -> bool {
        let executors = self.executors.read();
        executors.contains_key(name)
    }

    /// 获取已注册执行器数量
    pub fn len(&self) -> usize {
        let executors = self.executors.read();
        executors.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        let executors = self.executors.read();
        executors.is_empty()
    }
}

// ============================================================================
// Skill Execution Engine
// ============================================================================

/// 技能执行引擎
///
/// 负责管理技能的执行、调度和结果追踪。
#[allow(dead_code)]
pub struct SkillExecutionEngine {
    /// 技能执行器注册表
    executor_registry: Arc<ExecutorRegistry>,
    /// 技能元数据注册表
    metadata_registry: Arc<SkillRegistry>,
    /// 执行中的任务
    active_executions: RwLock<HashMap<String, ActiveExecution>>,
    /// 执行历史
    execution_history: RwLock<Vec<ExecutionRecord>>,
}

/// 活跃执行任务
#[allow(dead_code)]
struct ActiveExecution {
    execution_id: String,
    skill_name: String,
    phase: String,
    started_at: Instant,
    context: SkillContext,
}

/// 执行记录
#[derive(Debug, Clone)]
pub struct ExecutionRecord {
    pub execution_id: String,
    pub skill_name: String,
    pub phase: String,
    pub success: bool,
    pub duration_ms: u64,
    pub error: Option<String>,
}

impl SkillExecutionEngine {
    /// 创建新的技能执行引擎
    pub fn new(
        executor_registry: Arc<ExecutorRegistry>,
        metadata_registry: Arc<SkillRegistry>,
    ) -> Self {
        Self {
            executor_registry,
            metadata_registry,
            active_executions: RwLock::new(HashMap::new()),
            execution_history: RwLock::new(Vec::new()),
        }
    }

    /// 获取执行器注册表
    pub fn executor_registry(&self) -> Arc<ExecutorRegistry> {
        self.executor_registry.clone()
    }

    /// 执行技能
    ///
    /// # Arguments
    /// * `skill_name` - 技能名称
    /// * `phase` - 要执行的阶段（如果为 None，则执行所有阶段）
    /// * `context` - 执行上下文
    ///
    /// # Returns
    /// 执行结果
    pub async fn execute(
        &self,
        skill_name: &str,
        phase: Option<&str>,
        context: SkillContext,
    ) -> Result<SkillExecutionResult, ExecutorError> {
        let execution_id = uuid::Uuid::new_v4().to_string();
        info!(
            "Starting skill execution: {} (id: {})",
            skill_name, execution_id
        );

        // 获取技能执行器
        let skill = self
            .executor_registry
            .get(skill_name)
            .ok_or_else(|| ExecutorError::SkillNotFound(skill_name.to_string()))?;

        // 如果指定了阶段，直接执行该阶段
        if let Some(phase_name) = phase {
            return self
                .execute_single_phase(&execution_id, skill_name, phase_name, &skill, context)
                .await;
        }

        // 否则执行所有阶段
        self.execute_all_phases(&execution_id, skill_name, &skill, context)
            .await
    }

    /// 执行单个阶段
    async fn execute_single_phase(
        &self,
        execution_id: &str,
        skill_name: &str,
        phase_name: &str,
        skill: &Arc<dyn SkillExecutor>,
        context: SkillContext,
    ) -> Result<SkillExecutionResult, ExecutorError> {
        // 验证参数
        skill
            .validate(&context.params)
            .map_err(|e| ExecutorError::InvalidParams(e.to_string()))?;

        // 检查阶段是否存在
        let phases = skill.phases();
        if !phases.iter().any(|p| p.name == phase_name) {
            return Err(ExecutorError::ExecutionFailed(format!(
                "Invalid phase '{}' for skill '{}'. Available phases: {:?}",
                phase_name,
                skill_name,
                phases.iter().map(|p| &p.name).collect::<Vec<_>>()
            )));
        }

        // 记录活跃执行
        {
            let mut active = self.active_executions.write();
            active.insert(
                execution_id.to_string(),
                ActiveExecution {
                    execution_id: execution_id.to_string(),
                    skill_name: skill_name.to_string(),
                    phase: phase_name.to_string(),
                    started_at: Instant::now(),
                    context: context.clone(),
                },
            );
        }

        // 执行阶段
        let start = Instant::now();
        let result = skill.execute(phase_name, &context).await;
        let duration_ms = start.elapsed().as_millis() as u64;

        // 移除活跃执行
        {
            let mut active = self.active_executions.write();
            active.remove(execution_id);
        }

        // 记录执行历史
        let success = result.as_ref().map(|r| r.success).unwrap_or(false);
        {
            let mut history = self.execution_history.write();
            history.push(ExecutionRecord {
                execution_id: execution_id.to_string(),
                skill_name: skill_name.to_string(),
                phase: phase_name.to_string(),
                success,
                duration_ms,
                error: result.as_ref().err().map(|e| e.to_string()),
            });
        }

        result.map_err(|e| ExecutorError::ExecutionFailed(e.to_string()))
    }

    /// 执行所有阶段
    async fn execute_all_phases(
        &self,
        execution_id: &str,
        skill_name: &str,
        skill: &Arc<dyn SkillExecutor>,
        context: SkillContext,
    ) -> Result<SkillExecutionResult, ExecutorError> {
        let phases = skill.phases();
        if phases.is_empty() {
            return Err(ExecutorError::ExecutionFailed(format!(
                "Skill '{}' has no phases to execute",
                skill_name
            )));
        }

        let mut outputs = Vec::new();
        let mut all_success = true;
        let mut last_error = None;

        for phase in &phases {
            if !phase.required && context.params.get(&phase.name).is_none() {
                info!("Skipping optional phase '{}'", phase.name);
                continue;
            }

            info!(
                "Executing phase '{}' for skill '{}'",
                phase.name, skill_name
            );

            let phase_result = self
                .execute_single_phase(
                    execution_id,
                    skill_name,
                    &phase.name,
                    skill,
                    context.clone(),
                )
                .await;

            match phase_result {
                Ok(result) => {
                    if result.success {
                        outputs.push(serde_json::json!({
                            "phase": result.phase,
                            "output": result.output,
                        }));
                    } else {
                        all_success = false;
                        last_error = result.error.clone();
                        if phase.required {
                            break;
                        }
                    }
                }
                Err(e) => {
                    all_success = false;
                    last_error = Some(e.to_string());
                    if phase.required {
                        break;
                    }
                }
            }
        }

        let total_duration: u64 = {
            let history = self.execution_history.read();
            history
                .iter()
                .filter(|r| r.execution_id == execution_id)
                .map(|r| r.duration_ms)
                .sum()
        };

        Ok(SkillExecutionResult {
            success: all_success,
            output: serde_json::json!({ "phase_results": outputs }),
            phase: None,
            error: last_error,
            duration_ms: total_duration,
        })
    }

    /// 获取活跃执行数量
    pub fn active_count(&self) -> usize {
        let active = self.active_executions.read();
        active.len()
    }

    /// 获取执行历史
    pub fn get_history(&self, limit: usize) -> Vec<ExecutionRecord> {
        let history = self.execution_history.read();
        history.iter().rev().take(limit).cloned().collect()
    }

    /// 清空执行历史
    pub fn clear_history(&self) {
        let mut history = self.execution_history.write();
        history.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::{SkillCategory, SkillError, SkillPhase};
    use async_trait::async_trait;

    /// 测试用的小技能
    #[allow(dead_code)]
    struct TestSkill {
        name: String,
        description: String,
        phases: Vec<SkillPhase>,
    }

    impl TestSkill {
        fn new(name: &str, description: &str, phases: Vec<SkillPhase>) -> Self {
            Self {
                name: name.to_string(),
                description: description.to_string(),
                phases,
            }
        }
    }

    #[async_trait]
    impl SkillExecutor for TestSkill {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            &self.description
        }

        fn category(&self) -> SkillCategory {
            SkillCategory::Development
        }

        fn phases(&self) -> Vec<SkillPhase> {
            self.phases.clone()
        }

        fn validate(&self, _params: &serde_json::Value) -> Result<(), SkillError> {
            Ok(())
        }

        async fn execute(
            &self,
            phase: &str,
            _context: &SkillContext,
        ) -> Result<SkillExecutionResult, SkillError> {
            Ok(SkillExecutionResult::success(
                phase,
                serde_json::json!({ "executed": phase }),
                100,
            ))
        }
    }

    #[test]
    fn test_execution_record() {
        let record = ExecutionRecord {
            execution_id: "test-1".to_string(),
            skill_name: "test-skill".to_string(),
            phase: "phase1".to_string(),
            success: true,
            duration_ms: 100,
            error: None,
        };

        assert!(record.success);
        assert_eq!(record.duration_ms, 100);
    }
}
