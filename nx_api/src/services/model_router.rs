//! 模型自动路由器
//!
//! 按规则匹配任务上下文，返回最合适的模型名称。
//! 规则按 priority 降序匹配，第一个命中的规则生效。

use serde::{Deserialize, Serialize};

/// 路由条件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RoutingCondition {
    /// prompt 包含指定关键词之一
    KeywordMatch { keywords: Vec<String> },
    /// prompt 长度超过 N 字符
    PromptLength { min_chars: usize },
    /// 任务类型标签（由调用方传入）
    TaskType { task_type: String },
    /// 文件扩展名（从 prompt 中检测）
    FileExtension { extensions: Vec<String> },
}

/// 单条路由规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRule {
    pub id: String,
    pub name: String,
    pub condition: RoutingCondition,
    pub model: String,
    /// 数字越大优先级越高
    pub priority: u8,
    pub enabled: bool,
}

/// 路由任务上下文
pub struct TaskContext<'a> {
    pub prompt: &'a str,
    pub task_type: Option<&'a str>,
}

/// 模型路由器
#[derive(Debug, Clone, Default)]
pub struct ModelRouter {
    rules: Vec<RoutingRule>,
}

impl ModelRouter {
    pub fn new(rules: Vec<RoutingRule>) -> Self {
        Self { rules }
    }

    /// 按优先级匹配规则，返回模型名；无匹配返回 None（使用全局默认）
    pub fn route(&self, ctx: &TaskContext<'_>) -> Option<String> {
        let mut sorted: Vec<&RoutingRule> = self.rules.iter().filter(|r| r.enabled).collect();
        sorted.sort_by(|a, b| b.priority.cmp(&a.priority));

        for rule in sorted {
            if self.matches(&rule.condition, ctx) {
                tracing::debug!("[ModelRouter] 命中规则 '{}' → {}", rule.name, rule.model);
                return Some(rule.model.clone());
            }
        }
        None
    }

    fn matches(&self, condition: &RoutingCondition, ctx: &TaskContext<'_>) -> bool {
        match condition {
            RoutingCondition::KeywordMatch { keywords } => {
                let lower = ctx.prompt.to_lowercase();
                keywords.iter().any(|k| lower.contains(&k.to_lowercase()))
            }
            RoutingCondition::PromptLength { min_chars } => ctx.prompt.len() >= *min_chars,
            RoutingCondition::TaskType { task_type } => {
                ctx.task_type.map_or(false, |t| t == task_type)
            }
            RoutingCondition::FileExtension { extensions } => extensions.iter().any(|ext| {
                let pattern = format!(".{}", ext.trim_start_matches('.'));
                ctx.prompt.contains(&pattern)
            }),
        }
    }

    pub fn rules(&self) -> &[RoutingRule] {
        &self.rules
    }

    pub fn set_rules(&mut self, rules: Vec<RoutingRule>) {
        self.rules = rules;
    }
}

/// 内置默认规则
pub fn default_rules() -> Vec<RoutingRule> {
    vec![
        RoutingRule {
            id: "rule_sensitive".to_string(),
            name: "敏感任务 → 本地模型".to_string(),
            condition: RoutingCondition::KeywordMatch {
                keywords: vec![
                    "密码".to_string(),
                    "密钥".to_string(),
                    "内网".to_string(),
                    "secret".to_string(),
                    "private_key".to_string(),
                ],
            },
            model: "ollama/llama3".to_string(),
            priority: 100,
            enabled: true,
        },
        RoutingRule {
            id: "rule_rust".to_string(),
            name: "Rust 代码 → Claude Sonnet".to_string(),
            condition: RoutingCondition::FileExtension {
                extensions: vec!["rs".to_string(), "toml".to_string()],
            },
            model: "claude-sonnet-4-6".to_string(),
            priority: 80,
            enabled: true,
        },
        RoutingRule {
            id: "rule_frontend".to_string(),
            name: "前端代码 → MiMo".to_string(),
            condition: RoutingCondition::FileExtension {
                extensions: vec![
                    "tsx".to_string(),
                    "ts".to_string(),
                    "jsx".to_string(),
                    "css".to_string(),
                ],
            },
            model: "mimo-v2.5-pro".to_string(),
            priority: 70,
            enabled: true,
        },
        RoutingRule {
            id: "rule_summary".to_string(),
            name: "总结/报告 → Qwen".to_string(),
            condition: RoutingCondition::KeywordMatch {
                keywords: vec![
                    "总结".to_string(),
                    "报告".to_string(),
                    "summary".to_string(),
                    "report".to_string(),
                    "文档".to_string(),
                ],
            },
            model: "qwen-2.5-72b".to_string(),
            priority: 50,
            enabled: true,
        },
    ]
}
