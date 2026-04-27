//! 工作流技能
//!
//! 提供工作流规划和执行相关的技能。

use crate::{ParameterType, Skill, SkillCategory, SkillId, SkillMetadata, SkillParameter};

/// 创建工作流规划技能
pub fn workflow_lite_plan() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("workflow-lite-plan"),
        "workflow-lite-plan",
        "轻量级单模块规划技能。适用于简单任务的快速规划，生成简洁的实现步骤。",
    )
    .with_category(SkillCategory::WorkflowPlanning)
    .with_tag("planning")
    .with_tag("lite")
    .with_tag("single-module")
    .with_parameter(SkillParameter {
        name: "task_description".to_string(),
        description: "任务描述".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "module".to_string(),
        description: "目标模块".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    });

    Skill::new(metadata, "workflow_lite_plan").with_config(serde_json::json!({
        "max_steps": 10,
        "timeout_secs": 300
    }))
}

/// 创建多 CLI 协作分析技能
pub fn workflow_multi_cli_plan() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("workflow-multi-cli-plan"),
        "workflow-multi-cli-plan",
        "多 CLI 协作分析技能。协调多个 CLI 工具进行联合分析，汇总结果。",
    )
    .with_category(SkillCategory::WorkflowPlanning)
    .with_tag("planning")
    .with_tag("multi-cli")
    .with_tag("collaboration")
    .with_parameter(SkillParameter {
        name: "task".to_string(),
        description: "分析任务".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "cli_tools".to_string(),
        description: "要使用的 CLI 工具列表".to_string(),
        param_type: ParameterType::Array,
        required: true,
        default: None,
    });

    Skill::new(metadata, "workflow_multi_cli_plan").with_config(serde_json::json!({
        "parallel": true,
        "timeout_secs": 600
    }))
}

/// 创建完整规划技能
pub fn workflow_plan() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("workflow-plan"),
        "workflow-plan",
        "完整规划技能。包含需求分析、架构设计、任务拆分的完整流程，支持会话持久化。",
    )
    .with_category(SkillCategory::WorkflowPlanning)
    .with_tag("planning")
    .with_tag("full")
    .with_tag("persistent")
    .with_parameter(SkillParameter {
        name: "project_name".to_string(),
        description: "项目名称".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "requirements".to_string(),
        description: "需求描述".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "save_session".to_string(),
        description: "是否保存会话".to_string(),
        param_type: ParameterType::Boolean,
        required: false,
        default: Some(serde_json::json!(true)),
    });

    Skill::new(metadata, "workflow_plan").with_config(serde_json::json!({
        "phases": ["analysis", "design", "breakdown", "review"],
        "timeout_secs": 1800
    }))
}

/// 创建 TDD 工作流技能
pub fn workflow_tdd_plan() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("workflow-tdd-plan"),
        "workflow-tdd-plan",
        "测试驱动开发工作流。遵循红-绿-重构循环，先写测试再实现。",
    )
    .with_category(SkillCategory::WorkflowPlanning)
    .with_tag("tdd")
    .with_tag("testing")
    .with_tag("development")
    .with_parameter(SkillParameter {
        name: "feature".to_string(),
        description: "要开发的功能".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "test_framework".to_string(),
        description: "测试框架".to_string(),
        param_type: ParameterType::String,
        required: false,
        default: Some(serde_json::json!("default")),
    });

    Skill::new(metadata, "workflow_tdd_plan").with_config(serde_json::json!({
        "phases": ["write_test", "run_test_red", "implement", "run_test_green", "refactor"],
        "coverage_target": 80
    }))
}

/// 创建测试修复循环技能
pub fn workflow_test_fix() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("workflow-test-fix"),
        "workflow-test-fix",
        "测试生成修复循环。持续生成测试和修复，直到测试通过。",
    )
    .with_category(SkillCategory::WorkflowPlanning)
    .with_tag("testing")
    .with_tag("fix")
    .with_tag("loop")
    .with_parameter(SkillParameter {
        name: "test_target".to_string(),
        description: "测试目标".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "max_iterations".to_string(),
        description: "最大迭代次数".to_string(),
        param_type: ParameterType::Integer,
        required: false,
        default: Some(serde_json::json!(5)),
    });

    Skill::new(metadata, "workflow_test_fix").with_config(serde_json::json!({
        "loop_mode": true,
        "timeout_secs": 1200
    }))
}

/// 创建头脑风暴技能
pub fn brainstorm() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("brainstorm"),
        "brainstorm",
        "多角色头脑风暴。模拟多个专家角色进行创意讨论和问题解决。",
    )
    .with_category(SkillCategory::WorkflowPlanning)
    .with_tag("brainstorm")
    .with_tag("creative")
    .with_tag("multi-role")
    .with_parameter(SkillParameter {
        name: "topic".to_string(),
        description: "讨论主题".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "roles".to_string(),
        description: "参与角色列表".to_string(),
        param_type: ParameterType::Array,
        required: false,
        default: Some(serde_json::json!(["architect", "developer", "tester"])),
    })
    .with_parameter(SkillParameter {
        name: "rounds".to_string(),
        description: "讨论轮数".to_string(),
        param_type: ParameterType::Integer,
        required: false,
        default: Some(serde_json::json!(3)),
    });

    Skill::new(metadata, "brainstorm").with_config(serde_json::json!({
        "enable_diversity": true,
        "timeout_secs": 900
    }))
}

/// 获取所有工作流技能
pub fn all_workflow_skills() -> Vec<Skill> {
    vec![
        workflow_lite_plan(),
        workflow_multi_cli_plan(),
        workflow_plan(),
        workflow_tdd_plan(),
        workflow_test_fix(),
        brainstorm(),
    ]
}
