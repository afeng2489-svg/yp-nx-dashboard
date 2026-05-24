use std::fs;

use nexus_workflow::artifacts::{
    ComponentSpec, DataField, DataModelSpec, ImportMapping, PageManifest, RouteDefinition,
};

/// Creates a temp staging directory with fixture files matching the given manifest.
/// Returns (staging_dir, manifest).
pub fn setup_staging_dir_with_files(manifest: &PageManifest) -> (tempfile::TempDir, PageManifest) {
    let dir = tempfile::tempdir().expect("Failed to create temp directory");

    // Create minimal tsconfig.json so tsc --noEmit passes
    fs::write(
        dir.path().join("tsconfig.json"),
        r#"{"compilerOptions": {"strict": true, "jsx": "react-jsx", "module": "esnext", "target": "es2020", "moduleResolution": "bundler", "skipLibCheck": true}}"#,
    )
    .ok();
    fs::write(dir.path().join("package.json"), r#"{"name": "test"}"#).ok();

    // Create component files
    for comp in &manifest.components {
        let path = dir.path().join(&comp.file_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).ok();
        }
        fs::write(&path, "// fixture component\n").ok();
    }

    // Create data model files
    for dm in &manifest.data_models {
        let data_dir = dir.path().join("data");
        fs::create_dir_all(&data_dir).ok();
        let path = data_dir.join(format!("{}.ts", dm.name));
        fs::write(&path, "// fixture data model\n").ok();
    }

    // Create import files for relative imports
    for imp in &manifest.imports {
        if imp.from_module.starts_with('.') {
            let resolved = dir.path().join(&imp.from_module);
            // Try .ts extension
            let ts_path = if resolved.extension().is_none() {
                resolved.with_extension("ts")
            } else {
                resolved.clone()
            };
            if let Some(parent) = ts_path.parent() {
                fs::create_dir_all(parent).ok();
            }
            fs::write(&ts_path, "// fixture import\n").ok();
        }
    }

    (dir, manifest.clone())
}

/// Creates a minimal manifest with one of everything.
pub fn minimal_manifest() -> PageManifest {
    PageManifest {
        page_name: "test_page".into(),
        routes: vec![RouteDefinition {
            path: "/test".into(),
            component_name: "TestPage".into(),
            layout: None,
            auth_required: false,
        }],
        components: vec![ComponentSpec {
            name: "TestComponent".into(),
            file_path: "components/TestComponent.tsx".into(),
            props: vec!["title".into()],
            children: vec![],
            uses_data_model: None,
        }],
        data_models: vec![DataModelSpec {
            name: "TestModel".into(),
            fields: vec![DataField {
                name: "id".into(),
                field_type: "string".into(),
                required: true,
            }],
            methods: vec![],
            mock_data: None,
        }],
        imports: vec![ImportMapping {
            from_module: "./components/TestComponent".into(),
            import_names: vec!["TestComponent".into()],
        }],
    }
}

/// Creates a manifest that references a non-existent component file.
pub fn manifest_with_missing_component() -> PageManifest {
    PageManifest {
        page_name: "missing_page".into(),
        routes: vec![RouteDefinition {
            path: "/missing".into(),
            component_name: "MissingComponent".into(),
            layout: None,
            auth_required: false,
        }],
        components: vec![ComponentSpec {
            name: "MissingComponent".into(),
            file_path: "components/MissingComponent.tsx".into(),
            props: vec![],
            children: vec![],
            uses_data_model: None,
        }],
        data_models: vec![],
        imports: vec![],
    }
}

/// Creates a fully populated manifest for serde roundtrip testing.
pub fn full_manifest() -> PageManifest {
    PageManifest {
        page_name: "FullPage".into(),
        routes: vec![
            RouteDefinition {
                path: "/full".into(),
                component_name: "FullPage".into(),
                layout: Some("DashboardLayout".into()),
                auth_required: true,
            },
            RouteDefinition {
                path: "/full/settings".into(),
                component_name: "SettingsPage".into(),
                layout: None,
                auth_required: true,
            },
        ],
        components: vec![
            ComponentSpec {
                name: "FullPage".into(),
                file_path: "pages/FullPage.tsx".into(),
                props: vec!["user".into(), "config".into()],
                children: vec!["Header".into(), "Sidebar".into()],
                uses_data_model: Some("UserModel".into()),
            },
            ComponentSpec {
                name: "Header".into(),
                file_path: "components/Header.tsx".into(),
                props: vec!["title".into()],
                children: vec![],
                uses_data_model: None,
            },
        ],
        data_models: vec![DataModelSpec {
            name: "UserModel".into(),
            fields: vec![
                DataField {
                    name: "id".into(),
                    field_type: "string".into(),
                    required: true,
                },
                DataField {
                    name: "email".into(),
                    field_type: "string".into(),
                    required: true,
                },
                DataField {
                    name: "age".into(),
                    field_type: "number".into(),
                    required: false,
                },
            ],
            methods: vec!["fetchAll".into(), "findById".into()],
            mock_data: Some(serde_json::json!({
                "id": "1",
                "email": "test@example.com",
                "age": 30
            })),
        }],
        imports: vec![
            ImportMapping {
                from_module: "react".into(),
                import_names: vec!["useState".into(), "useEffect".into()],
            },
            ImportMapping {
                from_module: "./components/Header".into(),
                import_names: vec!["Header".into()],
            },
        ],
    }
}
