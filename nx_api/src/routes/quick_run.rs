use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::routes::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/api/v1/quick-run", post(quick_run))
}

#[derive(Deserialize)]
pub struct QuickRunReq {
    pub prompt: String,
}

#[derive(Serialize)]
pub struct QuickRunResp {
    pub execution_id: String,
    pub workflow_name: String,
    pub variables: serde_json::Value,
}

/// 根据用户输入匹配最合适的工作流并启动
pub async fn quick_run(
    State(state): State<Arc<AppState>>,
    Json(req): Json<QuickRunReq>,
) -> Json<serde_json::Value> {
    let prompt = req.prompt.trim().to_string();
    if prompt.is_empty() {
        return Json(serde_json::json!({ "ok": false, "error": "prompt 不能为空" }));
    }

    // 1. 列出所有工作流
    let workflows = match state.workflow_service.list_workflows() {
        Ok(w) => w,
        Err(e) => return Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
    };

    if workflows.is_empty() {
        return Json(serde_json::json!({ "ok": false, "error": "没有可用的工作流，请先在工作流页面创建" }));
    }

    // 2. 关键词匹配选工作流
    let workflow = match_workflow(&workflows, &prompt);

    // 3. 从 prompt 推断 variables
    let variables = infer_variables(&workflow.definition, &prompt);

    // 4. 构建 YAML
    let mut workflow_def = serde_json::json!({
        "name": workflow.name,
        "version": workflow.version,
    });
    if let Some(desc) = &workflow.description {
        workflow_def["description"] = serde_json::json!(desc);
    }
    if let Some(obj) = workflow.definition.as_object() {
        for (k, v) in obj {
            if !["name", "version", "description"].contains(&k.as_str()) {
                workflow_def[k] = v.clone();
            }
        }
    }
    let workflow_yaml = match serde_yaml::to_string(&workflow_def) {
        Ok(y) => y,
        Err(e) => return Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
    };

    let current_workspace = state.current_workspace_path.read().clone();

    match state
        .execution_service
        .execute_workflow(workflow.id.clone(), &workflow_yaml, variables.clone(), None, current_workspace)
        .await
    {
        Ok(execution_id) => Json(serde_json::json!({
            "ok": true,
            "data": {
                "execution_id": execution_id,
                "workflow_name": workflow.name,
                "variables": variables,
            }
        })),
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
    }
}

/// 关键词匹配：返回最合适的工作流
fn match_workflow(
    workflows: &[crate::services::workflow_service::Workflow],
    prompt: &str,
) -> crate::services::workflow_service::Workflow {
    let lower = prompt.to_lowercase();

    // 评分规则：名称/描述中关键词命中数
    let scored: Vec<(usize, &crate::services::workflow_service::Workflow)> = workflows
        .iter()
        .map(|w| {
            let text = format!(
                "{} {}",
                w.name.to_lowercase(),
                w.description.as_deref().unwrap_or("").to_lowercase()
            );
            let score = KEYWORDS
                .iter()
                .filter(|(kws, _)| kws.iter().any(|k| lower.contains(k)))
                .filter(|(_, wf_kws)| wf_kws.iter().any(|k| text.contains(k)))
                .count()
                + text.split_whitespace().filter(|w| lower.contains(*w)).count();
            (score, w)
        })
        .collect();

    scored
        .into_iter()
        .max_by_key(|(s, _)| *s)
        .map(|(_, w)| w.clone())
        .unwrap_or_else(|| workflows[0].clone())
}

/// prompt 关键词 → 工作流关键词映射
static KEYWORDS: &[(&[&str], &[&str])] = &[
    (&["bug", "修复", "fix", "错误", "报错", "crash"], &["fix", "bug", "investigate", "quick"]),
    (&["开发", "功能", "feature", "实现", "implement", "新增"], &["dev", "workflow", "feature"]),
    (&["测试", "test", "tdd", "单元", "覆盖率"], &["tdd", "test", "fix"]),
    (&["审查", "review", "代码质量", "code review"], &["review"]),
    (&["分析", "brainstorm", "头脑风暴", "方案", "设计"], &["brainstorm", "investigate"]),
    (&["调查", "根因", "investigate", "排查"], &["investigate"]),
];

/// 从工作流 definition 的 inputs 字段推断 variables
fn infer_variables(definition: &serde_json::Value, prompt: &str) -> serde_json::Value {
    let inputs = definition
        .pointer("/triggers/0/inputs")
        .or_else(|| definition.get("inputs"));

    let Some(inputs) = inputs.and_then(|v| v.as_object()) else {
        return serde_json::json!({});
    };

    let mut vars = serde_json::Map::new();
    for (key, _) in inputs {
        // 对所有 required 字段，用 prompt 本身填充
        vars.insert(key.clone(), serde_json::json!(prompt));
    }
    serde_json::Value::Object(vars)
}
