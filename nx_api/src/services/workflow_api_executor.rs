//! Bridges nexus_ai::AIModelManager to workflow ApiExecutor (AF-04b / AF-MM-02).

use std::sync::Arc;

use nexus_ai::{AIModelManager, AIRequestParams, ModelConfig, ProviderType};
use nexus_workflow::executor::{ApiCompletionRequest, ApiCompletionResult, ApiExecutor};
use nexus_workflow::parser::WorkflowError;

use super::text_lane_cost::resolve_text_lane_model;

pub struct WorkflowApiExecutor {
    manager: Arc<AIModelManager>,
}

impl WorkflowApiExecutor {
    pub fn new(manager: Arc<AIModelManager>) -> Arc<Self> {
        Arc::new(Self { manager })
    }
}

#[async_trait::async_trait]
impl ApiExecutor for WorkflowApiExecutor {
    async fn complete(
        &self,
        request: ApiCompletionRequest,
    ) -> Result<ApiCompletionResult, WorkflowError> {
        let routed_model = resolve_text_lane_model(
            &request.model,
            request.stage_name.as_deref(),
            request.cost_mode.as_deref(),
        );

        let model = self.manager.resolve_routable_model(&routed_model);

        let params = AIRequestParams::completion(model, request.user_message)
            .with_system_prompt(request.system_prompt);

        let resp = self
            .manager
            .complete(params)
            .await
            .map_err(|e| WorkflowError::Validation(format!("API 执行失败: {e}")))?;

        let estimated_cost_usd = estimate_cost(&resp.provider, resp.input_tokens, resp.output_tokens);

        Ok(ApiCompletionResult {
            text: resp.text,
            input_tokens: resp.input_tokens as u64,
            output_tokens: resp.output_tokens as u64,
            provider: resp.provider,
            estimated_cost_usd,
        })
    }
}

fn estimate_cost(provider: &str, input: usize, output: usize) -> f64 {
    let (in_rate, out_rate) = match provider.to_lowercase().as_str() {
        "openai" | "gpt" => (2.5, 10.0),
        "google" | "gemini" => (1.25, 5.0),
        "deepseek" => (0.14, 0.28),
        "ollama" | "local" => (0.0, 0.0),
        _ => (3.0, 15.0),
    };
    (input as f64 * in_rate / 1_000_000.0) + (output as f64 * out_rate / 1_000_000.0)
}
