//! Webhook 触发路由

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

use super::AppState;

#[derive(Debug, Deserialize)]
pub struct WebhookQuery {
    pub secret: Option<String>,
}

/// POST /api/v1/triggers/webhook/:workflow_id - Webhook 触发工作流
pub async fn trigger_webhook(
    State(state): State<Arc<AppState>>,
    Path(workflow_id): Path<String>,
    Query(query): Query<WebhookQuery>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // 获取工作流
    let workflow = state
        .workflow_service
        .get_workflow(&workflow_id)
        .map_err(|_| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({"error": format!("工作流 {} 不存在", workflow_id)})),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({"error": format!("工作流 {} 不存在", workflow_id)})),
            )
        })?;

    // 检查 triggers 配置
    let triggers = workflow
        .definition
        .get("triggers")
        .and_then(|t| t.as_array())
        .cloned()
        .unwrap_or_default();

    // 找 webhook 类型的 trigger
    let webhook_trigger = triggers.iter().find(|t| {
        t.get("type")
            .and_then(|v| v.as_str())
            .map(|s| s == "webhook")
            .unwrap_or(false)
    });

    // 校验 secret
    if let Some(trigger) = webhook_trigger {
        let expected_secret = trigger.get("secret").and_then(|s| s.as_str());
        if let Some(expected) = expected_secret {
            let provided = query.secret.as_deref().unwrap_or("");
            if expected != provided {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(json!({"error": "Webhook secret 验证失败"})),
                ));
            }
        }
    } else {
        // 没有 webhook trigger 配置也允许触发（宽松模式）
        tracing::debug!(
            "[Webhook] 工作流 {} 没有 webhook trigger 配置，仍然触发",
            workflow_id
        );
    }

    // 构建工作流 YAML
    let mut workflow_def = serde_json::json!({
        "name": workflow.name,
        "version": workflow.version,
    });
    if let Some(desc) = workflow.description {
        workflow_def["description"] = serde_json::json!(desc);
    }
    if let Some(obj) = workflow.definition.as_object() {
        for (key, value) in obj {
            if !["name", "version", "description"].contains(&key.as_str()) {
                workflow_def[key] = value.clone();
            }
        }
    }

    let workflow_yaml = serde_yaml::to_string(&workflow_def).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("YAML 序列化失败: {}", e)})),
        )
    })?;

    // 使用 body 作为 variables
    let variables = body
        .as_object()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .fold(serde_json::json!({}), |mut acc, (k, v)| {
            acc.as_object_mut().unwrap().insert(k, v);
            acc
        });

    let current_workspace = state.current_workspace_path.read().clone();

    let execution_id = state
        .execution_service
        .execute_workflow(
            workflow.id.clone(),
            &workflow_yaml,
            variables,
            None,
            current_workspace,
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("执行启动失败: {}", e)})),
            )
        })?;

    Ok(Json(json!({
        "execution_id": execution_id,
        "workflow_id": workflow.id,
        "status": "running",
    })))
}
