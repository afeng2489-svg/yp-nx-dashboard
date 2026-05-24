use nexus_workflow::state::WorkflowState;

/// Verify that project-level variables are injected into WorkflowState
/// and can be retrieved after injection.
#[test]
fn test_execution_service_injects_project_vars() {
    let mut state = WorkflowState::new("page-generate-workflow");

    // Simulate what the execution service does: inject project context variables
    state.set_var(
        "project_kb_id",
        serde_json::Value::String("kb-frontend-components".into()),
    );
    state.set_var(
        "project_path",
        serde_json::Value::String("/workspace/my-app".into()),
    );
    state.set_var(
        "project_id",
        serde_json::Value::String("project-abc-123".into()),
    );
    state.set_var(
        "escalate_model",
        serde_json::Value::String("claude-opus-4-5".into()),
    );

    // Verify each injected variable is stored correctly
    assert_eq!(
        state.get_var("project_kb_id").and_then(|v| v.as_str()),
        Some("kb-frontend-components")
    );
    assert_eq!(
        state.get_var("project_path").and_then(|v| v.as_str()),
        Some("/workspace/my-app")
    );
    assert_eq!(
        state.get_var("project_id").and_then(|v| v.as_str()),
        Some("project-abc-123")
    );
    assert_eq!(
        state.get_var("escalate_model").and_then(|v| v.as_str()),
        Some("claude-opus-4-5")
    );

    // Verify template resolution uses injected vars
    let prompt = "Generate a page for project {{project_id}}. Use knowledge base {{project_kb_id}}. Escalate to {{escalate_model}} on failure.";
    let resolved = state.resolve_template(prompt);

    assert!(resolved.contains("project-abc-123"));
    assert!(resolved.contains("kb-frontend-components"));
    assert!(resolved.contains("claude-opus-4-5"));
    assert!(!resolved.contains("{{project_id}}"));
    assert!(!resolved.contains("{{project_kb_id}}"));
    assert!(!resolved.contains("{{escalate_model}}"));
}
