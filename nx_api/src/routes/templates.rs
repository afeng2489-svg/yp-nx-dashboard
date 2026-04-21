//! 工作流模板路由

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use super::AppState;

/// 模板元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateMetadata {
    pub name: String,
    pub description: String,
    pub category: String,
}

/// 模板定义（完整结构）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateDefinition {
    pub name: String,
    pub description: String,
    pub category: String,
    pub stages: Vec<Stage>,
    pub agents: Vec<Agent>,
    #[serde(default)]
    pub variables: std::collections::HashMap<String, serde_json::Value>,
}

/// 阶段定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stage {
    pub name: String,
    pub agents: Vec<String>,
    #[serde(default)]
    pub parallel: bool,
}

/// Agent 定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub role: String,
    pub model: String,
    pub prompt: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
}

/// 模板摘要响应
#[derive(Debug, Serialize, Deserialize)]
pub struct TemplateSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub stage_count: usize,
    pub agent_count: usize,
}

/// 模板详情响应
#[derive(Debug, Serialize, Deserialize)]
pub struct TemplateResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub stages: Vec<Stage>,
    pub agents: Vec<Agent>,
    pub variables: std::collections::HashMap<String, serde_json::Value>,
}

/// 创建模板请求
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTemplateRequest {
    pub name: String,
    pub description: String,
    pub category: String,
    pub stages: Vec<Stage>,
    pub agents: Vec<Agent>,
}

/// 实例化模板请求
#[derive(Debug, Serialize, Deserialize)]
pub struct InstantiateTemplateRequest {
    #[serde(default)]
    pub variables: Option<serde_json::Value>,
}

/// 列表响应
#[derive(Debug, Serialize, Deserialize)]
pub struct ListResponse<T> {
    pub items: Vec<T>,
    pub total: usize,
}

/// 应用状态扩展 - 包含模板路径
pub struct TemplateState {
    pub templates_path: PathBuf,
}

/// 获取模板目录路径
fn get_templates_path() -> PathBuf {
    PathBuf::from(std::env::var("TEMPLATES_PATH").unwrap_or_else(|_| {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(PathBuf::new)
            .join("templates")
            .to_string_lossy()
            .to_string()
    }))
}

/// 列出所有模板
pub async fn list_templates(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<ListResponse<TemplateSummary>>, AppError> {
    let templates_path = get_templates_path();

    let mut templates = Vec::new();

    if templates_path.exists() {
        for entry in fs::read_dir(&templates_path).map_err(|e| AppError::Internal(e.to_string()))? {
            let entry = entry.map_err(|e| AppError::Internal(e.to_string()))?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                match parse_template_file(&path) {
                    Ok((id, template)) => {
                        templates.push(TemplateSummary {
                            id,
                            name: template.name.clone(),
                            description: template.description.clone(),
                            category: template.category.clone(),
                            stage_count: template.stages.len(),
                            agent_count: template.agents.len(),
                        });
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse template {:?}: {}", path, e);
                    }
                }
            }
        }
    }

    // 当文件系统中没有模板时，返回内置中文模板
    if templates.is_empty() {
        for (id, template) in get_builtin_templates() {
            templates.push(TemplateSummary {
                id,
                name: template.name,
                description: template.description,
                category: template.category,
                stage_count: template.stages.len(),
                agent_count: template.agents.len(),
            });
        }
    }

    // 按名称排序
    templates.sort_by(|a, b| a.name.cmp(&b.name));

    let total = templates.len();

    Ok(Json(ListResponse { items: templates, total }))
}

/// 获取单个模板
pub async fn get_template(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<TemplateResponse>, AppError> {
    let templates_path = get_templates_path();
    let template_path = templates_path.join(format!("{}.yaml", id));

    if template_path.exists() {
        let (template_id, template) = parse_template_file(&template_path)?;
        return Ok(Json(TemplateResponse {
            id: template_id,
            name: template.name,
            description: template.description,
            category: template.category,
            stages: template.stages,
            agents: template.agents,
            variables: template.variables,
        }));
    }

    // 尝试从内置模板中查找
    for (builtin_id, template) in get_builtin_templates() {
        if builtin_id == id {
            return Ok(Json(TemplateResponse {
                id: builtin_id,
                name: template.name,
                description: template.description,
                category: template.category,
                stages: template.stages,
                agents: template.agents,
                variables: template.variables,
            }));
        }
    }

    Err(AppError::NotFound(format!("Template '{}' not found", id)))
}

/// 创建模板
pub async fn create_template(
    State(_state): State<Arc<AppState>>,
    Json(payload): Json<CreateTemplateRequest>,
) -> Result<Json<TemplateResponse>, AppError> {
    let templates_path = get_templates_path();

    // 确保目录存在
    fs::create_dir_all(&templates_path).map_err(|e| AppError::Internal(e.to_string()))?;

    // 生成 ID
    let id = payload.name.replace(' ', "-").to_lowercase();
    let template_path = templates_path.join(format!("{}.yaml", id));

    if template_path.exists() {
        return Err(AppError::BadRequest(format!("Template '{}' already exists", id)));
    }

    // 创建模板定义
    let template_def = TemplateDefinition {
        name: payload.name.clone(),
        description: payload.description.clone(),
        category: payload.category.clone(),
        stages: payload.stages.clone(),
        agents: payload.agents.clone(),
        variables: std::collections::HashMap::new(),
    };

    // 序列化为 YAML
    let yaml = serde_yaml::to_string(&template_def).map_err(|e| AppError::Internal(e.to_string()))?;

    // 写入文件
    fs::write(&template_path, yaml).map_err(|e| AppError::Internal(e.to_string()))?;

    tracing::info!("Created template '{}' at {:?}", id, template_path);

    Ok(Json(TemplateResponse {
        id,
        name: payload.name,
        description: payload.description,
        category: payload.category,
        stages: payload.stages,
        agents: payload.agents,
        variables: std::collections::HashMap::new(),
    }))
}

/// 从模板实例化工作流
pub async fn instantiate_template(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<InstantiateTemplateRequest>,
) -> Result<Json<InstantiateResponse>, AppError> {
    let templates_path = get_templates_path();
    let template_path = templates_path.join(format!("{}.yaml", id));

    // 优先检查 config/workflows/{id}.yaml（支持完整 stage_type/user_input 等特性）
    let config_workflow_path = std::path::PathBuf::from(
        std::env::var("NEXUS_BASE_DIR")
            .unwrap_or_else(|_| "/Users/Zhuanz/Desktop/yp-nx-dashboard".to_string())
    )
    .join("config/workflows")
    .join(format!("{}.yaml", id));

    let (workflow_name, workflow_description, workflow_definition) = if config_workflow_path.exists() {
        // 直接读取完整 YAML，用 serde_json::Value 保留所有字段（含 stage_type 等）
        let yaml_content = fs::read_to_string(&config_workflow_path)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let mut yaml_val: serde_json::Value = serde_yaml::from_str(&yaml_content)
            .map_err(|e| AppError::Internal(format!("YAML 解析失败: {}", e)))?;

        // 将用户传入变量注入 variables 字段（非空值覆盖 YAML 默认值）
        if let Some(user_vars) = payload.variables.as_ref().and_then(|v| v.as_object()) {
            let vars = yaml_val.get_mut("variables").and_then(|v| v.as_object_mut());
            if let Some(vars_obj) = vars {
                for (k, v) in user_vars {
                    if let Some(s) = v.as_str() {
                        if !s.is_empty() {
                            vars_obj.insert(k.clone(), v.clone());
                        }
                    }
                }
            }
        }

        let name = yaml_val.get("name").and_then(|v| v.as_str()).unwrap_or(&id).to_string();
        let desc = yaml_val.get("description").and_then(|v| v.as_str()).map(|s| s.to_string());
        (name, desc, yaml_val)
    } else {
        // 从模板文件系统或内置模板获取定义（简单模板，不含 user_input）
        let template_def = if template_path.exists() {
            let (_, def) = parse_template_file(&template_path)?;
            def
        } else {
            get_builtin_templates()
                .into_iter()
                .find(|(builtin_id, _)| builtin_id == &id)
                .map(|(_, def)| def)
                .ok_or_else(|| AppError::NotFound(format!("Template '{}' not found", id)))?
        };

        let definition = serde_json::json!({
            "stages": template_def.stages,
            "agents": template_def.agents,
            "variables": payload.variables.unwrap_or(serde_json::json!({})),
        });
        (template_def.name, Some(template_def.description), definition)
    };

    // 使用工作流服务创建工作流
    let workflow = state
        .workflow_service
        .create_workflow(
            workflow_name,
            Some("1.0.0".to_string()),
            workflow_description,
            workflow_definition,
        )
        .map_err(AppError::from)?;

    tracing::info!(
        "Instantiated template '{}' into workflow '{}'",
        id,
        workflow.id
    );

    Ok(Json(InstantiateResponse {
        workflow_id: workflow.id,
        name: workflow.name,
        description: workflow.description,
        created_at: workflow.created_at.to_rfc3339(),
    }))
}

/// 实例化响应
#[derive(Debug, Serialize, Deserialize)]
pub struct InstantiateResponse {
    pub workflow_id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
}

/// 解析模板文件
fn parse_template_file(path: &std::path::Path) -> Result<(String, TemplateDefinition), AppError> {
    let content = fs::read_to_string(path).map_err(|e| AppError::Internal(e.to_string()))?;

    let template: TemplateDefinition =
        serde_yaml::from_str(&content).map_err(|e| AppError::Internal(e.to_string()))?;

    let id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    Ok((id, template))
}

/// 获取内置中文模板（当文件系统模板为空时作为 fallback）
fn get_builtin_templates() -> Vec<(String, TemplateDefinition)> {
    vec![
        (
            "review-cycle".to_string(),
            TemplateDefinition {
                name: "代码审查工作流".to_string(),
                description: "预分析目标代码，暂停等待用户选择审查深度，然后执行对应深度的审查并生成报告".to_string(),
                category: "development".to_string(),
                stages: vec![
                    Stage { name: "预分析".to_string(), agents: vec!["pre-analyzer".to_string()], parallel: false },
                    Stage { name: "代码审查".to_string(), agents: vec!["reviewer".to_string()], parallel: false },
                    Stage { name: "生成报告".to_string(), agents: vec!["reporter".to_string()], parallel: false },
                ],
                agents: vec![
                    Agent {
                        id: "pre-analyzer".to_string(),
                        role: "预分析器".to_string(),
                        model: "claude-haiku-4-5".to_string(),
                        prompt: "你是代码预分析专家。请快速扫描目标代码或功能描述，识别语言、框架、代码规模、主要模块，输出简洁的预分析摘要（不超过200字）。目标：{{target}}".to_string(),
                        depends_on: vec![],
                    },
                    Agent {
                        id: "reviewer".to_string(),
                        role: "代码审查员".to_string(),
                        model: "claude-sonnet-4-6".to_string(),
                        prompt: "你是经验丰富的代码审查员。审查模式：{{review_mode}}。目标：{{target}}。根据选择的审查模式执行对应深度的审查：快速扫描仅关注高危问题；标准审查全面检查代码质量；深度分析包含架构评估和安全审计。".to_string(),
                        depends_on: vec!["pre-analyzer".to_string()],
                    },
                    Agent {
                        id: "reporter".to_string(),
                        role: "报告生成器".to_string(),
                        model: "claude-haiku-4-5".to_string(),
                        prompt: "整合预分析和代码审查结果，生成结构清晰的审查报告。报告需包含：执行摘要、问题列表（按严重程度排序）、改进建议、总体评分。".to_string(),
                        depends_on: vec!["reviewer".to_string()],
                    },
                ],
                variables: {
                    let mut m = std::collections::HashMap::new();
                    m.insert("target".to_string(), serde_json::Value::String(String::new()));
                    m
                },
            },
        ),
        (
            "code-review".to_string(),
            TemplateDefinition {
                name: "代码审查".to_string(),
                description: "自动化代码质量检查，包括可读性、性能、安全漏洞分析，并生成详细报告".to_string(),
                category: "development".to_string(),
                stages: vec![
                    Stage { name: "代码分析".to_string(), agents: vec!["reviewer".to_string()], parallel: false },
                    Stage { name: "安全检查".to_string(), agents: vec!["security-checker".to_string()], parallel: false },
                    Stage { name: "生成报告".to_string(), agents: vec!["reporter".to_string()], parallel: false },
                ],
                agents: vec![
                    Agent {
                        id: "reviewer".to_string(),
                        role: "代码审查员".to_string(),
                        model: "claude-sonnet-4-6".to_string(),
                        prompt: "你是一位经验丰富的代码审查员。请仔细分析代码的可读性、性能和最佳实践，指出需要改进的地方。".to_string(),
                        depends_on: vec![],
                    },
                    Agent {
                        id: "security-checker".to_string(),
                        role: "安全专家".to_string(),
                        model: "claude-sonnet-4-6".to_string(),
                        prompt: "你是安全专家。请检查代码中的安全漏洞，包括 SQL 注入、XSS、CSRF 等常见安全问题。".to_string(),
                        depends_on: vec!["reviewer".to_string()],
                    },
                    Agent {
                        id: "reporter".to_string(),
                        role: "报告生成器".to_string(),
                        model: "claude-haiku-4-5".to_string(),
                        prompt: "整合代码审查和安全检查结果，生成结构清晰的审查报告，包含问题列表和改进建议。".to_string(),
                        depends_on: vec!["reviewer".to_string(), "security-checker".to_string()],
                    },
                ],
                variables: std::collections::HashMap::new(),
            },
        ),
        (
            "bug-investigation".to_string(),
            TemplateDefinition {
                name: "Bug 调查".to_string(),
                description: "系统性分析 Bug 根因，提出修复方案并验证，适合复杂问题排查".to_string(),
                category: "development".to_string(),
                stages: vec![
                    Stage { name: "问题分析".to_string(), agents: vec!["analyzer".to_string()], parallel: false },
                    Stage { name: "方案制定".to_string(), agents: vec!["solver".to_string()], parallel: false },
                    Stage { name: "验证".to_string(), agents: vec!["validator".to_string()], parallel: false },
                ],
                agents: vec![
                    Agent {
                        id: "analyzer".to_string(),
                        role: "问题分析师".to_string(),
                        model: "claude-opus-4-6".to_string(),
                        prompt: "你是问题分析专家。请系统性地分析 Bug 现象、复现步骤，定位根本原因，提供详细的诊断报告。".to_string(),
                        depends_on: vec![],
                    },
                    Agent {
                        id: "solver".to_string(),
                        role: "解决方案专家".to_string(),
                        model: "claude-sonnet-4-6".to_string(),
                        prompt: "根据问题分析结果，制定具体的修复方案，包括代码修改建议和影响范围评估。".to_string(),
                        depends_on: vec!["analyzer".to_string()],
                    },
                    Agent {
                        id: "validator".to_string(),
                        role: "验证工程师".to_string(),
                        model: "claude-haiku-4-5".to_string(),
                        prompt: "验证修复方案的正确性，设计验证测试用例，确认 Bug 已被修复且未引入新问题。".to_string(),
                        depends_on: vec!["solver".to_string()],
                    },
                ],
                variables: std::collections::HashMap::new(),
            },
        ),
        (
            "requirements-analysis".to_string(),
            TemplateDefinition {
                name: "需求分析".to_string(),
                description: "从用户需求到技术规格，包含需求拆解、用例设计和技术方案制定".to_string(),
                category: "planning".to_string(),
                stages: vec![
                    Stage { name: "需求理解".to_string(), agents: vec!["analyst".to_string()], parallel: false },
                    Stage { name: "技术规划".to_string(), agents: vec!["architect".to_string()], parallel: false },
                    Stage { name: "输出文档".to_string(), agents: vec!["documenter".to_string()], parallel: false },
                ],
                agents: vec![
                    Agent {
                        id: "analyst".to_string(),
                        role: "需求分析师".to_string(),
                        model: "claude-opus-4-6".to_string(),
                        prompt: "你是需求分析专家。请深入理解用户需求，提炼核心功能点，识别潜在歧义，生成结构化需求列表。".to_string(),
                        depends_on: vec![],
                    },
                    Agent {
                        id: "architect".to_string(),
                        role: "技术架构师".to_string(),
                        model: "claude-opus-4-6".to_string(),
                        prompt: "根据需求分析结果，设计技术实现方案，包括系统架构、接口设计、数据模型和技术选型。".to_string(),
                        depends_on: vec!["analyst".to_string()],
                    },
                    Agent {
                        id: "documenter".to_string(),
                        role: "文档工程师".to_string(),
                        model: "claude-haiku-4-5".to_string(),
                        prompt: "将需求分析和技术方案整理成规范的技术文档，包括 PRD、接口文档和开发计划。".to_string(),
                        depends_on: vec!["analyst".to_string(), "architect".to_string()],
                    },
                ],
                variables: std::collections::HashMap::new(),
            },
        ),
        (
            "tdd-development".to_string(),
            TemplateDefinition {
                name: "TDD 测试驱动开发".to_string(),
                description: "严格遵循 TDD 流程：先写测试，再实现代码，最后重构优化".to_string(),
                category: "testing".to_string(),
                stages: vec![
                    Stage { name: "编写测试".to_string(), agents: vec!["test-writer".to_string()], parallel: false },
                    Stage { name: "功能实现".to_string(), agents: vec!["developer".to_string()], parallel: false },
                    Stage { name: "代码重构".to_string(), agents: vec!["refactorer".to_string()], parallel: false },
                ],
                agents: vec![
                    Agent {
                        id: "test-writer".to_string(),
                        role: "测试工程师".to_string(),
                        model: "claude-haiku-4-5".to_string(),
                        prompt: "为下一个功能编写失败的测试用例，包括单元测试和边界条件测试，确保测试覆盖关键场景。".to_string(),
                        depends_on: vec![],
                    },
                    Agent {
                        id: "developer".to_string(),
                        role: "开发工程师".to_string(),
                        model: "claude-opus-4-6".to_string(),
                        prompt: "根据测试用例实现功能代码，以最小改动使所有测试通过，不过度设计。".to_string(),
                        depends_on: vec!["test-writer".to_string()],
                    },
                    Agent {
                        id: "refactorer".to_string(),
                        role: "重构专家".to_string(),
                        model: "claude-sonnet-4-6".to_string(),
                        prompt: "在保持测试通过的前提下，重构代码以提升可读性、消除重复、优化结构。".to_string(),
                        depends_on: vec!["developer".to_string()],
                    },
                ],
                variables: std::collections::HashMap::new(),
            },
        ),
        (
            "tech-research".to_string(),
            TemplateDefinition {
                name: "技术调研".to_string(),
                description: "全面的技术选型调研，包括方案对比、优劣分析和最终推荐".to_string(),
                category: "research".to_string(),
                stages: vec![
                    Stage { name: "方案收集".to_string(), agents: vec!["researcher".to_string()], parallel: false },
                    Stage { name: "对比分析".to_string(), agents: vec!["analyst".to_string()], parallel: false },
                    Stage { name: "决策建议".to_string(), agents: vec!["advisor".to_string()], parallel: false },
                ],
                agents: vec![
                    Agent {
                        id: "researcher".to_string(),
                        role: "技术研究员".to_string(),
                        model: "claude-sonnet-4-6".to_string(),
                        prompt: "收集业界主流技术方案，整理各方案的核心特性、使用场景、社区生态和成熟度。".to_string(),
                        depends_on: vec![],
                    },
                    Agent {
                        id: "analyst".to_string(),
                        role: "技术分析师".to_string(),
                        model: "claude-opus-4-6".to_string(),
                        prompt: "从性能、可维护性、学习成本、社区支持等维度对各技术方案进行深入对比分析。".to_string(),
                        depends_on: vec!["researcher".to_string()],
                    },
                    Agent {
                        id: "advisor".to_string(),
                        role: "技术顾问".to_string(),
                        model: "claude-opus-4-6".to_string(),
                        prompt: "结合项目实际需求，给出明确的技术选型建议和实施路径规划。".to_string(),
                        depends_on: vec!["analyst".to_string()],
                    },
                ],
                variables: std::collections::HashMap::new(),
            },
        ),
        (
            "content-creation".to_string(),
            TemplateDefinition {
                name: "内容创作".to_string(),
                description: "多智能体协作内容创作：研究、写作、编辑三步流程，高质量输出".to_string(),
                category: "writing".to_string(),
                stages: vec![
                    Stage { name: "背景研究".to_string(), agents: vec!["researcher".to_string()], parallel: false },
                    Stage { name: "内容写作".to_string(), agents: vec!["writer".to_string()], parallel: false },
                    Stage { name: "编辑润色".to_string(), agents: vec!["editor".to_string()], parallel: false },
                ],
                agents: vec![
                    Agent {
                        id: "researcher".to_string(),
                        role: "研究员".to_string(),
                        model: "claude-sonnet-4-6".to_string(),
                        prompt: "收集主题相关的背景信息、数据、案例，整理成结构化的素材库。".to_string(),
                        depends_on: vec![],
                    },
                    Agent {
                        id: "writer".to_string(),
                        role: "内容写作者".to_string(),
                        model: "claude-opus-4-6".to_string(),
                        prompt: "基于研究素材，创作结构清晰、逻辑流畅、观点深刻的内容，吸引目标读者。".to_string(),
                        depends_on: vec!["researcher".to_string()],
                    },
                    Agent {
                        id: "editor".to_string(),
                        role: "编辑".to_string(),
                        model: "claude-haiku-4-5".to_string(),
                        prompt: "对内容进行润色编辑，修正语言表达，优化段落结构，提升整体可读性和专业度。".to_string(),
                        depends_on: vec!["writer".to_string()],
                    },
                ],
                variables: std::collections::HashMap::new(),
            },
        ),
    ]
}

/// 按类别列出模板
pub async fn list_templates_by_category(
    State(_state): State<Arc<AppState>>,
    Path(category): Path<String>,
) -> Result<Json<ListResponse<TemplateSummary>>, AppError> {
    let templates_path = get_templates_path();

    let mut templates = Vec::new();

    if templates_path.exists() {
        for entry in fs::read_dir(&templates_path).map_err(|e| AppError::Internal(e.to_string()))? {
            let entry = entry.map_err(|e| AppError::Internal(e.to_string()))?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                match parse_template_file(&path) {
                    Ok((id, template)) => {
                        if template.category.to_lowercase() == category.to_lowercase() {
                            templates.push(TemplateSummary {
                                id,
                                name: template.name.clone(),
                                description: template.description.clone(),
                                category: template.category.clone(),
                                stage_count: template.stages.len(),
                                agent_count: template.agents.len(),
                            });
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse template {:?}: {}", path, e);
                    }
                }
            }
        }
    }

    // 当文件系统无结果时从内置模板中过滤
    if templates.is_empty() {
        for (id, template) in get_builtin_templates() {
            if template.category.to_lowercase() == category.to_lowercase() {
                templates.push(TemplateSummary {
                    id,
                    name: template.name,
                    description: template.description,
                    category: template.category,
                    stage_count: template.stages.len(),
                    agent_count: template.agents.len(),
                });
            }
        }
    }

    // 按名称排序
    templates.sort_by(|a, b| a.name.cmp(&b.name));

    let total = templates.len();

    Ok(Json(ListResponse { items: templates, total }))
}

// ============ 错误类型 ============

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

/// 应用错误类型
#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    BadRequest(String),
    Internal(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AppError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            AppError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::Internal(msg) => {
                tracing::error!("内部错误: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "内部服务器错误".to_string())
            }
        };

        let body = serde_json::json!({
            "error": message
        });

        (status, Json(body)).into_response()
    }
}

impl From<crate::services::workflow_service::WorkflowServiceError> for AppError {
    fn from(err: crate::services::workflow_service::WorkflowServiceError) -> Self {
        match err {
            crate::services::workflow_service::WorkflowServiceError::NotFound(id) => {
                AppError::NotFound(id)
            }
            crate::services::workflow_service::WorkflowServiceError::AlreadyExists(id) => {
                AppError::BadRequest(format!("工作流 {} 已存在", id))
            }
            crate::services::workflow_service::WorkflowServiceError::Internal(msg) => {
                AppError::Internal(msg)
            }
        }
    }
}