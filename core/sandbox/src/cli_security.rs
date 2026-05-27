//! Claude CLI 安全策略：permissions 模式、workspace 边界、危险命令拦截

use parking_lot::RwLock;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use thiserror::Error;

/// CLI 权限模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PermissionsMode {
    /// 不传 --dangerously-skip-permissions（默认）
    #[default]
    Strict,
    /// 开发/内测：自动跳过 Claude 权限确认
    Trusted,
}

impl PermissionsMode {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "strict" => Some(Self::Strict),
            "trusted" => Some(Self::Trusted),
            _ => None,
        }
    }

    pub fn from_env() -> Self {
        if let Some(m) = *RUNTIME_MODE.read() {
            return m;
        }
        if let Ok(v) = std::env::var("NEXUS_PERMISSIONS_MODE") {
            if let Some(m) = Self::parse(&v) {
                return m;
            }
        }
        PermissionsMode::Strict
    }

    pub fn allows_skip_permissions(self) -> bool {
        matches!(self, Self::Trusted)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Strict => "strict",
            Self::Trusted => "trusted",
        }
    }
}

static RUNTIME_MODE: LazyLock<RwLock<Option<PermissionsMode>>> =
    LazyLock::new(|| RwLock::new(None));

/// 运行时覆盖（API / 设置页），优先于 env
pub fn set_runtime_permissions_mode(mode: Option<PermissionsMode>) {
    *RUNTIME_MODE.write() = mode;
}

pub fn runtime_permissions_mode() -> Option<PermissionsMode> {
    *RUNTIME_MODE.read()
}

#[derive(Debug, Error)]
pub enum CliSecurityError {
    #[error("工作目录必须在 workspace 内: {work_dir} 不在 {workspace} 下")]
    WorkspaceBoundary { work_dir: String, workspace: String },
    #[error("无法解析路径: {path}: {reason}")]
    InvalidPath { path: String, reason: String },
    #[error("strict 模式必须指定 workspace")]
    WorkspaceRequired,
    #[error("内容命中安全 blocklist: {pattern}")]
    BlocklistMatch { pattern: String },
}

/// 向 Claude CLI 参数列表追加 `-p` 与可选的 skip-permissions
pub fn push_prompt_args(args: &mut Vec<String>, no_session_persistence: bool) {
    push_prompt_args_with_mode(args, no_session_persistence, PermissionsMode::from_env());
}

pub fn push_prompt_args_with_mode(
    args: &mut Vec<String>,
    no_session_persistence: bool,
    mode: PermissionsMode,
) {
    args.push("-p".to_string());
    if mode.allows_skip_permissions() {
        args.push("--dangerously-skip-permissions".to_string());
    }
    if no_session_persistence {
        args.push("--no-session-persistence".to_string());
    }
}

/// 向 Command 追加 prompt 模式参数（不含 prompt 正文）
pub fn apply_prompt_flags<I>(args: &mut I, no_session_persistence: bool)
where
    I: Extend<String>,
{
    let mut v = Vec::new();
    push_prompt_args(&mut v, no_session_persistence);
    args.extend(v);
}

/// 校验 working_directory 位于 workspace_root 内
pub fn validate_working_directory(
    work_dir: &str,
    workspace_root: Option<&str>,
) -> Result<PathBuf, CliSecurityError> {
    let mode = PermissionsMode::from_env();
    let work_path = Path::new(work_dir);

    if workspace_root.is_none() {
        if mode == PermissionsMode::Strict && work_dir.is_empty() {
            return Err(CliSecurityError::WorkspaceRequired);
        }
        if work_path.exists() {
            return work_path.canonicalize().map_err(|e| CliSecurityError::InvalidPath {
                path: work_dir.to_string(),
                reason: e.to_string(),
            });
        }
        return Ok(work_path.to_path_buf());
    }

    let root = workspace_root.unwrap();
    let root_canon = canonicalize_existing(root)?;
    let work_canon = if work_path.exists() {
        canonicalize_existing(work_dir)?
    } else {
        // 允许尚未创建的子目录：基于父路径解析
        let parent = work_path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .and_then(|p| p.canonicalize().ok());
        if let Some(parent_canon) = parent {
            let joined = parent_canon.join(
                work_path
                    .file_name()
                    .unwrap_or_else(|| work_path.as_os_str()),
            );
            joined
        } else {
            return Err(CliSecurityError::InvalidPath {
                path: work_dir.to_string(),
                reason: "path does not exist".to_string(),
            });
        }
    };

    if !work_canon.starts_with(&root_canon) {
        return Err(CliSecurityError::WorkspaceBoundary {
            work_dir: work_canon.display().to_string(),
            workspace: root_canon.display().to_string(),
        });
    }

    Ok(work_canon)
}

fn canonicalize_existing(path: &str) -> Result<PathBuf, CliSecurityError> {
    Path::new(path).canonicalize().map_err(|e| CliSecurityError::InvalidPath {
        path: path.to_string(),
        reason: e.to_string(),
    })
}

/// 默认危险模式（quality_gate shell / prompt 扫描）
pub fn default_blocklist_patterns() -> Vec<&'static str> {
    vec![
        r"rm\s+-rf\s+/",
        r"rm\s+-rf\s+/\s",
        r"rm\s+-rf\s+/\*",
        r"curl[^\n|]*\|\s*(ba)?sh",
        r"wget[^\n|]*\|\s*(ba)?sh",
        r"chmod\s+777",
        r"mkfs\.",
        r"dd\s+if=[^\n]+of=/dev/",
        r">\s*/dev/sd[a-z]",
        r":\(\)\{\s*:\|:&\s*\};:",
        r"sudo\s+rm\s+-rf",
        r"format\s+[a-z]:",
    ]
}

static BLOCKLIST: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    default_blocklist_patterns()
        .into_iter()
        .filter_map(|p| Regex::new(&format!("(?i){p}")).ok())
        .collect()
});

/// 扫描文本是否命中 blocklist
pub fn check_blocklist(text: &str) -> Result<(), CliSecurityError> {
    for re in BLOCKLIST.iter() {
        if re.is_match(text) {
            return Err(CliSecurityError::BlocklistMatch {
                pattern: re.as_str().to_string(),
            });
        }
    }
    Ok(())
}

/// 执行 quality_gate / shell 前校验
pub fn validate_shell_command(cmd: &str) -> Result<(), CliSecurityError> {
    check_blocklist(cmd)
}

/// Claude prompt 提交前校验（blocklist + 可选 workspace 提示）
pub fn validate_prompt(prompt: &str, working_dir: Option<&str>, workspace: Option<&str>) -> Result<(), CliSecurityError> {
    check_blocklist(prompt)?;
    if let Some(dir) = working_dir {
        validate_working_directory(dir, workspace)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn strict_no_skip_flag() {
        let mut args = Vec::new();
        push_prompt_args_with_mode(&mut args, false, PermissionsMode::Strict);
        assert!(!args.iter().any(|a| a.contains("skip-permissions")));
    }

    #[test]
    fn trusted_has_skip_flag() {
        let mut args = Vec::new();
        push_prompt_args_with_mode(&mut args, true, PermissionsMode::Trusted);
        assert!(args.contains(&"--dangerously-skip-permissions".to_string()));
        assert!(args.contains(&"--no-session-persistence".to_string()));
    }

    #[test]
    fn blocklist_catches_rm_rf() {
        assert!(check_blocklist("please run rm -rf / now").is_err());
    }

    #[test]
    fn blocklist_allows_cargo_test() {
        assert!(validate_shell_command("cargo test -p nexus_workflow --lib").is_ok());
    }

    #[test]
    fn workspace_must_be_inside_root() {
        let tmp = env::temp_dir();
        let root = tmp.join("nexus_ws_root");
        let inner = root.join("proj");
        let _ = std::fs::create_dir_all(&inner);
        let outside = tmp.join("nexus_outside");
        let _ = std::fs::create_dir_all(&outside);

        assert!(validate_working_directory(
            inner.to_str().unwrap(),
            Some(root.to_str().unwrap())
        )
        .is_ok());

        assert!(validate_working_directory(
            outside.to_str().unwrap(),
            Some(root.to_str().unwrap())
        )
        .is_err());

        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&outside);
    }
}
