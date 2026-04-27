//! NexusFlow Skill System - 技能系统
//!
//! 提供 37+ 预定义技能，支持工作流规划、测试驱动开发、头脑风暴等场景。

pub mod collaboration_skills;
pub mod core_skills;
pub mod development_skills;
pub mod executor;
pub mod planning_skills;
pub mod registry;
pub mod skill;
pub mod workflow_skills;

// Re-export skill trait and types
pub use executor::{ExecutionRecord, ExecutorError, ExecutorRegistry, SkillExecutionEngine};
pub use registry::{RegistryError, SkillRegistry};
pub use skill::{
    ParameterType, Skill, SkillCategory, SkillContext, SkillError, SkillExecutionResult,
    SkillExecutor, SkillId, SkillMetadata, SkillParameter, SkillPhase,
};

// Re-export core skills functions
pub use core_skills::all_core_skills;

/// 注册所有预定义技能到注册表
pub fn register_all_skills(registry: &SkillRegistry) -> Result<usize, RegistryError> {
    // 批量注册所有技能
    let count = registry.register_many(workflow_skills::all_workflow_skills())?;
    let count = count + registry.register_many(planning_skills::all_planning_skills())?;
    let count = count + registry.register_many(collaboration_skills::all_collaboration_skills())?;
    let count = count + registry.register_many(development_skills::all_development_skills())?;

    Ok(count)
}

/// 获取所有预定义技能
pub fn all_skills() -> Vec<Skill> {
    let mut skills = Vec::new();
    skills.extend(workflow_skills::all_workflow_skills());
    skills.extend(planning_skills::all_planning_skills());
    skills.extend(collaboration_skills::all_collaboration_skills());
    skills.extend(development_skills::all_development_skills());
    skills
}
