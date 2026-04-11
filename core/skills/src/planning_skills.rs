//! 规划技能
//!
//! 提供各种规划相关的技能。

use crate::{Skill, SkillCategory, SkillId, SkillMetadata, SkillParameter, ParameterType};

/// 创建架构设计技能
pub fn architecture_plan() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("architecture-plan"),
        "architecture-plan",
        "架构设计规划技能。分析需求并设计系统架构，包括组件划分、数据流设计等。"
    )
    .with_category(SkillCategory::WorkflowPlanning)
    .with_tag("architecture")
    .with_tag("design")
    .with_tag("planning")
    .with_parameter(SkillParameter {
        name: "project_type".to_string(),
        description: "项目类型".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "constraints".to_string(),
        description: "约束条件".to_string(),
        param_type: ParameterType::Array,
        required: false,
        default: None,
    });

    Skill::new(metadata, "architecture_plan")
        .with_config(serde_json::json!({
            "output": "architecture_document",
            "timeout_secs": 1800
        }))
}

/// 创建任务拆分技能
pub fn task_breakdown() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("task-breakdown"),
        "task-breakdown",
        "任务拆解技能。将大型任务拆分为可管理的小任务。"
    )
    .with_category(SkillCategory::WorkflowPlanning)
    .with_tag("planning")
    .with_tag("task")
    .with_tag("breakdown")
    .with_parameter(SkillParameter {
        name: "task".to_string(),
        description: "要拆分的任务".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "granularity".to_string(),
        description: "拆解粒度".to_string(),
        param_type: ParameterType::String,
        required: false,
        default: Some(serde_json::json!("medium")),
    });

    Skill::new(metadata, "task_breakdown")
        .with_config(serde_json::json!({
            "output_format": "task_list",
            "estimate_hours": true
        }))
}

/// 创建代码审查规划技能
pub fn code_review_plan() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("code-review-plan"),
        "code-review-plan",
        "代码审查规划技能。制定代码审查计划，确定审查重点和审查点。"
    )
    .with_category(SkillCategory::WorkflowPlanning)
    .with_tag("review")
    .with_tag("code")
    .with_tag("planning")
    .with_parameter(SkillParameter {
        name: "changes".to_string(),
        description: "代码变更".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "focus_areas".to_string(),
        description: "重点审查领域".to_string(),
        param_type: ParameterType::Array,
        required: false,
        default: None,
    });

    Skill::new(metadata, "code_review_plan")
        .with_config(serde_json::json!({
            "checklist": ["security", "performance", "correctness", "maintainability"],
            "timeout_secs": 600
        }))
}

/// 创建迁移规划技能
pub fn migration_plan() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("migration-plan"),
        "migration-plan",
        "迁移规划技能。规划系统或依赖的迁移路径。"
    )
    .with_category(SkillCategory::WorkflowPlanning)
    .with_tag("migration")
    .with_tag("planning")
    .with_tag("upgrade")
    .with_parameter(SkillParameter {
        name: "from_version".to_string(),
        description: "源版本".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "to_version".to_string(),
        description: "目标版本".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "scope".to_string(),
        description: "迁移范围".to_string(),
        param_type: ParameterType::String,
        required: false,
        default: Some(serde_json::json!("full")),
    });

    Skill::new(metadata, "migration_plan")
        .with_config(serde_json::json!({
            "risk_assessment": true,
            "rollback_plan": true
        }))
}

/// 创建发布规划技能
pub fn release_plan() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("release-plan"),
        "release-plan",
        "发布规划技能。规划版本发布，包括功能列表、测试计划、部署步骤。"
    )
    .with_category(SkillCategory::WorkflowPlanning)
    .with_tag("release")
    .with_tag("planning")
    .with_tag("deployment")
    .with_parameter(SkillParameter {
        name: "version".to_string(),
        description: "版本号".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "features".to_string(),
        description: "功能列表".to_string(),
        param_type: ParameterType::Array,
        required: true,
        default: None,
    });

    Skill::new(metadata, "release_plan")
        .with_config(serde_json::json!({
            "include_changelog": true,
            "include_rollback": true,
            "timeout_secs": 900
        }))
}

/// 获取所有规划技能
pub fn all_planning_skills() -> Vec<Skill> {
    vec![
        architecture_plan(),
        task_breakdown(),
        code_review_plan(),
        migration_plan(),
        release_plan(),
    ]
}
