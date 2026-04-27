//! 协作技能
//!
//! 提供团队协作相关的技能。

use crate::{ParameterType, Skill, SkillCategory, SkillId, SkillMetadata, SkillParameter};

/// 创建代码审查技能
pub fn code_review() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("code-review"),
        "code-review",
        "代码审查技能。执行代码审查，发现问题并提供改进建议。",
    )
    .with_category(SkillCategory::Review)
    .with_tag("review")
    .with_tag("code")
    .with_parameter(SkillParameter {
        name: "diff".to_string(),
        description: "代码差异".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "language".to_string(),
        description: "编程语言".to_string(),
        param_type: ParameterType::String,
        required: false,
        default: None,
    });

    Skill::new(metadata, "code_review").with_config(serde_json::json!({
        "check_security": true,
        "check_performance": true
    }))
}

/// 创建 PR 审查技能
pub fn pr_review() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("pr-review"),
        "pr-review",
        "Pull Request 审查技能。全面审查 PR，包括设计、实现、测试等方面。",
    )
    .with_category(SkillCategory::Review)
    .with_tag("review")
    .with_tag("pr")
    .with_tag("collaboration")
    .with_parameter(SkillParameter {
        name: "pr_url".to_string(),
        description: "PR URL".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "focus".to_string(),
        description: "审查重点".to_string(),
        param_type: ParameterType::Array,
        required: false,
        default: Some(serde_json::json!(["correctness", "design", "tests"])),
    });

    Skill::new(metadata, "pr_review").with_config(serde_json::json!({
        "auto_description": true,
        "check_ci": true
    }))
}

/// 创建设计评审技能
pub fn design_review() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("design-review"),
        "design-review",
        "设计评审技能。评审系统设计，发现架构问题和改进点。",
    )
    .with_category(SkillCategory::Review)
    .with_tag("review")
    .with_tag("design")
    .with_tag("architecture")
    .with_parameter(SkillParameter {
        name: "design_doc".to_string(),
        description: "设计文档".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "criteria".to_string(),
        description: "评审标准".to_string(),
        param_type: ParameterType::Array,
        required: false,
        default: Some(serde_json::json!([
            "scalability",
            "maintainability",
            "security",
            "performance"
        ])),
    });

    Skill::new(metadata, "design_review").with_config(serde_json::json!({
        "score_design": true,
        "provide_alternatives": true
    }))
}

/// 创建结对编程技能
pub fn pair_programming() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("pair-programming"),
        "pair-programming",
        "结对编程技能。模拟结对编程场景，驾驶员和导航员角色切换。",
    )
    .with_category(SkillCategory::Collaboration)
    .with_tag("pair")
    .with_tag("programming")
    .with_tag("collaboration")
    .with_parameter(SkillParameter {
        name: "task".to_string(),
        description: "任务描述".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "driver".to_string(),
        description: "驾驶员".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "navigator".to_string(),
        description: "导航员".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    });

    Skill::new(metadata, "pair_programming").with_config(serde_json::json!({
        "switch_interval_minutes": 15,
        "record_session": true
    }))
}

/// 创建团队回顾技能
pub fn team_retrospective() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("team-retrospective"),
        "team-retrospective",
        "团队回顾技能。帮助团队进行迭代回顾，总结经验教训。",
    )
    .with_category(SkillCategory::Collaboration)
    .with_tag("retrospective")
    .with_tag("team")
    .with_tag("improvement")
    .with_parameter(SkillParameter {
        name: "sprint".to_string(),
        description: "冲刺/迭代名称".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "what_went_well".to_string(),
        description: "做得好的方面".to_string(),
        param_type: ParameterType::Array,
        required: false,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "what_to_improve".to_string(),
        description: "需要改进的方面".to_string(),
        param_type: ParameterType::Array,
        required: false,
        default: None,
    });

    Skill::new(metadata, "team_retrospective").with_config(serde_json::json!({
        "format": "start_stop_continue",
        "action_items": true
    }))
}

/// 创建知识分享技能
pub fn knowledge_sharing() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("knowledge-sharing"),
        "knowledge-sharing",
        "知识分享技能。组织和促进技术知识分享会议。",
    )
    .with_category(SkillCategory::Collaboration)
    .with_tag("knowledge")
    .with_tag("sharing")
    .with_tag("presentation")
    .with_parameter(SkillParameter {
        name: "topic".to_string(),
        description: "分享主题".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "audience".to_string(),
        description: "受众群体".to_string(),
        param_type: ParameterType::Array,
        required: false,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "duration_minutes".to_string(),
        description: "时长（分钟）".to_string(),
        param_type: ParameterType::Integer,
        required: false,
        default: Some(serde_json::json!(30)),
    });

    Skill::new(metadata, "knowledge_sharing").with_config(serde_json::json!({
        "include_examples": true,
        "include_quiz": true
    }))
}

/// 获取所有协作技能
pub fn all_collaboration_skills() -> Vec<Skill> {
    vec![
        code_review(),
        pr_review(),
        design_review(),
        pair_programming(),
        team_retrospective(),
        knowledge_sharing(),
    ]
}
