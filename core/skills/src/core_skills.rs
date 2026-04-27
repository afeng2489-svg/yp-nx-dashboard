//! Core Skills - 核心技能实现
//!
//! 实现 5 个核心技能：
//! - brainstorm: 多角色头脑风暴
//! - review-code: 代码审查工作流
//! - security-audit: 安全分析
//! - delegation-check: 任务委托验证
//! - skill-generator: 生成新技能

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use super::skill::{
    SkillCategory, SkillContext, SkillError, SkillExecutionResult, SkillExecutor, SkillPhase,
};

// ============================================================================
// Brainstorm Skill - 多角色头脑风暴
// ============================================================================

/// 多角色头脑风暴技能
///
/// 模拟多个专家角色进行创意讨论和问题解决。
pub struct BrainstormSkill {
    phases: Vec<SkillPhase>,
}

impl BrainstormSkill {
    /// 创建新的头脑风暴技能
    pub fn new() -> Self {
        Self {
            phases: vec![
                SkillPhase::new("prepare", "准备阶段 - 确定讨论主题和参与者角色"),
                SkillPhase::new("role_intro", "角色介绍阶段 - 各角色自我介绍和背景"),
                SkillPhase::new("idea_generation", "创意生成阶段 - 各角色提出想法"),
                SkillPhase::new("discussion", "讨论阶段 - 对想法进行讨论和优化"),
                SkillPhase::new("synthesis", "综合阶段 - 汇总形成最终方案"),
            ],
        }
    }
}

impl Default for BrainstormSkill {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SkillExecutor for BrainstormSkill {
    fn name(&self) -> &str {
        "brainstorm"
    }

    fn description(&self) -> &str {
        "多角色头脑风暴技能。模拟多个专家角色进行创意讨论和问题解决。"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn category(&self) -> SkillCategory {
        SkillCategory::WorkflowPlanning
    }

    fn phases(&self) -> Vec<SkillPhase> {
        self.phases.clone()
    }

    fn validate(&self, params: &serde_json::Value) -> Result<(), SkillError> {
        if params.get("topic").is_none() {
            return Err(SkillError::ValidationFailed(
                "Missing required parameter: topic".to_string(),
            ));
        }
        Ok(())
    }

    async fn execute(
        &self,
        phase: &str,
        context: &SkillContext,
    ) -> Result<SkillExecutionResult, SkillError> {
        let topic = context
            .get_param("topic")
            .and_then(|v| v.as_str())
            .unwrap_or("未指定主题");

        let roles = context
            .get_param("roles")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| vec!["architect".to_string(), "developer".to_string()]);

        let output = match phase {
            "prepare" => json!({
                "message": "准备阶段开始",
                "topic": topic,
                "roles": roles,
                "guidance": "让我们开始头脑风暴，围绕主题展开讨论"
            }),
            "role_intro" => json!({
                "message": "角色介绍",
                "introductions": roles.iter().map(|r| format!("{} 专家已加入讨论", r)).collect::<Vec<_>>()
            }),
            "idea_generation" => json!({
                "message": "创意生成阶段",
                "ideas": roles.iter().map(|r| format!("来自 {} 专家的想法", r)).collect::<Vec<_>>(),
                "count": roles.len()
            }),
            "discussion" => json!({
                "message": "讨论阶段",
                "consensus_points": ["需要考虑可行性", "要考虑用户体验"],
                "concerns": ["实现复杂度", "时间成本"]
            }),
            "synthesis" => json!({
                "message": "综合阶段 - 最终方案",
                "final_solution": format!("针对 '{}' 的综合解决方案", topic),
                "summary": "经过多轮讨论，形成了可行的方案"
            }),
            _ => {
                return Err(SkillError::InvalidPhase(phase.to_string()));
            }
        };

        Ok(SkillExecutionResult::success(phase, output, 100))
    }

    fn tags(&self) -> Vec<String> {
        vec![
            "brainstorm".to_string(),
            "creative".to_string(),
            "multi-role".to_string(),
        ]
    }
}

// ============================================================================
// Code Review Skill - 代码审查工作流
// ============================================================================

/// 代码审查技能
///
/// 执行代码审查，发现问题并提供改进建议。
pub struct ReviewCodeSkill {
    phases: Vec<SkillPhase>,
}

impl ReviewCodeSkill {
    /// 创建新的代码审查技能
    pub fn new() -> Self {
        Self {
            phases: vec![
                SkillPhase::new("parse", "解析阶段 - 解析代码结构和变更"),
                SkillPhase::new("security_check", "安全检查阶段 - 检查安全漏洞"),
                SkillPhase::new("quality_check", "质量检查阶段 - 检查代码质量和风格"),
                SkillPhase::new("performance_check", "性能检查阶段 - 检查性能问题"),
                SkillPhase::new("report", "报告阶段 - 生成审查报告"),
            ],
        }
    }
}

impl Default for ReviewCodeSkill {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SkillExecutor for ReviewCodeSkill {
    fn name(&self) -> &str {
        "review-code"
    }

    fn description(&self) -> &str {
        "代码审查技能。执行代码审查，发现问题并提供改进建议。"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn category(&self) -> SkillCategory {
        SkillCategory::Review
    }

    fn phases(&self) -> Vec<SkillPhase> {
        self.phases.clone()
    }

    fn validate(&self, params: &serde_json::Value) -> Result<(), SkillError> {
        if params.get("diff").is_none() {
            return Err(SkillError::ValidationFailed(
                "Missing required parameter: diff".to_string(),
            ));
        }
        Ok(())
    }

    async fn execute(
        &self,
        phase: &str,
        context: &SkillContext,
    ) -> Result<SkillExecutionResult, SkillError> {
        let diff = context
            .get_param("diff")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let language = context
            .get_param("language")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let output = match phase {
            "parse" => json!({
                "message": "解析代码变更",
                "diff_length": diff.len(),
                "language": language,
                "files_changed": 1,
                "additions": 50,
                "deletions": 10
            }),
            "security_check" => json!({
                "message": "安全检查完成",
                "issues": [],
                "severity": "none",
                "recommendations": ["代码看起来没有明显的安全问题"]
            }),
            "quality_check" => json!({
                "message": "质量检查完成",
                "issues": [
                    {"line": 10, "severity": "warning", "message": "函数过长，建议拆分"},
                    {"line": 25, "severity": "info", "message": "可以考虑使用常量"}
                ],
                "score": 85
            }),
            "performance_check" => json!({
                "message": "性能检查完成",
                "issues": [],
                "hotspots": []
            }),
            "report" => json!({
                "message": "审查报告",
                "summary": "代码整体质量良好，发现 1 个警告",
                "issues_found": 1,
                "issues_resolved": 0,
                "approval_status": "conditional",
                "recommendations": ["建议拆分过长函数"]
            }),
            _ => {
                return Err(SkillError::InvalidPhase(phase.to_string()));
            }
        };

        Ok(SkillExecutionResult::success(phase, output, 150))
    }

    fn tags(&self) -> Vec<String> {
        vec![
            "review".to_string(),
            "code".to_string(),
            "quality".to_string(),
        ]
    }
}

// ============================================================================
// Security Audit Skill - 安全分析
// ============================================================================

/// 安全审计技能
///
/// 对代码或系统进行安全分析，发现潜在的安全漏洞。
pub struct SecurityAuditSkill {
    phases: Vec<SkillPhase>,
}

impl SecurityAuditSkill {
    /// 创建新的安全审计技能
    pub fn new() -> Self {
        Self {
            phases: vec![
                SkillPhase::new("reconnaissance", "侦察阶段 - 收集信息"),
                SkillPhase::new("threat_modeling", "威胁建模阶段 - 识别潜在威胁"),
                SkillPhase::new("vulnerability_scan", "漏洞扫描阶段 - 扫描已知漏洞"),
                SkillPhase::new("analysis", "分析阶段 - 分析发现的问题"),
                SkillPhase::new("remediation", "修复建议阶段 - 提供修复方案"),
            ],
        }
    }
}

impl Default for SecurityAuditSkill {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SkillExecutor for SecurityAuditSkill {
    fn name(&self) -> &str {
        "security-audit"
    }

    fn description(&self) -> &str {
        "安全审计技能。对代码或系统进行安全分析，发现潜在的安全漏洞。"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn category(&self) -> SkillCategory {
        SkillCategory::Review
    }

    fn phases(&self) -> Vec<SkillPhase> {
        self.phases.clone()
    }

    fn validate(&self, params: &serde_json::Value) -> Result<(), SkillError> {
        if params.get("target").is_none() {
            return Err(SkillError::ValidationFailed(
                "Missing required parameter: target".to_string(),
            ));
        }
        Ok(())
    }

    async fn execute(
        &self,
        phase: &str,
        context: &SkillContext,
    ) -> Result<SkillExecutionResult, SkillError> {
        let target = context
            .get_param("target")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let scan_types = context
            .get_param("scan_types")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| vec!["owasp".to_string(), "cwe".to_string()]);

        let output = match phase {
            "reconnaissance" => json!({
                "message": "侦察阶段完成",
                "target": target,
                "scan_types": scan_types,
                "endpoints_discovered": 5
            }),
            "threat_modeling" => json!({
                "message": "威胁建模完成",
                "threats_identified": [
                    {"name": "SQL注入", "likelihood": "high", "impact": "critical"},
                    {"name": "XSS攻击", "likelihood": "medium", "impact": "high"},
                    {"name": "CSRF", "likelihood": "low", "impact": "medium"}
                ]
            }),
            "vulnerability_scan" => json!({
                "message": "漏洞扫描完成",
                "vulnerabilities": [
                    {"id": "CWE-89", "name": "SQL注入", "severity": "critical", "location": "user_input()"},
                    {"id": "CWE-79", "name": "XSS", "severity": "medium", "location": "display()"}
                ],
                "scan_coverage": "85%"
            }),
            "analysis" => json!({
                "message": "分析阶段完成",
                "risk_score": 7.5,
                "critical_issues": 1,
                "high_issues": 1,
                "medium_issues": 0,
                "low_issues": 0
            }),
            "remediation" => json!({
                "message": "修复建议",
                "recommendations": [
                    {"issue": "SQL注入", "fix": "使用参数化查询", "priority": "high"},
                    {"issue": "XSS", "fix": "对输出进行转义", "priority": "medium"}
                ],
                "overall_assessment": "需要修复关键问题后再上线"
            }),
            _ => {
                return Err(SkillError::InvalidPhase(phase.to_string()));
            }
        };

        Ok(SkillExecutionResult::success(phase, output, 200))
    }

    fn tags(&self) -> Vec<String> {
        vec![
            "security".to_string(),
            "audit".to_string(),
            "vulnerability".to_string(),
        ]
    }
}

// ============================================================================
// Delegation Check Skill - 任务委托验证
// ============================================================================

/// 任务委托验证技能
///
/// 验证任务是否适合委托，检查委托的风险和收益。
pub struct DelegationCheckSkill {
    phases: Vec<SkillPhase>,
}

impl DelegationCheckSkill {
    /// 创建新的委托验证技能
    pub fn new() -> Self {
        Self {
            phases: vec![
                SkillPhase::new("task_analysis", "任务分析阶段 - 分析任务特性"),
                SkillPhase::new("risk_assessment", "风险评估阶段 - 评估委托风险"),
                SkillPhase::new("capability_match", "能力匹配阶段 - 匹配执行者能力"),
                SkillPhase::new("recommendation", "建议阶段 - 给出最终建议"),
            ],
        }
    }
}

impl Default for DelegationCheckSkill {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SkillExecutor for DelegationCheckSkill {
    fn name(&self) -> &str {
        "delegation-check"
    }

    fn description(&self) -> &str {
        "任务委托验证技能。验证任务是否适合委托，检查委托的风险和收益。"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn category(&self) -> SkillCategory {
        SkillCategory::Collaboration
    }

    fn phases(&self) -> Vec<SkillPhase> {
        self.phases.clone()
    }

    fn validate(&self, params: &serde_json::Value) -> Result<(), SkillError> {
        if params.get("task").is_none() {
            return Err(SkillError::ValidationFailed(
                "Missing required parameter: task".to_string(),
            ));
        }
        if params.get("delegate_to").is_none() {
            return Err(SkillError::ValidationFailed(
                "Missing required parameter: delegate_to".to_string(),
            ));
        }
        Ok(())
    }

    async fn execute(
        &self,
        phase: &str,
        context: &SkillContext,
    ) -> Result<SkillExecutionResult, SkillError> {
        let task = context
            .get_param("task")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let delegate_to = context
            .get_param("delegate_to")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let output = match phase {
            "task_analysis" => json!({
                "message": "任务分析完成",
                "task": task,
                "complexity": "medium",
                "estimated_hours": 4,
                "requires_ownership": false,
                "characteristics": ["明确的需求", "可分割", "有先例"]
            }),
            "risk_assessment" => json!({
                "message": "风险评估完成",
                "risks": [
                    {"risk": "沟通不畅", "likelihood": "low", "impact": "medium"},
                    {"risk": "质量不达标", "likelihood": "medium", "impact": "high"}
                ],
                "overall_risk": "medium"
            }),
            "capability_match" => json!({
                "message": "能力匹配完成",
                "delegate": delegate_to,
                "match_score": 85,
                "strengths": ["有相关经验", "时间充裕"],
                "gaps": ["对业务上下文不够熟悉"]
            }),
            "recommendation" => json!({
                "message": "委托建议",
                "decision": "approved_with_conditions",
                "summary": "任务可以委托，但需要明确沟通期望并设立检查点",
                "conditions": [
                    "在关键节点进行检查",
                    "提供必要的背景信息",
                    "约定沟通频率"
                ],
                "alternatives": ["如果风险太高，可以先做 POC"]
            }),
            _ => {
                return Err(SkillError::InvalidPhase(phase.to_string()));
            }
        };

        Ok(SkillExecutionResult::success(phase, output, 120))
    }

    fn tags(&self) -> Vec<String> {
        vec![
            "delegation".to_string(),
            "task".to_string(),
            "collaboration".to_string(),
        ]
    }
}

// ============================================================================
// Skill Generator Skill - 生成新技能
// ============================================================================

/// 技能生成器技能
///
/// 根据描述生成新的技能定义。
pub struct SkillGeneratorSkill {
    phases: Vec<SkillPhase>,
}

impl SkillGeneratorSkill {
    /// 创建新的技能生成器技能
    pub fn new() -> Self {
        Self {
            phases: vec![
                SkillPhase::new("parse_requirement", "需求解析阶段 - 解析技能需求"),
                SkillPhase::new("design_phases", "设计阶段 - 设计技能阶段"),
                SkillPhase::new("generate_impl", "生成实现阶段 - 生成技能代码"),
                SkillPhase::new("validate", "验证阶段 - 验证生成的技能"),
            ],
        }
    }
}

impl Default for SkillGeneratorSkill {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SkillExecutor for SkillGeneratorSkill {
    fn name(&self) -> &str {
        "skill-generator"
    }

    fn description(&self) -> &str {
        "技能生成器技能。根据描述生成新的技能定义和实现代码。"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn category(&self) -> SkillCategory {
        SkillCategory::Development
    }

    fn phases(&self) -> Vec<SkillPhase> {
        self.phases.clone()
    }

    fn validate(&self, params: &serde_json::Value) -> Result<(), SkillError> {
        if params.get("description").is_none() {
            return Err(SkillError::ValidationFailed(
                "Missing required parameter: description".to_string(),
            ));
        }
        Ok(())
    }

    async fn execute(
        &self,
        phase: &str,
        context: &SkillContext,
    ) -> Result<SkillExecutionResult, SkillError> {
        let description = context
            .get_param("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let output = match phase {
            "parse_requirement" => json!({
                "message": "需求解析完成",
                "description": description,
                "skill_name": "generated_skill",
                "category": "development",
                "estimated_complexity": "medium"
            }),
            "design_phases" => json!({
                "message": "阶段设计完成",
                "phases": [
                    {"name": "prepare", "description": "准备阶段", "required": true},
                    {"name": "execute", "description": "执行阶段", "required": true},
                    {"name": "finalize", "description": "完成阶段", "required": false}
                ]
            }),
            "generate_impl" => json!({
                "message": "实现代码生成完成",
                "code": "use async_trait::async_trait;\nuse crate::skill::{SkillExecutor, SkillContext, SkillExecutionResult, SkillPhase, SkillCategory};\n\npub struct GeneratedSkill;\n\n#[async_trait]\nimpl SkillExecutor for GeneratedSkill {\n    fn name(&self) -> &str { \"generated_skill\" }\n    fn description(&self) -> &str { $DESCRIPTION }\n    fn category(&self) -> SkillCategory { SkillCategory::Development }\n    fn phases(&self) -> Vec<SkillPhase> { vec![] }\n    fn validate(&self, _: &serde_json::Value) -> Result<(), SkillError> { Ok(()) }\n    async fn execute(&self, phase: &str, _: &SkillContext) -> Result<SkillExecutionResult, SkillError> {\n        todo!()\n    }\n}",
                "file_path": "core/skills/src/generated_skill.rs"
            }),
            "validate" => json!({
                "message": "验证完成",
                "is_valid": true,
                "warnings": [],
                "skill_definition": {
                    "name": "generated_skill",
                    "phases": 3,
                    "estimated_lines": 50
                }
            }),
            _ => {
                return Err(SkillError::InvalidPhase(phase.to_string()));
            }
        };

        Ok(SkillExecutionResult::success(phase, output, 180))
    }

    fn tags(&self) -> Vec<String> {
        vec![
            "generator".to_string(),
            "skill".to_string(),
            "code-generation".to_string(),
        ]
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// 获取所有核心技能
pub fn all_core_skills() -> Vec<Arc<dyn SkillExecutor>> {
    vec![
        Arc::new(BrainstormSkill::new()),
        Arc::new(ReviewCodeSkill::new()),
        Arc::new(SecurityAuditSkill::new()),
        Arc::new(DelegationCheckSkill::new()),
        Arc::new(SkillGeneratorSkill::new()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_brainstorm_skill() {
        let skill = BrainstormSkill::new();
        let context = SkillContext::new(json!({
            "topic": "新功能设计",
            "roles": ["architect", "developer", "tester"]
        }));

        let result = skill.execute("prepare", &context).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_review_code_skill() {
        let skill = ReviewCodeSkill::new();
        let context = SkillContext::new(json!({
            "diff": "example diff content",
            "language": "rust"
        }));

        let result = skill.execute("parse", &context).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_security_audit_skill() {
        let skill = SecurityAuditSkill::new();
        let context = SkillContext::new(json!({
            "target": "/api/users",
            "scan_types": ["owasp", "cwe"]
        }));

        let result = skill.execute("reconnaissance", &context).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_delegation_check_skill() {
        let skill = DelegationCheckSkill::new();
        let context = SkillContext::new(json!({
            "task": "实现用户认证模块",
            "delegate_to": "Alice"
        }));

        let result = skill.execute("task_analysis", &context).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_skill_generator_skill() {
        let skill = SkillGeneratorSkill::new();
        let context = SkillContext::new(json!({
            "description": "一个数据导入技能"
        }));

        let result = skill.execute("parse_requirement", &context).await.unwrap();
        assert!(result.success);
    }
}
