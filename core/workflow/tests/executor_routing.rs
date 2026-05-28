//! AF-04b: API executor lane integration (mock, no real API key).

use std::sync::Arc;

use nexus_workflow::engine::WorkflowEngine;
use nexus_workflow::events::{EventCollector, EventEmitter, WorkflowEvent};
use nexus_workflow::executor::{
    ApiCompletionRequest, ApiCompletionResult, ApiExecutor, ExecutorKind,
};
use nexus_workflow::parser::{WorkflowError, WorkflowParser};

struct MockApiExecutor;

#[async_trait::async_trait]
impl ApiExecutor for MockApiExecutor {
    async fn complete(
        &self,
        _request: ApiCompletionRequest,
    ) -> Result<ApiCompletionResult, WorkflowError> {
        Ok(ApiCompletionResult {
            text: "交付完成\n```json\n{\"summary\":\"mock summary\",\"files_changed\":[\"README.md\"]}\n```"
                .into(),
            input_tokens: 12,
            output_tokens: 34,
            provider: "mock".into(),
            estimated_cost_usd: 0.0001,
        })
    }
}

struct CollectorEmitter {
    inner: Arc<EventCollector>,
}

impl EventEmitter for CollectorEmitter {
    fn emit(&self, event: WorkflowEvent) {
        self.inner.record(event);
    }

    fn subscribe(&self) -> tokio::sync::mpsc::Receiver<WorkflowEvent> {
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        drop(tx);
        rx
    }
}

#[tokio::test]
async fn api_executor_stage_emits_token_usage_with_executor_api() {
    let yaml = r#"
name: api-test
agents:
  - id: summary
    role: writer
    model: test-model
    executor: api
    prompt: "写交付摘要"
stages:
  - name: 交付摘要
    executor: api
    agents: [summary]
"#;
    let def = WorkflowParser::parse(yaml).unwrap();
    assert_eq!(def.stages[0].executor, Some(ExecutorKind::Api));

    let collector = Arc::new(EventCollector::new());
    let emitter = Arc::new(CollectorEmitter {
        inner: collector.clone(),
    });
    let mut engine = WorkflowEngine::new(emitter);
    engine.set_api_executor(Arc::new(MockApiExecutor));

    let result = engine.execute(&def).await.expect("workflow should complete");
    assert_eq!(result.status, nexus_workflow::WorkflowStatus::Completed);

    let events = collector.get_events();
    let token = events.iter().find_map(|e| match e {
        WorkflowEvent::AgentTokenUsage {
            executor,
            provider,
            ..
        } if executor == "api" => Some(provider.clone()),
        _ => None,
    });
    assert_eq!(token.as_deref(), Some("mock"));
}
