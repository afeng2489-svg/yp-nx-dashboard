//! Workflow Integration Tests
//!
//! Tests for workflow execution and state management.

use nx_api::services::ExecutionService;
use nx_api::services::execution_service::{Execution, ExecutionStatus};

#[test]
fn test_workflow_execution_states() {
    let service = ExecutionService::new();

    // Start execution
    let execution = service.start_execution("test-workflow".to_string(), serde_json::json!({
        "input": "test"
    }));

    assert_eq!(execution.status, ExecutionStatus::Running);
    assert!(execution.started_at.is_some());
    assert!(execution.finished_at.is_none());

    // Complete execution
    service.update_status(&execution.id, ExecutionStatus::Completed);

    let found = service.get_execution(&execution.id).unwrap();
    assert_eq!(found.status, ExecutionStatus::Completed);
    assert!(found.finished_at.is_some());
}

#[test]
fn test_workflow_execution_failure() {
    let service = ExecutionService::new();
    let execution = service.start_execution("failing-workflow".to_string(), serde_json::json!({}));

    service.update_status(&execution.id, ExecutionStatus::Failed);

    let found = service.get_execution(&execution.id).unwrap();
    assert_eq!(found.status, ExecutionStatus::Failed);
}

#[test]
fn test_workflow_execution_cancellation() {
    let service = ExecutionService::new();
    let execution = service.start_execution("long-running-workflow".to_string(), serde_json::json!({}));

    // Simulate long running
    service.add_stage_output(&execution.id, "stage-1".to_string(), serde_json::json!({}));

    // Cancel
    let cancelled = service.cancel_execution(&execution.id);
    assert!(cancelled);

    let found = service.get_execution(&execution.id).unwrap();
    assert_eq!(found.status, ExecutionStatus::Cancelled);
    assert!(found.finished_at.is_some());
}

#[test]
fn test_workflow_stage_sequencing() {
    let service = ExecutionService::new();
    let execution = service.start_execution("multi-stage-workflow".to_string(), serde_json::json!({}));

    let stages = ["initialize", "validate", "process", "finalize"];

    for stage in &stages {
        service.add_stage_output(
            &execution.id,
            stage.to_string(),
            serde_json::json!({"stage": stage, "completed": true}),
        );
    }

    let found = service.get_execution(&execution.id).unwrap();
    assert_eq!(found.stage_results.len(), 4);

    for (i, stage) in stages.iter().enumerate() {
        assert_eq!(found.stage_results[i].stage_name, *stage);
    }
}

#[test]
fn test_multiple_concurrent_workflows() {
    let service = ExecutionService::new();

    // Start multiple workflows
    let executions: Vec<Execution> = (0..5)
        .map(|i| service.start_execution(format!("workflow-{}", i), serde_json::json!({})))
        .collect();

    assert_eq!(service.get_all_executions().len(), 5);

    // Complete some
    service.update_status(&executions[0].id, ExecutionStatus::Completed);
    service.update_status(&executions[2].id, ExecutionStatus::Completed);

    // Fail one
    service.update_status(&executions[4].id, ExecutionStatus::Failed);

    let all = service.get_all_executions();
    let completed = all.iter().filter(|e| e.status == ExecutionStatus::Completed).count();
    let failed = all.iter().filter(|e| e.status == ExecutionStatus::Failed).count();
    let running = all.iter().filter(|e| e.status == ExecutionStatus::Running).count();

    assert_eq!(completed, 2);
    assert_eq!(failed, 1);
    assert_eq!(running, 2);
}

#[test]
fn test_workflow_variables_preserved() {
    let service = ExecutionService::new();
    let variables = serde_json::json!({
        "name": "test-workflow",
        "parameters": {
            "timeout": 300,
            "retry": true
        },
        "inputs": ["file1.txt", "file2.txt"]
    });

    let execution = service.start_execution("var-workflow".to_string(), variables.clone());
    let found = service.get_execution(&execution.id).unwrap();

    assert_eq!(found.variables, variables);
}

#[test]
fn test_simulate_execution_returns_completed_execution() {
    let service = ExecutionService::new();
    let result = service.simulate_execution("test-workflow".to_string());

    assert_eq!(result.workflow_id, "test-workflow");
    assert_eq!(result.status, ExecutionStatus::Completed);
    assert!(result.finished_at.is_some());
    assert!(!result.stage_results.is_empty());
}
