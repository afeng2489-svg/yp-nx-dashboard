pub struct QualityGateResult {
    pub passed: bool,
    pub checks: Vec<CheckResult>,
}

pub struct CheckResult {
    pub cmd: String,
    pub passed: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl std::fmt::Display for QualityGateResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let passed = self.checks.iter().filter(|c| c.passed).count();
        write!(f, "质量门: {}/{} 通过", passed, self.checks.len())
    }
}

pub fn run_quality_gate(working_dir: Option<&str>) -> Option<QualityGateResult> {
    let dir = working_dir?;

    let checks = if std::path::Path::new(&format!("{}/Cargo.toml", dir)).exists() {
        vec![("cargo build", 300u64), ("cargo test", 300)]
    } else if std::path::Path::new(&format!("{}/package.json", dir)).exists() {
        vec![("npx tsc --noEmit", 300), ("npm test", 300)]
    } else if std::path::Path::new(&format!("{}/go.mod", dir)).exists() {
        vec![("go build ./...", 300), ("go test ./...", 300)]
    } else if std::path::Path::new(&format!("{}/pyproject.toml", dir)).exists()
        || std::path::Path::new(&format!("{}/setup.py", dir)).exists()
    {
        vec![("python -m pytest", 300)]
    } else {
        return None;
    };

    let mut results = Vec::new();
    let mut all_passed = true;

    for (cmd, _timeout) in &checks {
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .current_dir(dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output();

        let result = match output {
            Ok(out) => {
                let passed = out.status.success();
                if !passed { all_passed = false; }
                CheckResult {
                    cmd: cmd.to_string(),
                    passed,
                    exit_code: out.status.code(),
                    stdout: String::from_utf8_lossy(&out.stdout).chars().take(2000).collect(),
                    stderr: String::from_utf8_lossy(&out.stderr).chars().take(2000).collect(),
                }
            }
            Err(e) => {
                all_passed = false;
                CheckResult {
                    cmd: cmd.to_string(),
                    passed: false,
                    exit_code: None,
                    stdout: String::new(),
                    stderr: e.to_string(),
                }
            }
        };
        tracing::info!("[QualityGate] '{}' → {}", cmd, if result.passed { "PASS" } else { "FAIL" });
        results.push(result);
    }

    Some(QualityGateResult { passed: all_passed, checks: results })
}
