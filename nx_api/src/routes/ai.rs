//! AI Intent Parsing Routes
//!
//! AI-powered parse-intent endpoint for QuickLaunch page.

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::routes::AppState;
use crate::services::claude_cli::call_claude_cli_with_timeout;

#[derive(Debug, Deserialize)]
pub struct ParseIntentRequest {
    pub input: String,
    #[serde(default)]
    pub context: Option<String>,
    #[serde(default)]
    pub screenshot_url: Option<String>,
    #[serde(default)]
    pub reference_url: Option<String>,
    #[serde(default)]
    pub api_doc: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ParseIntentResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_components: Option<Vec<SuggestedComponent>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_data_models: Option<Vec<SuggestedDataModel>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub needs_clarification: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestions: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SuggestedComponent {
    pub name: String,
    pub component_type: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SuggestedDataModel {
    pub name: String,
    pub fields: Vec<String>,
    pub description: String,
}

/// POST /api/v1/ai/parse-intent
///
/// Uses Claude CLI to parse natural language input into structured page requirements.
/// Returns structured requirements if confidence >= 0.5, otherwise asks for clarification.
pub async fn parse_intent(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ParseIntentRequest>,
) -> Result<Json<ParseIntentResponse>, (StatusCode, String)> {
    let prompt = build_parse_intent_prompt(&req);

    let working_dir = state.current_workspace_path.read().clone();
    let output = call_claude_cli_with_timeout(&prompt, 120, working_dir.as_deref())
        .await
        .map_err(|e| {
            tracing::error!("Parse intent failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("AI parsing failed: {}", e),
            )
        })?;

    // Extract JSON from Claude output (it may be wrapped in markdown code blocks)
    let json_str = extract_json_from_output(&output);

    match serde_json::from_str::<serde_json::Value>(json_str) {
        Ok(v) => {
            let confidence = v["confidence"].as_f64().unwrap_or(0.5);

            if confidence < 0.5 {
                let suggestions: Vec<String> = v["suggestions"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|s| s.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                Ok(Json(ParseIntentResponse {
                    page_name: None,
                    description: None,
                    suggested_components: None,
                    suggested_data_models: None,
                    confidence: Some(confidence),
                    needs_clarification: Some(true),
                    suggestions: Some(suggestions),
                }))
            } else {
                let page_name = v["page_name"].as_str().map(String::from);
                let description = v["description"].as_str().map(String::from);
                let suggested_components = v["suggested_components"].as_array().map(|a| {
                    a.iter()
                        .filter_map(|c| {
                            Some(SuggestedComponent {
                                name: c["name"].as_str()?.to_string(),
                                component_type: c["component_type"]
                                    .as_str()
                                    .unwrap_or("component")
                                    .to_string(),
                                description: c["description"].as_str().unwrap_or("").to_string(),
                            })
                        })
                        .collect()
                });
                let suggested_data_models = v["suggested_data_models"].as_array().map(|a| {
                    a.iter()
                        .filter_map(|m| {
                            Some(SuggestedDataModel {
                                name: m["name"].as_str()?.to_string(),
                                fields: m["fields"]
                                    .as_array()
                                    .map(|fa| {
                                        fa.iter()
                                            .filter_map(|f| f.as_str().map(String::from))
                                            .collect()
                                    })
                                    .unwrap_or_default(),
                                description: m["description"].as_str().unwrap_or("").to_string(),
                            })
                        })
                        .collect()
                });

                Ok(Json(ParseIntentResponse {
                    page_name,
                    description,
                    suggested_components,
                    suggested_data_models,
                    confidence: Some(confidence),
                    needs_clarification: Some(false),
                    suggestions: None,
                }))
            }
        }
        Err(e) => {
            tracing::warn!(
                "Failed to parse Claude output as JSON: {}. Raw output: {}",
                e,
                &output[..output.len().min(500)]
            );
            // Fallback: return clarification request with raw output as suggestion
            Ok(Json(ParseIntentResponse {
                page_name: None,
                description: None,
                suggested_components: None,
                suggested_data_models: None,
                confidence: Some(0.0),
                needs_clarification: Some(true),
                suggestions: Some(vec![
                    "AI 返回内容格式异常，请用更具体的语言描述您的页面需求".to_string(),
                ]),
            }))
        }
    }
}

fn build_parse_intent_prompt(req: &ParseIntentRequest) -> String {
    let mut prompt = String::from(
        "你是一个前端页面需求分析助手。根据用户的自然语言输入，提取结构化的页面需求。\n\n\
        输出必须是有效的 JSON，格式如下:\n\
        {\n  \"page_name\": \"页面名称\",\n  \"description\": \"页面描述\",\n  \
        \"suggested_components\": [\n    { \"name\": \"组件名\", \"component_type\": \"component|hook|type|util\", \"description\": \"组件描述\" }\n  ],\n  \
        \"suggested_data_models\": [\n    { \"name\": \"模型名\", \"fields\": [\"字段1\", \"字段2\"], \"description\": \"模型描述\" }\n  ],\n  \
        \"confidence\": 0.0-1.0\n}\n\n\
        如果需求不够明确或无法提取足够信息，设置 confidence < 0.5 并提供 suggestions 数组:\n\
        {\n  \"confidence\": 0.3,\n  \"suggestions\": [\"建议1\", \"建议2\"]\n}\n\n\
        只输出 JSON，不要输出其他内容。\n\n",
    );

    prompt.push_str(&format!("用户输入: {}\n", req.input));

    if let Some(ref ctx) = req.context {
        prompt.push_str(&format!("补充上下文: {}\n", ctx));
    }
    if let Some(ref url) = req.screenshot_url {
        prompt.push_str(&format!("截图参考: {}\n", url));
    }
    if let Some(ref url) = req.reference_url {
        prompt.push_str(&format!("参考网站: {}\n", url));
    }
    if let Some(ref doc) = req.api_doc {
        prompt.push_str(&format!("API 文档: {}\n", doc));
    }

    prompt
}

/// Extract JSON string from Claude output, removing markdown code fences if present.
fn extract_json_from_output(output: &str) -> &str {
    let output = output.trim();

    // Try to find JSON between ```json ... ``` fences
    if let Some(start) = output.find("```json") {
        let after_start = &output[start + 7..];
        if let Some(end) = after_start.find("```") {
            return after_start[..end].trim();
        }
    }

    // Try to find JSON between ``` ... ``` fences
    if let Some(start) = output.find("```") {
        let after_start = &output[start + 3..];
        if let Some(end) = after_start.find("```") {
            return after_start[..end].trim();
        }
    }

    // Try to find { and } to extract JSON object
    if let Some(start) = output.find('{') {
        if let Some(end) = output.rfind('}') {
            return &output[start..=end];
        }
    }

    output
}
