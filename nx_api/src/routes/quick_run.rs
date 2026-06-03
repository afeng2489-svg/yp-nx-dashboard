use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::routes::AppState;
use crate::services::execution_service::ExecutionLaunchContext;

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/api/v1/quick-run", post(quick_run))
}

#[derive(Deserialize)]
pub struct QuickRunReq {
    pub prompt: String,
    #[serde(default)]
    pub team_id: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub workflow_name: Option<String>,
    /// AF-UX-08
    #[serde(default)]
    pub approval_policy: Option<String>,
    /// AF-MM-04
    #[serde(default)]
    pub text_lane_cost_mode: Option<String>,
    /// AF-UX-09
    #[serde(default)]
    pub retry_from_stage: Option<String>,
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

    let workflows = match state.workflow_service.list_workflows() {
        Ok(w) => w,
        Err(e) => return Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
    };

    if workflows.is_empty() {
        return Json(
            serde_json::json!({ "ok": false, "error": "没有可用的工作流，请先在工作流页面创建" }),
        );
    }

    let project_workflow_id = req
        .project_id
        .as_deref()
        .and_then(|pid| state.project_service.get_project(pid).ok().flatten())
        .and_then(|p| p.workflow_id);

    let workflow = select_workflow(
        &workflows,
        &prompt,
        req.workflow_name.as_deref(),
        project_workflow_id.as_deref(),
    );

    let mut variables = infer_variables(&workflow.definition, &prompt);

    if let Some(ref team_id) = req.team_id {
        apply_role_overlay(&state, team_id, &prompt, &mut variables);
    }

    apply_project_context(&state, req.project_id.as_deref(), &mut variables);
    apply_stack_profile(&state, req.project_id.as_deref(), &mut variables);
    apply_knowledge_injection(&state, req.project_id.as_deref(), &prompt, &mut variables).await;

    if let Some(obj) = variables.as_object_mut() {
        if let Some(p) = req.approval_policy {
            obj.insert("approval_policy".into(), serde_json::json!(p));
        }
        if let Some(m) = req.text_lane_cost_mode {
            obj.insert("text_lane_cost_mode".into(), serde_json::json!(m));
        }
        if let Some(s) = req.retry_from_stage {
            obj.insert("retry_from_stage".into(), serde_json::json!(s));
        }
    }

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
    let launch = Some(ExecutionLaunchContext {
        team_id: req.team_id.clone(),
        project_id: req.project_id.clone(),
        trigger_source: Some("factory".to_string()),
    });

    match state
        .execution_service
        .execute_workflow(
            workflow.id.clone(),
            &workflow_yaml,
            variables.clone(),
            None,
            current_workspace,
            launch,
        )
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

fn select_workflow(
    workflows: &[crate::services::workflow_service::Workflow],
    prompt: &str,
    workflow_name: Option<&str>,
    project_workflow_id: Option<&str>,
) -> crate::services::workflow_service::Workflow {
    if let Some(name) = workflow_name {
        if let Some(w) = workflows
            .iter()
            .find(|w| w.name.eq_ignore_ascii_case(name))
        {
            return w.clone();
        }
    }

    if let Some(wid) = project_workflow_id {
        if let Some(w) = workflows.iter().find(|w| w.id == wid) {
            return w.clone();
        }
    }

    if let Some(w) = workflows
        .iter()
        .find(|w| w.name.eq_ignore_ascii_case("solo-dev"))
    {
        return w.clone();
    }

    match_workflow(workflows, prompt)
}

fn apply_project_context(
    state: &AppState,
    project_id: Option<&str>,
    variables: &mut serde_json::Value,
) {
    let Some(pid) = project_id else {
        return;
    };
    let Some(obj) = variables.as_object_mut() else {
        return;
    };

    // Legacy execution project
    if let Ok(Some(project)) = state.project_service.get_project(pid) {
        obj.insert("project_id".to_string(), serde_json::json!(project.id));
        obj.insert("project_name".to_string(), serde_json::json!(project.name));
        for (key, value) in &project.variables {
            obj.entry(key.clone())
                .or_insert_with(|| serde_json::json!(value));
        }
        return;
    }

    // Unified model: workspace id passed as project_id
    if let Ok(Some(ws)) = state.workspace_service.get_workspace(pid) {
        obj.insert("workspace_id".to_string(), serde_json::json!(ws.id));
        obj.insert("project_id".to_string(), serde_json::json!(ws.id));
        obj.insert("project_name".to_string(), serde_json::json!(ws.name));
        if let Some(path) = &ws.root_path {
            obj.insert("project_path".to_string(), serde_json::json!(path));
            obj.insert("workspace_path".to_string(), serde_json::json!(path));
        }
        if let Some(team_id) = ws.settings.get("team_id").and_then(|v| v.as_str()) {
            if !team_id.is_empty() {
                obj.entry("team_id".to_string())
                    .or_insert_with(|| serde_json::json!(team_id));
            }
        }
        if let Some(settings) = ws.settings.as_object() {
            for (key, value) in settings {
                if key != "team_id" {
                    obj.entry(key.clone()).or_insert_with(|| value.clone());
                }
            }
        }
    }
}

/// Phase B — detect stack from workspace root and inject lang / gate hints into variables
fn apply_stack_profile(
    state: &AppState,
    project_id: Option<&str>,
    variables: &mut serde_json::Value,
) {
    let Some(pid) = project_id else { return };
    let Some(obj) = variables.as_object_mut() else { return };

    let root_path = state
        .workspace_service
        .get_workspace(pid)
        .ok()
        .flatten()
        .and_then(|ws| ws.root_path)
        .filter(|p| !p.is_empty());

    let Some(root) = root_path else { return };
    let root = std::path::Path::new(&root);

    let profile = detect_stack_at_root(root);
    if profile.lang != "auto" {
        obj.entry("lang".to_string())
            .or_insert_with(|| serde_json::json!(profile.lang));
    }
    if let Some(test_cmd) = profile.test_cmd {
        obj.insert("stack_test_cmd".to_string(), serde_json::json!(test_cmd));
    }
    if let Some(build_cmd) = profile.build_cmd {
        obj.insert("stack_build_cmd".to_string(), serde_json::json!(build_cmd));
    }
    obj.insert("stack_profile".to_string(), serde_json::json!(profile.lang));
}

struct StackProfileHint {
    lang: &'static str,
    test_cmd: Option<&'static str>,
    build_cmd: Option<&'static str>,
}

fn detect_stack_at_root(root: &std::path::Path) -> StackProfileHint {
    if root.join("Cargo.toml").exists() {
        return StackProfileHint {
            lang: "rust",
            test_cmd: Some("cargo test"),
            build_cmd: Some("cargo build"),
        };
    }
    if root.join("go.mod").exists() {
        return StackProfileHint {
            lang: "go",
            test_cmd: Some("go test ./..."),
            build_cmd: Some("go build ./..."),
        };
    }
    if root.join("pyproject.toml").exists() || root.join("requirements.txt").exists() {
        return StackProfileHint {
            lang: "python",
            test_cmd: Some("pytest"),
            build_cmd: None,
        };
    }
    if root.join("package.json").exists() {
        let lang = if root.join("tsconfig.json").exists() {
            "typescript"
        } else {
            "typescript"
        };
        return StackProfileHint {
            lang,
            test_cmd: Some("npm test"),
            build_cmd: Some("npm run build"),
        };
    }
    StackProfileHint {
        lang: "auto",
        test_cmd: None,
        build_cmd: None,
    }
}

/// EF9 — 工厂台 quick-run 注入知识库 ID 与检索上下文
async fn apply_knowledge_injection(
    state: &AppState,
    project_id: Option<&str>,
    prompt: &str,
    variables: &mut serde_json::Value,
) {
    let kb_id = resolve_kb_id(state, project_id);
    let Some(kb_id) = kb_id else {
        return;
    };

    let Some(obj) = variables.as_object_mut() else {
        return;
    };

    obj.insert("project_kb_id".to_string(), serde_json::json!(kb_id));

    match state
        .knowledge_service
        .retrieve_texts(&kb_id, prompt, 5, 0.5)
        .await
    {
        Ok(texts) if !texts.is_empty() => {
            let context = texts.join("\n---\n");
            obj.insert("knowledge_context".to_string(), serde_json::json!(context));

            let mut enriched = false;
            for key in ["task", "prompt", "user_task", "goal", "input"] {
                if let Some(v) = obj.get(key).and_then(|v| v.as_str()) {
                    obj.insert(
                        key.to_string(),
                        serde_json::json!(format!(
                            "{v}\n\n[Knowledge Base Context]\n{context}"
                        )),
                    );
                    enriched = true;
                    break;
                }
            }
            if !enriched {
                obj.insert("task".to_string(), serde_json::json!(format!(
                    "{prompt}\n\n[Knowledge Base Context]\n{context}"
                )));
            }
            tracing::info!(
                "[quick-run] EF9 知识库注入 kb={kb_id} chunks={}",
                texts.len()
            );
        }
        Ok(_) => tracing::debug!("[quick-run] 知识库 {kb_id} 无匹配片段"),
        Err(e) => tracing::warn!("[quick-run] 知识库检索失败: {e}"),
    }
}

fn resolve_kb_id(state: &AppState, project_id: Option<&str>) -> Option<String> {
    if let Some(pid) = project_id {
        if let Ok(Some(project)) = state.project_service.get_project(pid) {
            for key in ["project_kb_id", "knowledge_base_id", "kb_id"] {
                if let Some(v) = project.variables.get(key) {
                    if !v.trim().is_empty() {
                        return Some(v.clone());
                    }
                }
            }
        }
        if let Ok(Some(ws)) = state.workspace_service.get_workspace(pid) {
            for key in ["project_kb_id", "knowledge_base_id", "kb_id"] {
                if let Some(v) = ws.settings.get(key).and_then(|v| v.as_str()) {
                    if !v.trim().is_empty() {
                        return Some(v.to_string());
                    }
                }
            }
        }
    }

    state
        .knowledge_service
        .list_knowledge_bases()
        .ok()
        .and_then(|kbs| kbs.into_iter().next().map(|kb| kb.id))
}

/// 将团队角色的 system_prompt 叠加到任务变量（下次 Run 随角色更新而变）
fn apply_role_overlay(
    state: &AppState,
    team_id: &str,
    prompt: &str,
    variables: &mut serde_json::Value,
) {
    let Ok(roles) = state.teams_state.team_service.list_roles(team_id) else {
        return;
    };
    let Some(role) = roles.first() else {
        return;
    };
    let system = role.system_prompt.trim();
    if system.is_empty() {
        return;
    }

    let overlay_prompt = format!("[Role: {}]\n{}\n\n[Task]\n{}", role.name, system, prompt);

    if let Some(obj) = variables.as_object_mut() {
        let mut updated = false;
        for (_, v) in obj.iter_mut() {
            if let Some(s) = v.as_str() {
                if s.trim() == prompt {
                    *v = serde_json::Value::String(overlay_prompt.clone());
                    updated = true;
                }
            }
        }
        if !updated {
            for key in ["task", "prompt", "user_task", "input"] {
                if obj.contains_key(key) {
                    obj.insert(key.to_string(), serde_json::json!(overlay_prompt));
                    updated = true;
                    break;
                }
            }
        }
        if !updated || obj.is_empty() {
            obj.insert("task".to_string(), serde_json::json!(overlay_prompt));
        }
        obj.insert(
            "role_system_prompt".to_string(),
            serde_json::json!(system),
        );
        obj.insert("role_name".to_string(), serde_json::json!(role.name));
    }
}

/// 关键词匹配：返回最合适的工作流
fn match_workflow(
    workflows: &[crate::services::workflow_service::Workflow],
    prompt: &str,
) -> crate::services::workflow_service::Workflow {
    let lower = prompt.to_lowercase();

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
                + text
                    .split_whitespace()
                    .filter(|w| lower.contains(*w))
                    .count();
            (score, w)
        })
        .collect();

    scored
        .into_iter()
        .max_by_key(|(s, _)| *s)
        .map(|(_, w)| w.clone())
        .unwrap_or_else(|| workflows[0].clone())
}

static KEYWORDS: &[(&[&str], &[&str])] = &[
    (
        &["bug", "修复", "fix", "错误", "报错", "crash"],
        &["fix", "bug", "investigate", "quick"],
    ),
    (
        &["开发", "功能", "feature", "实现", "implement", "新增"],
        &["dev", "workflow", "feature"],
    ),
    (
        &["测试", "test", "tdd", "单元", "覆盖率"],
        &["tdd", "test", "fix"],
    ),
    (&["审查", "review", "代码质量", "code review"], &["review"]),
    (
        &["分析", "brainstorm", "头脑风暴", "方案", "设计"],
        &["brainstorm", "investigate"],
    ),
    (&["调查", "根因", "investigate", "排查"], &["investigate"]),
];

fn infer_variables(definition: &serde_json::Value, prompt: &str) -> serde_json::Value {
    let inputs = definition
        .pointer("/triggers/0/inputs")
        .or_else(|| definition.get("inputs"));

    let Some(inputs) = inputs.and_then(|v| v.as_object()) else {
        return serde_json::json!({});
    };

    let mut vars = serde_json::Map::new();
    for (key, _) in inputs {
        vars.insert(key.clone(), serde_json::json!(prompt));
    }
    serde_json::Value::Object(vars)
}
