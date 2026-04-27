//! 开发技能
//!
//! 提供软件开发相关的技能。

use crate::{ParameterType, Skill, SkillCategory, SkillId, SkillMetadata, SkillParameter};

// ============================================================================
// 测试技能
// ============================================================================

/// 创建单元测试技能
pub fn unit_test() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("unit-test"),
        "unit-test",
        "单元测试技能。为代码生成单元测试。",
    )
    .with_category(SkillCategory::Testing)
    .with_tag("testing")
    .with_tag("unit")
    .with_parameter(SkillParameter {
        name: "code".to_string(),
        description: "要测试的代码".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "framework".to_string(),
        description: "测试框架".to_string(),
        param_type: ParameterType::String,
        required: false,
        default: Some(serde_json::json!("default")),
    });

    Skill::new(metadata, "unit_test").with_config(serde_json::json!({
        "min_coverage": 80,
        "mock_dependencies": true
    }))
}

/// 创建集成测试技能
pub fn integration_test() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("integration-test"),
        "integration-test",
        "集成测试技能。编写集成测试验证组件间的交互。",
    )
    .with_category(SkillCategory::Testing)
    .with_tag("testing")
    .with_tag("integration")
    .with_parameter(SkillParameter {
        name: "components".to_string(),
        description: "要测试的组件列表".to_string(),
        param_type: ParameterType::Array,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "test_data".to_string(),
        description: "测试数据配置".to_string(),
        param_type: ParameterType::Object,
        required: false,
        default: None,
    });

    Skill::new(metadata, "integration_test").with_config(serde_json::json!({
        "use_test_db": true,
        "cleanup_after": true
    }))
}

/// 创建 E2E 测试技能
pub fn e2e_test() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("e2e-test"),
        "e2e-test",
        "端到端测试技能。编写端到端测试验证完整用户流程。",
    )
    .with_category(SkillCategory::Testing)
    .with_tag("testing")
    .with_tag("e2e")
    .with_tag("user-flow")
    .with_parameter(SkillParameter {
        name: "user_flow".to_string(),
        description: "用户流程描述".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "browser".to_string(),
        description: "浏览器类型".to_string(),
        param_type: ParameterType::String,
        required: false,
        default: Some(serde_json::json!("chromium")),
    });

    Skill::new(metadata, "e2e_test").with_config(serde_json::json!({
        "headless": true,
        "screenshot_on_failure": true
    }))
}

/// 创建性能测试技能
pub fn performance_test() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("performance-test"),
        "performance-test",
        "性能测试技能。进行性能基准测试和负载测试。",
    )
    .with_category(SkillCategory::Testing)
    .with_tag("testing")
    .with_tag("performance")
    .with_tag("benchmark")
    .with_parameter(SkillParameter {
        name: "endpoint".to_string(),
        description: "测试端点".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "concurrent_users".to_string(),
        description: "并发用户数".to_string(),
        param_type: ParameterType::Integer,
        required: false,
        default: Some(serde_json::json!(100)),
    });

    Skill::new(metadata, "performance_test").with_config(serde_json::json!({
        "duration_seconds": 60,
        "report_latency": true
    }))
}

// ============================================================================
// 文档技能
// ============================================================================

/// 创建 API 文档技能
pub fn api_documentation() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("api-documentation"),
        "api-documentation",
        "API 文档生成技能。从代码生成 API 文档。",
    )
    .with_category(SkillCategory::Documentation)
    .with_tag("documentation")
    .with_tag("api")
    .with_parameter(SkillParameter {
        name: "api_spec".to_string(),
        description: "API 规范或代码".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "format".to_string(),
        description: "输出格式".to_string(),
        param_type: ParameterType::String,
        required: false,
        default: Some(serde_json::json!("openapi")),
    });

    Skill::new(metadata, "api_documentation").with_config(serde_json::json!({
        "include_examples": true,
        "include_errors": true
    }))
}

/// 创建 README 生成技能
pub fn readme_generator() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("readme-generator"),
        "readme-generator",
        "README 生成技能。生成项目 README 文档。",
    )
    .with_category(SkillCategory::Documentation)
    .with_tag("documentation")
    .with_tag("readme")
    .with_tag("generator")
    .with_parameter(SkillParameter {
        name: "project".to_string(),
        description: "项目信息".to_string(),
        param_type: ParameterType::Object,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "sections".to_string(),
        description: "包含的章节".to_string(),
        param_type: ParameterType::Array,
        required: false,
        default: Some(serde_json::json!([
            "installation",
            "usage",
            "examples",
            "license"
        ])),
    });

    Skill::new(metadata, "readme_generator").with_config(serde_json::json!({
        "badges": true,
        "toc": true
    }))
}

// ============================================================================
// 重构技能
// ============================================================================

/// 创建重构技能
pub fn refactor() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("refactor"),
        "refactor",
        "代码重构技能。分析代码并提出重构建议。",
    )
    .with_category(SkillCategory::Development)
    .with_tag("refactor")
    .with_tag("code-quality")
    .with_parameter(SkillParameter {
        name: "code".to_string(),
        description: "要重构的代码".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "target".to_string(),
        description: "重构目标".to_string(),
        param_type: ParameterType::String,
        required: false,
        default: Some(serde_json::json!("readability")),
    });

    Skill::new(metadata, "refactor").with_config(serde_json::json!({
        "preserve_behavior": true,
        "incremental": true
    }))
}

/// 创建死代码清理技能
pub fn dead_code_cleanup() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("dead-code-cleanup"),
        "dead-code-cleanup",
        "死代码清理技能。识别并清理未使用的代码。",
    )
    .with_category(SkillCategory::Development)
    .with_tag("cleanup")
    .with_tag("maintenance")
    .with_tag("refactor")
    .with_parameter(SkillParameter {
        name: "scope".to_string(),
        description: "分析范围".to_string(),
        param_type: ParameterType::String,
        required: false,
        default: Some(serde_json::json!("project")),
    })
    .with_parameter(SkillParameter {
        name: "languages".to_string(),
        description: "语言列表".to_string(),
        param_type: ParameterType::Array,
        required: false,
        default: None,
    });

    Skill::new(metadata, "dead_code_cleanup").with_config(serde_json::json!({
        "analyze_imports": true,
        "analyze_symbols": true
    }))
}

// ============================================================================
// 安全技能
// ============================================================================

/// 创建安全扫描技能
pub fn security_scan() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("security-scan"),
        "security-scan",
        "安全扫描技能。扫描代码中的安全漏洞。",
    )
    .with_category(SkillCategory::Review)
    .with_tag("security")
    .with_tag("scan")
    .with_tag("vulnerability")
    .with_parameter(SkillParameter {
        name: "target".to_string(),
        description: "扫描目标".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "scan_types".to_string(),
        description: "扫描类型".to_string(),
        param_type: ParameterType::Array,
        required: false,
        default: Some(serde_json::json!(["owasp", "cwe", "sql-injection", "xss"])),
    });

    Skill::new(metadata, "security_scan").with_config(serde_json::json!({
        "severity_threshold": "medium",
        "include_remediation": true
    }))
}

/// 创建依赖审计技能
pub fn dependency_audit() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("dependency-audit"),
        "dependency-audit",
        "依赖审计技能。审计项目依赖的安全漏洞和许可证问题。",
    )
    .with_category(SkillCategory::Review)
    .with_tag("security")
    .with_tag("dependencies")
    .with_tag("audit")
    .with_parameter(SkillParameter {
        name: "project_path".to_string(),
        description: "项目路径".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "check_licenses".to_string(),
        description: "检查许可证".to_string(),
        param_type: ParameterType::Boolean,
        required: false,
        default: Some(serde_json::json!(true)),
    });

    Skill::new(metadata, "dependency_audit").with_config(serde_json::json!({
        "check_updates": true,
        "auto_fix": false
    }))
}

// ============================================================================
// 研究技能
// ============================================================================

/// 创建技术调研技能
pub fn tech_research() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("tech-research"),
        "tech-research",
        "技术调研技能。调研新技术并生成分析报告。",
    )
    .with_category(SkillCategory::Research)
    .with_tag("research")
    .with_tag("technology")
    .with_tag("analysis")
    .with_parameter(SkillParameter {
        name: "topic".to_string(),
        description: "调研主题".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "depth".to_string(),
        description: "调研深度".to_string(),
        param_type: ParameterType::String,
        required: false,
        default: Some(serde_json::json!("medium")),
    });

    Skill::new(metadata, "tech_research").with_config(serde_json::json!({
        "include_pros_cons": true,
        "include_alternatives": true,
        "include_use_cases": true
    }))
}

/// 创建竞品分析技能
pub fn competitor_analysis() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("competitor-analysis"),
        "competitor-analysis",
        "竞品分析技能。分析竞争对手的产品和功能。",
    )
    .with_category(SkillCategory::Research)
    .with_tag("research")
    .with_tag("competitor")
    .with_tag("analysis")
    .with_parameter(SkillParameter {
        name: "competitors".to_string(),
        description: "竞品列表".to_string(),
        param_type: ParameterType::Array,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "aspects".to_string(),
        description: "分析维度".to_string(),
        param_type: ParameterType::Array,
        required: false,
        default: Some(serde_json::json!([
            "features",
            "pricing",
            "ux",
            "performance"
        ])),
    });

    Skill::new(metadata, "competitor_analysis").with_config(serde_json::json!({
        "comparison_table": true,
        "recommendations": true
    }))
}

// ============================================================================
// 调试技能
// ============================================================================

/// 创建调试技能
pub fn debug() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("debug"),
        "debug",
        "调试技能。帮助诊断和修复代码问题。",
    )
    .with_category(SkillCategory::Development)
    .with_tag("debug")
    .with_tag("troubleshooting")
    .with_tag("fix")
    .with_parameter(SkillParameter {
        name: "error".to_string(),
        description: "错误信息".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "context".to_string(),
        description: "上下文代码".to_string(),
        param_type: ParameterType::String,
        required: false,
        default: None,
    });

    Skill::new(metadata, "debug").with_config(serde_json::json!({
        "root_cause_analysis": true,
        "suggest_fixes": true
    }))
}

/// 创建日志分析技能
pub fn log_analysis() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("log-analysis"),
        "log-analysis",
        "日志分析技能。分析应用日志定位问题。",
    )
    .with_category(SkillCategory::Development)
    .with_tag("debug")
    .with_tag("logs")
    .with_tag("analysis")
    .with_parameter(SkillParameter {
        name: "logs".to_string(),
        description: "日志内容".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "error_pattern".to_string(),
        description: "错误模式".to_string(),
        param_type: ParameterType::String,
        required: false,
        default: None,
    });

    Skill::new(metadata, "log_analysis").with_config(serde_json::json!({
        "parse_format": "auto",
        "summarize": true
    }))
}

// ============================================================================
// 部署技能
// ============================================================================

/// 创建部署规划技能
pub fn deployment_plan() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("deployment-plan"),
        "deployment-plan",
        "部署规划技能。规划应用程序的部署策略。",
    )
    .with_category(SkillCategory::WorkflowPlanning)
    .with_tag("deployment")
    .with_tag("planning")
    .with_tag("devops")
    .with_parameter(SkillParameter {
        name: "application".to_string(),
        description: "应用程序信息".to_string(),
        param_type: ParameterType::Object,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "environment".to_string(),
        description: "目标环境".to_string(),
        param_type: ParameterType::String,
        required: false,
        default: Some(serde_json::json!("production")),
    });

    Skill::new(metadata, "deployment_plan").with_config(serde_json::json!({
        "strategy": "blue_green",
        "rollback_plan": true,
        "health_checks": true
    }))
}

/// 创建容器化技能
pub fn containerize() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("containerize"),
        "containerize",
        "应用容器化技能。为应用程序创建 Dockerfile 和容器配置。",
    )
    .with_category(SkillCategory::Development)
    .with_tag("docker")
    .with_tag("container")
    .with_tag("devops")
    .with_parameter(SkillParameter {
        name: "project_path".to_string(),
        description: "项目路径".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "base_image".to_string(),
        description: "基础镜像".to_string(),
        param_type: ParameterType::String,
        required: false,
        default: None,
    });

    Skill::new(metadata, "containerize").with_config(serde_json::json!({
        "multi_stage": true,
        "optimize_size": true
    }))
}

// ============================================================================
// 数据库技能
// ============================================================================

/// 创建数据库迁移技能
pub fn db_migration() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("db-migration"),
        "db-migration",
        "数据库迁移技能。生成和管理数据库迁移脚本。",
    )
    .with_category(SkillCategory::Development)
    .with_tag("database")
    .with_tag("migration")
    .with_tag("devops")
    .with_parameter(SkillParameter {
        name: "changes".to_string(),
        description: "数据库变更".to_string(),
        param_type: ParameterType::Object,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "direction".to_string(),
        description: "迁移方向".to_string(),
        param_type: ParameterType::String,
        required: false,
        default: Some(serde_json::json!("up")),
    });

    Skill::new(metadata, "db_migration").with_config(serde_json::json!({
        "generate_rollback": true,
        "seed_data": true
    }))
}

/// 创建查询优化技能
pub fn query_optimization() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("query-optimization"),
        "query-optimization",
        "SQL 查询优化技能。分析和优化 SQL 查询性能。",
    )
    .with_category(SkillCategory::Development)
    .with_tag("database")
    .with_tag("performance")
    .with_tag("optimization")
    .with_parameter(SkillParameter {
        name: "query".to_string(),
        description: "SQL 查询".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "database_type".to_string(),
        description: "数据库类型".to_string(),
        param_type: ParameterType::String,
        required: false,
        default: Some(serde_json::json!("postgresql")),
    });

    Skill::new(metadata, "query_optimization").with_config(serde_json::json!({
        "explain_analyze": true,
        "suggest_indexes": true
    }))
}

// ============================================================================
// API 开发技能
// ============================================================================

/// 创建 REST API 设计技能
pub fn rest_api_design() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("rest-api-design"),
        "rest-api-design",
        "REST API 设计技能。设计和文档化 RESTful API。",
    )
    .with_category(SkillCategory::Development)
    .with_tag("api")
    .with_tag("rest")
    .with_tag("design")
    .with_parameter(SkillParameter {
        name: "resources".to_string(),
        description: "API 资源".to_string(),
        param_type: ParameterType::Array,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "version".to_string(),
        description: "API 版本".to_string(),
        param_type: ParameterType::String,
        required: false,
        default: Some(serde_json::json!("v1")),
    });

    Skill::new(metadata, "rest_api_design").with_config(serde_json::json!({
        "openapi_output": true,
        "include_auth": true
    }))
}

/// 创建 GraphQL 技能
pub fn graphql_schema_design() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("graphql-schema-design"),
        "graphql-schema-design",
        "GraphQL Schema 设计技能。设计和生成 GraphQL Schema。",
    )
    .with_category(SkillCategory::Development)
    .with_tag("api")
    .with_tag("graphql")
    .with_tag("design")
    .with_parameter(SkillParameter {
        name: "types".to_string(),
        description: "类型定义".to_string(),
        param_type: ParameterType::Array,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "queries".to_string(),
        description: "查询定义".to_string(),
        param_type: ParameterType::Array,
        required: false,
        default: None,
    });

    Skill::new(metadata, "graphql_schema_design").with_config(serde_json::json!({
        "generate_resolvers": true,
        "include_subscriptions": false
    }))
}

// ============================================================================
// CI/CD 技能
// ============================================================================

/// 创建 CI 配置技能
pub fn ci_config() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("ci-config"),
        "ci-config",
        "CI 配置技能。生成持续集成配置文件。",
    )
    .with_category(SkillCategory::Development)
    .with_tag("ci")
    .with_tag("cd")
    .with_tag("devops")
    .with_parameter(SkillParameter {
        name: "platform".to_string(),
        description: "CI 平台".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: Some(serde_json::json!("github-actions")),
    })
    .with_parameter(SkillParameter {
        name: "language".to_string(),
        description: "编程语言".to_string(),
        param_type: ParameterType::String,
        required: false,
        default: None,
    });

    Skill::new(metadata, "ci_config").with_config(serde_json::json!({
        "run_tests": true,
        "run_lint": true,
        "build_docker": false
    }))
}

// ============================================================================
// 通用技能
// ============================================================================

/// 创建代码生成技能
pub fn code_generator() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("code-generator"),
        "code-generator",
        "代码生成技能。根据模板或规范生成代码。",
    )
    .with_category(SkillCategory::Development)
    .with_tag("generator")
    .with_tag("scaffolding")
    .with_tag("boilerplate")
    .with_parameter(SkillParameter {
        name: "template".to_string(),
        description: "代码模板".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "context".to_string(),
        description: "上下文数据".to_string(),
        param_type: ParameterType::Object,
        required: true,
        default: None,
    });

    Skill::new(metadata, "code_generator").with_config(serde_json::json!({
        "validate_output": true,
        "format_code": true
    }))
}

/// 创建代码翻译技能
pub fn code_translate() -> Skill {
    let metadata = SkillMetadata::new(
        SkillId::new("code-translate"),
        "code-translate",
        "代码翻译技能。将代码从一种语言翻译到另一种语言。",
    )
    .with_category(SkillCategory::Development)
    .with_tag("translate")
    .with_tag("conversion")
    .with_tag("porting")
    .with_parameter(SkillParameter {
        name: "code".to_string(),
        description: "源代码".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "from_language".to_string(),
        description: "源语言".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    })
    .with_parameter(SkillParameter {
        name: "to_language".to_string(),
        description: "目标语言".to_string(),
        param_type: ParameterType::String,
        required: true,
        default: None,
    });

    Skill::new(metadata, "code_translate").with_config(serde_json::json!({
        "preserve_comments": true,
        "idiomatic_output": true
    }))
}

/// 获取所有开发技能
pub fn all_development_skills() -> Vec<Skill> {
    vec![
        // 测试
        unit_test(),
        integration_test(),
        e2e_test(),
        performance_test(),
        // 文档
        api_documentation(),
        readme_generator(),
        // 重构
        refactor(),
        dead_code_cleanup(),
        // 安全
        security_scan(),
        dependency_audit(),
        // 研究
        tech_research(),
        competitor_analysis(),
        // 调试
        debug(),
        log_analysis(),
        // 部署
        deployment_plan(),
        containerize(),
        // 数据库
        db_migration(),
        query_optimization(),
        // API
        rest_api_design(),
        graphql_schema_design(),
        // CI/CD
        ci_config(),
        // 通用
        code_generator(),
        code_translate(),
    ]
}
