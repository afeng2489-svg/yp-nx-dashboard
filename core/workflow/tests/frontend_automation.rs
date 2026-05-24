mod common;

use nexus_workflow::artifacts::PageManifest;
use nexus_workflow::state::WorkflowState;
use nexus_workflow::watchers::page_generate::PageGenerateWatcher;

use common::{
    full_manifest, manifest_with_missing_component, minimal_manifest, setup_staging_dir_with_files,
};

// ────────────────────────────────────────────
// Test 1: PageManifest serde roundtrip
// ────────────────────────────────────────────

#[test]
fn test_page_manifest_serde_roundtrip() {
    let manifest = full_manifest();

    let json = serde_json::to_string_pretty(&manifest).expect("Serialize should succeed");
    let roundtripped: PageManifest =
        serde_json::from_str(&json).expect("Deserialize should succeed");

    assert_eq!(roundtripped.page_name, manifest.page_name);
    assert_eq!(roundtripped.routes.len(), manifest.routes.len());
    assert_eq!(roundtripped.components.len(), manifest.components.len());
    assert_eq!(roundtripped.data_models.len(), manifest.data_models.len());
    assert_eq!(roundtripped.imports.len(), manifest.imports.len());

    // Spot-check nested fields
    assert_eq!(roundtripped.routes[0].path, "/full");
    assert_eq!(roundtripped.routes[0].auth_required, true);
    assert_eq!(
        roundtripped.components[0].uses_data_model,
        Some("UserModel".into())
    );
    assert_eq!(roundtripped.data_models[0].fields.len(), 3);
    assert_eq!(roundtripped.data_models[0].mock_data.is_some(), true);
    assert_eq!(roundtripped.imports[1].from_module, "./components/Header");
}

// ────────────────────────────────────────────
// Test 2: Watcher PASS when all files match
// ────────────────────────────────────────────

#[test]
fn test_page_generate_watcher_r1_r9_pass() {
    let manifest = minimal_manifest();
    let (staging_dir, manifest) = setup_staging_dir_with_files(&manifest);

    let review = PageGenerateWatcher::validate(staging_dir.path(), &manifest);

    // R1/R2/R4 should all pass since we created matching files
    // R3/R9: tsc --noEmit may fail if npx is unavailable — ignore those
    let non_tsc_failures: Vec<_> = review
        .failures
        .iter()
        .filter(|f| f.rule != "R3/R9")
        .collect();
    assert!(
        non_tsc_failures.is_empty(),
        "Unexpected non-tsc failures: {:?}",
        non_tsc_failures
    );

    // TempDir will be cleaned up on drop
}

// ────────────────────────────────────────────
// Test 3: Watcher catches missing ComponentSpec file (R1)
// ────────────────────────────────────────────

#[test]
fn test_page_generate_watcher_r1_missing_file() {
    let manifest = manifest_with_missing_component();
    let dir = tempfile::tempdir().expect("Failed to create temp directory");
    // Do NOT create the component file — R1 should catch it

    let review = PageGenerateWatcher::validate(dir.path(), &manifest);

    assert_eq!(review.verdict, "MANIFEST_MISMATCH");
    let r1_failures: Vec<_> = review.failures.iter().filter(|f| f.rule == "R1").collect();
    assert!(!r1_failures.is_empty());
    let detail = &r1_failures[0].detail;
    assert!(detail.contains("MissingComponent"));
    assert!(detail.contains("components/MissingComponent.tsx"));
}

// ────────────────────────────────────────────
// Test 4: resolve_template replaces injected project vars
// ────────────────────────────────────────────

#[test]
fn test_resolve_template_injected_variables() {
    let mut state = WorkflowState::new("test-workflow");

    state.set_var("project_kb_id", serde_json::Value::String("kb-123".into()));
    state.set_var(
        "project_path",
        serde_json::Value::String("/home/user/project".into()),
    );
    state.set_var("project_id", serde_json::Value::String("proj-456".into()));
    state.set_var(
        "escalate_model",
        serde_json::Value::String("claude-opus-4-5".into()),
    );

    let template =
        "Work on project {{project_id}} at path {{project_path}} using KB {{project_kb_id}}. Fallback model: {{escalate_model}}.";
    let resolved = state.resolve_template(template);

    assert!(resolved.contains("proj-456"));
    assert!(resolved.contains("/home/user/project"));
    assert!(resolved.contains("kb-123"));
    assert!(resolved.contains("claude-opus-4-5"));
    assert!(!resolved.contains("{{project_id}}"));
    assert!(!resolved.contains("{{project_path}}"));
}

// ────────────────────────────────────────────
// Test 5: Unresolved templates pass through as literals
// ────────────────────────────────────────────

#[test]
fn test_resolve_template_not_injected_returns_literal() {
    let state = WorkflowState::new("test-workflow");

    let template = "The value is {{nonexistent_var}} and also {{another_missing}}.";
    let resolved = state.resolve_template(template);

    // Unresolved placeholders should remain as-is
    assert!(resolved.contains("{{nonexistent_var}}"));
    assert!(resolved.contains("{{another_missing}}"));
}
