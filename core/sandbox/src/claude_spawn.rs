//! Claude Code CLI 启动参数解析（workflow / nx_api 共用）

use std::path::Path;

use crate::cli_security::push_prompt_args;

/// 从环境变量解析 Claude CLI 可执行文件：(program, prefix_args)
///
/// 优先级：`CLAUDE_CLI_PATH_OVERRIDE` → `CLAUDE_BIN` → 常见路径 → PATH 中的 `claude`
pub fn claude_cli_spawn_spec() -> Result<(String, Vec<String>), String> {
    if let Ok(p) = std::env::var("CLAUDE_CLI_PATH_OVERRIDE") {
        let p = p.trim().to_string();
        if !p.is_empty() {
            return spawn_spec_from_path(&p);
        }
    }
    if let Ok(p) = std::env::var("CLAUDE_BIN") {
        let p = p.trim().to_string();
        if !p.is_empty() {
            return spawn_spec_from_path(&p);
        }
    }

    let candidates: Vec<String> = if cfg!(target_os = "windows") {
        vec![
            "claude.cmd".into(),
            "claude.exe".into(),
            "claude".into(),
        ]
    } else {
        vec![
            "/opt/homebrew/bin/claude".into(),
            "/usr/local/bin/claude".into(),
            "claude".into(),
        ]
    };

    for c in candidates {
        if Path::new(&c).exists() {
            return spawn_spec_from_path(&c);
        }
    }

    Err(
        "未找到 Claude Code CLI。请在「设置 → AI」配置路径，或安装: npm install -g @anthropic-ai/claude-code"
            .to_string(),
    )
}

fn spawn_spec_from_path(path: &str) -> Result<(String, Vec<String>), String> {
    if !Path::new(path).exists() {
        return Err(format!("Claude CLI 路径不存在: {}", path));
    }
    Ok(resolve_claude_executable(path))
}

/// Windows `.js` 入口需 node 包装；Unix 直接执行（shebang / Mach-O）
pub fn resolve_claude_executable(path: &str) -> (String, Vec<String>) {
    let p = Path::new(path);
    if cfg!(target_os = "windows") {
        match p.extension().and_then(|e| e.to_str()) {
            Some("js") => ("node".to_string(), vec![p.to_string_lossy().to_string()]),
            _ => (path.to_string(), Vec::new()),
        }
    } else {
        (path.to_string(), Vec::new())
    }
}

/// 构建 workflow 用的 Claude CLI 参数（stream-json + prompt 模式 flags）
pub fn build_workflow_cli_args(model: Option<&str>) -> Vec<String> {
    let mut args = Vec::new();
    push_prompt_args(&mut args, true);
    args.push("--verbose".to_string());
    args.push("--output-format".to_string());
    args.push("stream-json".to_string());
    if let Some(m) = model {
        args.push("--model".to_string());
        args.push(m.to_string());
    }
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_js_on_windows() {
        if cfg!(target_os = "windows") {
            let (exe, prefix) = resolve_claude_executable(r"C:\npm\claude.js");
            assert_eq!(exe, "node");
            assert_eq!(prefix, vec![r"C:\npm\claude.js"]);
        }
    }
}
