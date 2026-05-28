//! Integration tests for the workflow engine.
//!
//! These tests exercise the engine end-to-end using mock event emitters
//! and in-memory workflow definitions (no Claude CLI calls).

use nexus_workflow::events::{EventEmitter, InMemoryEventEmitter};
use nexus_workflow::parser::{StageType, WorkflowParser};
use nexus_workflow::{WorkflowEngine, WorkflowState, WorkflowStatus};
use std::sync::Arc;
use std::time::Duration;

fn make_engine() -> WorkflowEngine {
    WorkflowEngine::new(Arc::new(InMemoryEventEmitter::new()))
}

// ── Parsing + validation ──

#[test]
fn parse_and_validate_two_agent_workflow() {
    let yaml = r#"
name: "Simple"
agents:
  - id: "a1"
    role: "tester"
    model: "test-model"
    prompt: "return ok"
  - id: "a2"
    role: "tester"
    model: "test-model"
    prompt: "return ok"
    depends_on: ["a1"]
stages:
  - name: "First"
    agents: ["a1"]
  - name: "Second"
    agents: ["a2"]
"#;
    let wf = WorkflowParser::parse(yaml).unwrap();
    WorkflowParser::validate(&wf).unwrap();
    assert_eq!(wf.name, "Simple");
    assert_eq!(wf.stages.len(), 2);
}

// ── Workflow variables ──

#[test]
fn workflow_variables_initialized_in_state() {
    let yaml = r#"
name: "VarInit"
variables:
  target: "production"
  retries: 3
stages:
  - name: "S1"
"#;
    let wf = WorkflowParser::parse(yaml).unwrap();
    let mut state = WorkflowState::new(&wf.name);
    for (key, value) in &wf.variables {
        state.set_var(key, value.clone());
    }
    assert_eq!(
        state.get_var("target").unwrap().as_str().unwrap(),
        "production"
    );
    assert_eq!(state.get_var("retries").unwrap().as_i64().unwrap(), 3);
}

// ── UserInput stage (no resume channel → default option) ──

#[tokio::test]
async fn user_input_stage_uses_default_option_without_channel() {
    let yaml = r#"
name: "UserInputTest"
stages:
  - name: "Choose"
    stage_type: user_input
    question: "Pick one"
    options:
      - label: "Yes"
        value: "yes"
      - label: "No"
        value: "no"
    output_var: "choice"
"#;
    let wf = WorkflowParser::parse(yaml).unwrap();
    let engine = make_engine();

    let result = engine.execute(&wf).await;
    assert!(result.is_ok());
    let r = result.unwrap();
    assert_eq!(r.status, WorkflowStatus::Completed);
    assert_eq!(r.variables.get("choice").unwrap().as_str().unwrap(), "yes");
}

#[tokio::test]
async fn user_input_defaults_to_first_option() {
    let yaml = r#"
name: "InteractiveWorkflow"
stages:
  - name: "Pick"
    stage_type: user_input
    question: "Choose env"
    options:
      - label: "Staging"
        value: "staging"
      - label: "Production"
        value: "production"
    output_var: "env"
"#;
    let wf = WorkflowParser::parse(yaml).unwrap();
    let engine = make_engine();

    let result = engine.execute(&wf).await;
    assert!(result.is_ok());
    let r = result.unwrap();
    assert_eq!(r.status, WorkflowStatus::Completed);
    assert_eq!(r.variables.get("env").unwrap().as_str().unwrap(), "staging");
}

// ── Loop stage parsing ──

#[test]
fn parse_loop_workflow() {
    let yaml = r#"
name: "LoopTest"
agents:
  - id: "worker"
    role: "dev"
    model: "m"
    prompt: "do work"
stages:
  - name: "RetryLoop"
    stage_type: loop
    break_condition: "done == 'true'"
    body_stages: ["DoWork"]
    max_iterations: 3
  - name: "DoWork"
    agents: ["worker"]
"#;
    let wf = WorkflowParser::parse(yaml).unwrap();
    assert_eq!(wf.stages[0].stage_type, StageType::Loop);
    assert_eq!(wf.stages[0].max_iterations, 3);
    assert_eq!(wf.stages[0].body_stages, vec!["DoWork"]);
}

// ── Error conversions ──

#[test]
fn workflow_error_from_io() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let wf_err: nexus_workflow::WorkflowError = io_err.into();
    assert!(wf_err.to_string().contains("file not found"));
}

// ── Minimal workflow execution ──

#[test]
fn execute_minimal_workflow() {
    let engine = make_engine();
    let yaml = r#"
name: "Minimal"
stages:
  - name: "Only"
"#;
    let wf = WorkflowParser::parse(yaml).unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(engine.execute(&wf)).unwrap();

    assert_eq!(result.status, WorkflowStatus::Completed);
    assert_eq!(result.stage_results.len(), 1);
    assert_eq!(result.stage_results[0].stage_name, "Only");
}

// ── Event emission ──

#[tokio::test]
async fn execution_emits_start_and_complete_events() {
    let yaml = r#"
name: "EventTest"
stages:
  - name: "S1"
"#;
    let wf = WorkflowParser::parse(yaml).unwrap();
    let emitter = Arc::new(InMemoryEventEmitter::new());
    let mut rx = emitter.subscribe();
    let engine = WorkflowEngine::new(emitter);

    let result = engine.execute(&wf).await;
    assert!(result.is_ok());

    let mut events = Vec::new();
    while let Ok(Some(event)) =
        tokio::time::timeout(Duration::from_millis(200), rx.recv()).await
    {
        events.push(event);
    }

    let started = events.iter().any(|e| {
        matches!(
            e,
            nexus_workflow::events::WorkflowEvent::WorkflowStarted { .. }
        )
    });
    let completed = events.iter().any(|e| {
        matches!(
            e,
            nexus_workflow::events::WorkflowEvent::WorkflowCompleted { .. }
        )
    });
    assert!(started, "expected WorkflowStarted event among {events:?}");
    assert!(completed, "expected WorkflowCompleted event among {events:?}");
}

// ── Budget limit ──

#[test]
fn budget_limit_is_optional() {
    let yaml = r#"
name: "NoBudget"
stages:
  - name: "S1"
"#;
    let wf = WorkflowParser::parse(yaml).unwrap();
    assert!(wf.budget_limit_usd.is_none());
}

#[test]
fn budget_limit_parsed_correctly() {
    let yaml = r#"
name: "Budgeted"
budget_limit_usd: 100.0
stages:
  - name: "S1"
"#;
    let wf = WorkflowParser::parse(yaml).unwrap();
    assert_eq!(wf.budget_limit_usd, Some(100.0));
}

// ── Stage transitions ──

#[test]
fn stage_with_conditional_transitions() {
    let yaml = r#"
name: "Conditional"
agents:
  - id: "a"
    role: "r"
    model: "m"
    prompt: "p"
stages:
  - name: "Check"
    agents: ["a"]
    next:
      - condition: "status == 'ok'"
        goto: "Deploy"
      - goto: "Rollback"
  - name: "Deploy"
    agents: ["a"]
  - name: "Rollback"
    agents: ["a"]
"#;
    let wf = WorkflowParser::parse(yaml).unwrap();
    assert_eq!(wf.stages.len(), 3);
    assert_eq!(wf.stages[0].next.len(), 2);
    assert_eq!(wf.stages[0].next[0].goto, "Deploy");
    assert_eq!(wf.stages[0].next[1].goto, "Rollback");
}

// ── Working directory ──

#[test]
fn engine_with_working_directory() {
    let emitter = Arc::new(InMemoryEventEmitter::new());
    let engine =
        WorkflowEngine::with_working_directory(emitter, Some("/tmp/project".into()));
    // Just verify construction succeeds; working_directory is a private field
    // tested via unit tests
    let _ = engine;
}

// ── Watcher and provider injection ──

#[test]
fn engine_add_stage_watcher() {
    use nexus_workflow::watcher::StageWatcher;

    struct DummyWatcher;
    impl StageWatcher for DummyWatcher {
        fn before_stage(&self, _execution_id: &str, _stage_name: &str) {}
        fn after_stage(&self, _execution_id: &str, _stage_name: &str) {}
    }

    let mut engine = make_engine();
    engine.add_stage_watcher(Arc::new(DummyWatcher));
}

#[test]
fn engine_set_rag_provider() {
    use nexus_workflow::watcher::RagProvider;

    struct DummyRag;
    #[async_trait::async_trait]
    impl RagProvider for DummyRag {
        async fn retrieve(
            &self,
            _kb_id: &str,
            _query: &str,
            _top_k: usize,
            _threshold: f32,
        ) -> Vec<String> {
            vec!["mock result".into()]
        }
    }

    let mut engine = make_engine();
    engine.set_rag_provider(Arc::new(DummyRag));
}
