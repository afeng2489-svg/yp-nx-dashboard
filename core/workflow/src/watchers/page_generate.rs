use std::path::Path;
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::artifacts::PageManifest;

#[derive(Debug, Clone)]
pub struct ReviewResult {
    pub verdict: String,
    pub failures: Vec<Failure>,
    pub fixes_applied: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Failure {
    pub rule: String,
    pub detail: String,
}

pub struct PageGenerateWatcher;

impl PageGenerateWatcher {
    pub fn validate(staging_dir: &Path, manifest: &PageManifest) -> ReviewResult {
        let mut failures = Vec::new();

        // R1: ComponentSpec.name → file exists at file_path
        for comp in &manifest.components {
            let path = staging_dir.join(&comp.file_path);
            if !path.exists() {
                failures.push(Failure {
                    rule: "R1".into(),
                    detail: format!(
                        "ComponentSpec '{}' expects file at {}",
                        comp.name, comp.file_path
                    ),
                });
            }
        }

        // R2: ImportMapping → valid path or npm package
        for imp in &manifest.imports {
            if imp.from_module.starts_with('.') {
                let resolved = staging_dir.join(&imp.from_module);
                if !resolved.exists()
                    && !resolved.with_extension("ts").exists()
                    && !resolved.with_extension("tsx").exists()
                {
                    failures.push(Failure {
                        rule: "R2".into(),
                        detail: format!("Import path '{}' not resolved", imp.from_module),
                    });
                }
            }
        }

        // R4: DataModelSpec.name → data file exists
        for dm in &manifest.data_models {
            let path = staging_dir.join("data").join(format!("{}.ts", dm.name));
            if !path.exists() {
                failures.push(Failure {
                    rule: "R4".into(),
                    detail: format!("DataModelSpec '{}' expects data/{}.ts", dm.name, dm.name),
                });
            }
        }

        // R3 + R9: run tsc --noEmit on staging directory (only if tsconfig.json exists)
        if staging_dir.join("tsconfig.json").exists() {
            let tsc_result = Command::new("npx")
                .args(["tsc", "--noEmit"])
                .current_dir(staging_dir)
                .output();
            match tsc_result {
                Ok(output) if output.status.success() => {}
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    failures.push(Failure {
                        rule: "R3/R9".into(),
                        detail: format!("tsc --noEmit failed:\n{}", stderr),
                    });
                }
                Err(e) => {
                    tracing::warn!("tsc not available for R3/R9: {}", e);
                }
            }
        }

        let verdict = if failures.is_empty() {
            "PASS"
        } else {
            "MANIFEST_MISMATCH"
        };

        ReviewResult {
            verdict: verdict.into(),
            failures,
            fixes_applied: vec![],
        }
    }
}
