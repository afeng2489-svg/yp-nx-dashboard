//! Claude CLI 调用封装
//!
//! 通过本地 Claude Code CLI 调用 AI 模型，
//! Claude Switch 切换模型后会自动使用新模型。

use std::process::Command;
use std::sync::RwLock;
use tokio::process::Command as AsyncCommand;

/// Claude CLI 调用结果
pub type ClaudeCliResult = Result<String, String>;

/// Claude CLI 路径解析来源
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaudeCliSource {
    /// 用户在设置里手动指定
    User,
    /// 系统自动检测（shell PATH / 常见路径）
    Auto,
    /// 找不到
    None,
}

/// Claude CLI 解析状态
#[derive(Debug, Clone)]
pub struct ClaudeCliResolution {
    pub path: Option<String>,
    pub source: ClaudeCliSource,
}

impl Default for ClaudeCliResolution {
    fn default() -> Self {
        Self {
            path: None,
            source: ClaudeCliSource::None,
        }
    }
}

/// Claude CLI 路径运行时缓存（启动时初始化，可被用户配置覆盖）
static CLAUDE_CLI_STATE: once_cell::sync::Lazy<RwLock<ClaudeCliResolution>> =
    once_cell::sync::Lazy::new(|| RwLock::new(ClaudeCliResolution::default()));

/// 自动检索本地 Claude Code CLI 路径
fn find_claude_cli() -> Option<String> {
    tracing::info!("[Claude CLI] 开始搜索 Claude Code CLI...");

    // 0. 检查环境变量覆盖（由 Tauri 主进程传入）
    if let Ok(override_path) = std::env::var("CLAUDE_CLI_PATH_OVERRIDE") {
        if !override_path.is_empty() && std::path::Path::new(&override_path).exists() {
            tracing::info!(
                "[Claude CLI] 从环境变量 CLAUDE_CLI_PATH_OVERRIDE 发现: {}",
                override_path
            );
            return Some(override_path);
        }
        tracing::warn!(
            "[Claude CLI] CLAUDE_CLI_PATH_OVERRIDE={} 不存在，继续搜索",
            override_path
        );
    }

    // 1. 从用户 shell 环境获取完整 PATH（GUI 应用不继承 shell PATH）
    let shell_paths = get_shell_path();
    tracing::info!("[Claude CLI] shell PATH 包含 {} 个目录", shell_paths.len());

    // 2. 在完整 PATH 中搜索 claude
    // Windows: npm 安装的是 claude.cmd（批处理包装），不是 claude.exe
    let cli_names: &[&str] = if cfg!(target_os = "windows") {
        &["claude.cmd", "claude.exe", "claude"]
    } else {
        &["claude"]
    };
    for dir in &shell_paths {
        for name in cli_names {
            let candidate = std::path::Path::new(dir).join(name);
            if candidate.exists() {
                tracing::info!("[Claude CLI] 发现于: {}", candidate.display());
                return Some(candidate.to_string_lossy().to_string());
            }
        }
    }

    // 3. 常见路径兜底
    let common_paths = get_common_paths();
    tracing::info!("[Claude CLI] 检查 {} 个常见路径", common_paths.len());
    for p in &common_paths {
        if std::path::Path::new(p).exists() {
            tracing::info!("[Claude CLI] 发现于: {}", p);
            return Some(p.to_string());
        }
    }

    // 4. 终极兜底：交互式 shell 跑 which/command -v（覆盖只在 .zshrc 而非 .zprofile 配置 PATH 的用户）
    if !cfg!(target_os = "windows") {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
        for args in &[
            // -i -l：交互式 + 登录，最大限度加载 rc 文件
            vec!["-i", "-l", "-c", "command -v claude"],
            vec!["-l", "-c", "command -v claude"],
            vec!["-i", "-c", "command -v claude"],
        ] {
            let output = Command::new(&shell).args(args).output();
            if let Ok(out) = output {
                if out.status.success() {
                    let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if !path.is_empty() && std::path::Path::new(&path).exists() {
                        tracing::info!(
                            "[Claude CLI] 通过 {} {:?} command -v 发现: {}",
                            shell,
                            args,
                            path
                        );
                        return Some(path);
                    }
                }
            }
        }
    }

    tracing::warn!(
        "[Claude CLI] 未找到 Claude Code CLI。已搜索 shell PATH ({} 个目录) 和 {} 个常见路径",
        shell_paths.len(),
        common_paths.len()
    );
    tracing::debug!("[Claude CLI] 搜索过的 shell PATH: {:?}", shell_paths);
    tracing::debug!("[Claude CLI] 搜索过的常见路径: {:?}", common_paths);
    None
}

/// 从用户 shell 环境获取 PATH 列表
fn get_shell_path() -> Vec<String> {
    let separator = if cfg!(target_os = "windows") {
        ';'
    } else {
        ':'
    };

    if cfg!(target_os = "windows") {
        // Windows: 通过 PowerShell 获取用户完整 PATH
        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", "$env:PATH"])
            .output();
        if let Ok(out) = output {
            if out.status.success() {
                let path_str = String::from_utf8_lossy(&out.stdout);
                return path_str
                    .trim()
                    .split(separator)
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect();
            }
        }
    } else {
        // macOS / Linux: 通过登录 shell 获取完整 PATH
        // 关键：必须加 -i（交互式）让 shell 读 .zshrc/.bashrc，否则 nvm/fnm/volta 等
        // 在 rc 文件里动态注入 PATH 的工具都拿不到
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
        // 优先尝试 -i -l -c（交互式 + 登录 shell）
        // 兜底再用 -l -c（如果 -i 因 tty 问题失败）
        for args in &[
            vec!["-i", "-l", "-c", "echo $PATH"],
            vec!["-l", "-c", "echo $PATH"],
        ] {
            let output = Command::new(&shell).args(args).output();
            if let Ok(out) = output {
                if out.status.success() {
                    let path_str = String::from_utf8_lossy(&out.stdout);
                    let paths: Vec<String> = path_str
                        .trim()
                        .split(separator)
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string())
                        .collect();
                    if !paths.is_empty() {
                        tracing::info!(
                            "[Claude CLI] 从 {} {:?} 获取到 {} 个 PATH 目录",
                            shell,
                            args,
                            paths.len()
                        );
                        return paths;
                    }
                }
            }
        }

        // macOS: 使用 path_helper 作为后备（读取 /etc/paths 和 /etc/paths.d/）
        if cfg!(target_os = "macos") {
            let output = Command::new("/usr/libexec/path_helper").output();
            if let Ok(out) = output {
                if out.status.success() {
                    // path_helper 输出格式: PATH="/usr/local/bin:..."; export PATH;
                    let output_str = String::from_utf8_lossy(&out.stdout);
                    if let Some(start) = output_str.find("PATH=\"") {
                        let rest = &output_str[start + 6..];
                        if let Some(end) = rest.find('"') {
                            let path_str = &rest[..end];
                            let paths: Vec<String> = path_str
                                .split(separator)
                                .filter(|s| !s.is_empty())
                                .map(|s| s.to_string())
                                .collect();
                            if !paths.is_empty() {
                                tracing::info!(
                                    "[Claude CLI] 从 path_helper 获取到 {} 个 PATH 目录",
                                    paths.len()
                                );
                                return paths;
                            }
                        }
                    }
                }
            }
        }
    }

    // 回退到当前进程 PATH
    tracing::warn!("[Claude CLI] shell PATH 获取失败，使用进程 PATH");
    std::env::var("PATH")
        .unwrap_or_default()
        .split(separator)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

/// 获取常见 Claude CLI 安装路径
fn get_common_paths() -> Vec<String> {
    let mut paths = Vec::new();

    if cfg!(target_os = "windows") {
        // Windows: npm global (claude.cmd 是 npm 安装的主入口), user profile
        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            // npm 全局安装: claude.cmd（批处理包装脚本）
            paths.push(format!(r"{}\AppData\Roaming\npm\claude.cmd", userprofile));
            paths.push(format!(r"{}\AppData\Roaming\npm\claude", userprofile));
            // 直接安装
            paths.push(format!(
                r"{}\AppData\Local\Programs\claude\claude.exe",
                userprofile
            ));
            // fnm (Fast Node Manager)
            paths.push(format!(
                r"{}\AppData\Roaming\fnm\node\versions\{}\claude.cmd",
                userprofile, "latest"
            ));
            paths.push(format!(
                r"{}\AppData\Roaming\fnm\node\versions\{}\claude",
                userprofile, "latest"
            ));
        }
        if let Ok(appdata) = std::env::var("APPDATA") {
            paths.push(format!(r"{}\npm\claude.cmd", appdata));
            paths.push(format!(r"{}\npm\claude", appdata));
        }
        paths.push(r"C:\Program Files\claude\claude.exe".to_string());
    } else if cfg!(target_os = "macos") {
        // Apple Silicon homebrew
        paths.push("/opt/homebrew/bin/claude".to_string());
        // Intel homebrew
        paths.push("/usr/local/bin/claude".to_string());
        // MacPorts
        paths.push("/opt/local/bin/claude".to_string());
        paths.push("/usr/bin/claude".to_string());

        // 浅扫 /opt 下所有 */bin/claude 和 */claude（覆盖 /opt/anthropic、/opt/claude 等自定义安装）
        if let Ok(entries) = std::fs::read_dir("/opt") {
            for entry in entries.flatten() {
                let p = entry.path();
                let in_bin = p.join("bin/claude");
                if in_bin.exists() {
                    paths.push(in_bin.to_string_lossy().to_string());
                }
                let direct = p.join("claude");
                if direct.exists() && direct.is_file() {
                    paths.push(direct.to_string_lossy().to_string());
                }
            }
        }

        if let Ok(home) = std::env::var("HOME") {
            // npm global
            paths.push(format!("{}/.npm-global/bin/claude", home));
            // nvm default
            paths.push(format!("{}/.nvm/versions/node/default/bin/claude", home));
            // fnm
            paths.push(format!(
                "{}/Library/Application Support/fnm/aliases/default/bin/claude",
                home
            ));
            // volta
            paths.push(format!("{}/.volta/bin/claude", home));
            // local bin
            paths.push(format!("{}/.local/bin/claude", home));
            // Claude Code specific
            paths.push(format!("{}/.claude/bin/claude", home));
        }
    } else {
        // Linux
        paths.push("/usr/local/bin/claude".to_string());
        paths.push("/usr/bin/claude".to_string());
        paths.push("/snap/bin/claude".to_string());
        if let Ok(home) = std::env::var("HOME") {
            paths.push(format!("{}/.npm-global/bin/claude", home));
            paths.push(format!("{}/.local/bin/claude", home));
            // nvm
            paths.push(format!("{}/.nvm/versions/node/default/bin/claude", home));
            // volta
            paths.push(format!("{}/.volta/bin/claude", home));
            // fnm
            paths.push(format!(
                "{}/.local/share/fnm/aliases/default/bin/claude",
                home
            ));
            // Claude Code specific
            paths.push(format!("{}/.claude/bin/claude", home));
        }
    }

    paths
}

/// 获取 Claude CLI 路径（原始存储路径，用于显示）
pub fn get_claude_cli_path() -> Option<String> {
    CLAUDE_CLI_STATE.read().ok().and_then(|s| s.path.clone())
}

/// 获取可用于 spawn 的 Claude CLI 可执行文件路径和前置参数
///
/// Windows 上如果路径是 .js 文件，返回 (node.exe, [cli.js]) 以便 CreateProcessW 能
/// 正确执行。.cmd/.bat 文件由 Rust stdlib 自动处理，无需额外包装。
pub fn get_claude_cli_executable() -> Option<(String, Vec<String>)> {
    let path = get_claude_cli_path()?;
    Some(resolve_claude_executable(&path))
}

/// 将 Claude CLI 路径解析为可执行文件 + 前置参数
///
/// Windows: .js → 用 node.exe 执行；.cmd/.bat/.exe → 直接执行
/// Unix:   所有文件由 shebang 或直接执行处理
fn resolve_claude_executable(path: &str) -> (String, Vec<String>) {
    let p = std::path::Path::new(path);
    if cfg!(target_os = "windows") {
        match p.extension().and_then(|e| e.to_str()) {
            Some("js") => {
                tracing::info!("[Claude CLI] 检测到 .js 入口文件，使用 node 执行: {}", path);
                ("node".to_string(), vec![p.to_string_lossy().to_string()])
            }
            _ => (path.to_string(), Vec::new()),
        }
    } else {
        (path.to_string(), Vec::new())
    }
}

/// 获取当前解析状态（路径 + 来源）
pub fn get_resolution_status() -> ClaudeCliResolution {
    CLAUDE_CLI_STATE
        .read()
        .map(|s| s.clone())
        .unwrap_or_default()
}

/// 启动时初始化：读取用户配置（最高优先级）→ 否则智能搜索 → 写入 env
pub fn init_at_startup() {
    // 1. 用户在设置里指定的路径优先
    if let Some(user_path) = load_user_configured_path() {
        if std::path::Path::new(&user_path).exists() {
            tracing::info!("[Claude CLI] 使用用户配置路径: {}", user_path);
            update_state(Some(user_path.clone()), ClaudeCliSource::User);
            std::env::set_var("CLAUDE_CLI_PATH_OVERRIDE", &user_path);
            // 自检：同进程立刻读，确认 engine.rs 等同进程消费方一定拿到
            let readback = std::env::var("CLAUDE_CLI_PATH_OVERRIDE").unwrap_or_default();
            tracing::info!(
                "[Claude CLI] 自检 env CLAUDE_CLI_PATH_OVERRIDE = {} (与上一行应一致)",
                readback
            );
            // 用户配置分支也要注入 PATH，否则 claude 跑起来找不到 node
            inject_full_path_to_env();
            return;
        } else {
            tracing::warn!(
                "[Claude CLI] 用户配置路径 {} 不存在，回退到自动检测",
                user_path
            );
        }
    }

    // 2. 自动检测
    let detected = find_claude_cli();
    let source = if detected.is_some() {
        ClaudeCliSource::Auto
    } else {
        ClaudeCliSource::None
    };

    if let Some(ref p) = detected {
        // 回写到环境变量，让同进程内的 core/workflow/engine.rs 也能读到
        std::env::set_var("CLAUDE_CLI_PATH_OVERRIDE", p);
        // 自检：同进程立刻读
        let readback = std::env::var("CLAUDE_CLI_PATH_OVERRIDE").unwrap_or_default();
        tracing::info!(
            "[Claude CLI] 自检 env CLAUDE_CLI_PATH_OVERRIDE = {} (与上一行应一致)",
            readback
        );
    }
    update_state(detected, source);

    // 关键：把完整 shell PATH 注入到当前进程，让 spawn 的子进程（claude/node/...）
    // 能找到 #!/usr/bin/env node 中的 node。GUI 启动场景必须这样做
    inject_full_path_to_env();
}

/// 把登录 shell 的完整 PATH 合并到当前进程的 PATH
/// 解决 GUI 启动时 PATH 只有 /usr/bin:/bin:/usr/sbin:/sbin 导致 claude 子进程
/// 跑 `#!/usr/bin/env node` 时找不到 node 的问题
fn inject_full_path_to_env() {
    let shell_paths = get_shell_path();
    if shell_paths.is_empty() {
        tracing::warn!("[Claude CLI] 无法解析 shell PATH，子进程可能找不到 node 等依赖");
        return;
    }
    let sep = if cfg!(target_os = "windows") {
        ';'
    } else {
        ':'
    };
    let current = std::env::var("PATH").unwrap_or_default();
    let mut merged: Vec<String> = current
        .split(sep)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    let original_count = merged.len();
    for p in &shell_paths {
        if !merged.iter().any(|existing| existing == p) {
            merged.push(p.clone());
        }
    }
    let new_path = merged.join(&sep.to_string());
    std::env::set_var("PATH", &new_path);
    tracing::info!(
        "[Claude CLI] 已注入完整 PATH（{} 个目录，原 {} 个）",
        merged.len(),
        original_count
    );
    tracing::debug!("[Claude CLI] 注入后的 PATH: {}", new_path);
}

/// 用户在设置里更新路径（None = 清除用户配置改回自动检测）
pub fn set_user_path(path: Option<String>) -> Result<ClaudeCliResolution, String> {
    let trimmed = path.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());

    // 校验路径存在
    if let Some(ref p) = trimmed {
        if !std::path::Path::new(p).exists() {
            return Err(format!("路径不存在: {}", p));
        }
    }

    save_user_configured_path(trimmed.as_deref())?;

    // 立即应用
    if let Some(ref p) = trimmed {
        update_state(Some(p.clone()), ClaudeCliSource::User);
        std::env::set_var("CLAUDE_CLI_PATH_OVERRIDE", p);
    } else {
        // 用户清除了配置，重新自动检测（先把 env 清掉，避免命中旧值）
        std::env::remove_var("CLAUDE_CLI_PATH_OVERRIDE");
        let detected = find_claude_cli();
        let source = if detected.is_some() {
            ClaudeCliSource::Auto
        } else {
            ClaudeCliSource::None
        };
        if let Some(ref p) = detected {
            std::env::set_var("CLAUDE_CLI_PATH_OVERRIDE", p);
        }
        update_state(detected, source);
    }

    Ok(get_resolution_status())
}

/// 重新自动检测（不影响用户配置；如果有用户配置仍优先使用）
pub fn redetect() -> ClaudeCliResolution {
    if let Some(user_path) = load_user_configured_path() {
        if std::path::Path::new(&user_path).exists() {
            update_state(Some(user_path.clone()), ClaudeCliSource::User);
            std::env::set_var("CLAUDE_CLI_PATH_OVERRIDE", &user_path);
            return get_resolution_status();
        }
    }

    // 重新搜索时清掉旧 env，避免 find_claude_cli 命中已失效的 OVERRIDE
    std::env::remove_var("CLAUDE_CLI_PATH_OVERRIDE");
    let detected = find_claude_cli();
    let source = if detected.is_some() {
        ClaudeCliSource::Auto
    } else {
        ClaudeCliSource::None
    };
    if let Some(ref p) = detected {
        std::env::set_var("CLAUDE_CLI_PATH_OVERRIDE", p);
    }
    update_state(detected, source);
    get_resolution_status()
}

fn update_state(path: Option<String>, source: ClaudeCliSource) {
    if let Ok(mut s) = CLAUDE_CLI_STATE.write() {
        *s = ClaudeCliResolution { path, source };
    }
}

/// 用户配置文件路径
fn user_config_file() -> Option<std::path::PathBuf> {
    dirs::data_dir().map(|d| d.join("com.nx.dashboard").join("claude_cli_config.json"))
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct UserConfigFile {
    path: Option<String>,
}

fn load_user_configured_path() -> Option<String> {
    let file = user_config_file()?;
    if !file.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&file).ok()?;
    let parsed: UserConfigFile = serde_json::from_str(&content).ok()?;
    parsed.path.filter(|s| !s.is_empty())
}

fn save_user_configured_path(path: Option<&str>) -> Result<(), String> {
    let file = user_config_file().ok_or_else(|| "无法解析用户配置目录".to_string())?;
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建配置目录失败: {}", e))?;
    }
    let cfg = UserConfigFile {
        path: path.map(|s| s.to_string()),
    };
    let content =
        serde_json::to_string_pretty(&cfg).map_err(|e| format!("序列化配置失败: {}", e))?;
    std::fs::write(&file, content).map_err(|e| format!("写入配置失败: {}", e))?;
    Ok(())
}

/// 将 ChatMessage 列表转换为 prompt 字符串
pub fn messages_to_prompt(messages: &[nexus_ai::ChatMessage]) -> String {
    let mut prompt = String::new();

    for message in messages {
        match message.role.as_str() {
            "system" => {
                prompt.push_str("<system>\n");
                prompt.push_str(&message.content);
                prompt.push_str("\n</system>\n\n");
            }
            "user" => {
                prompt.push_str("<user>\n");
                prompt.push_str(&message.content);
                prompt.push_str("\n</user>\n\n");
            }
            "assistant" => {
                prompt.push_str("<assistant>\n");
                prompt.push_str(&message.content);
                prompt.push_str("\n</assistant>\n\n");
            }
            _ => {
                prompt.push_str(&message.content);
                prompt.push_str("\n\n");
            }
        }
    }

    prompt.trim().to_string()
}

/// 调用 Claude CLI 执行 prompt
///
/// Claude CLI 会自动使用本地配置的模型（由 Claude Switch 修改）
/// 如果提供了 working_directory，则使用 --project 参数切换到对应项目
pub async fn call_claude_cli(prompt: &str, working_directory: Option<&str>) -> ClaudeCliResult {
    call_claude_cli_with_timeout(prompt, 60, working_directory).await
}

/// 调用 Claude CLI 执行 prompt，带超时
pub async fn call_claude_cli_with_timeout(
    prompt: &str,
    timeout_secs: u64,
    working_directory: Option<&str>,
) -> ClaudeCliResult {
    let (exe_path, prefix_args) = get_claude_cli_executable().ok_or("Claude CLI not found")?;

    let timeout = tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), async {
        let mut cmd = AsyncCommand::new(&exe_path);
        for arg in &prefix_args {
            cmd.arg(arg);
        }
        cmd.args([
            "-p",
            "--dangerously-skip-permissions",
            "--no-session-persistence",
            prompt,
        ]);

        // 如果提供了 working_directory，设置当前工作目录
        if let Some(dir) = working_directory {
            cmd.current_dir(dir);
            tracing::info!(
                "[Claude CLI] 执行命令: cd {} && {} {} -p --dangerously-skip-permissions <prompt>",
                dir,
                exe_path,
                prefix_args.join(" ")
            );
        } else {
            tracing::info!(
                "[Claude CLI] 执行命令: {} {} -p --dangerously-skip-permissions <prompt>",
                exe_path,
                prefix_args.join(" ")
            );
        }

        let output = cmd
            .output()
            .await
            .map_err(|e| format!("Failed to execute Claude CLI: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Claude CLI error ({}): {}", output.status, stderr));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    })
    .await;

    match timeout {
        Ok(result) => result,
        Err(_) => Err("Claude CLI timed out".to_string()),
    }
}

/// 调用 Claude CLI，返回带工具调用的完整响应
/// 适用于需要解析 Claude 的 tool_use 等结构的场景
pub async fn call_claude_cli_with_tools(prompt: &str) -> ClaudeCliResult {
    let (exe_path, prefix_args) = get_claude_cli_executable().ok_or("Claude CLI not found")?;

    let mut cmd = AsyncCommand::new(&exe_path);
    for arg in &prefix_args {
        cmd.arg(arg);
    }
    let output = cmd
        .args([
            "-p",
            "--dangerously-skip-permissions",
            "--no-session-persistence",
            "--verbose",
            prompt,
        ])
        .output()
        .await
        .map_err(|e| format!("Failed to execute Claude CLI: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Claude CLI error: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(stdout)
}

/// 使用 Claude CLI 将对话摘要为结构化 JSON
///
/// 超时 15 秒，输入截断至 500 字符以控制成本
pub async fn summarize_for_memory(
    user_message: &str,
    assistant_reply: &str,
) -> Result<nx_memory::StructuredMemory, String> {
    let user_truncated: String = user_message.chars().take(500).collect();
    let assistant_truncated: String = assistant_reply.chars().take(500).collect();

    let prompt = format!(
        r#"Summarize this conversation into JSON. Return ONLY valid JSON, no markdown fences, no explanation.
Format: {{"topic":"<topic>","problem":"<what user asked>","solution":"<key answer>","keywords":["<kw1>","<kw2>","<kw3>"],"role":"assistant","timestamp":"{}","summary":"<1 sentence summary>"}}

User: {}
Assistant: {}"#,
        chrono::Utc::now().format("%Y-%m-%d %H:%M"),
        user_truncated,
        assistant_truncated,
    );

    let raw = call_claude_cli_with_timeout(&prompt, 15, None).await?;
    let json_str = extract_json_from_response(&raw)?;

    serde_json::from_str::<nx_memory::StructuredMemory>(&json_str).map_err(|e| {
        format!(
            "JSON parse error: {} from: {}",
            e,
            json_str.chars().take(200).collect::<String>()
        )
    })
}

/// 扩展搜索查询为关键词集合（本地处理，无需 Claude CLI）
///
/// 去除停用词后提取有意义的词项，避免额外的 Claude 冷启动开销。
pub async fn expand_query_for_search(query: &str) -> Result<String, String> {
    Ok(extract_keywords_local(query))
}

/// 本地关键词提取：分词 + 停用词过滤 + 去重
fn extract_keywords_local(query: &str) -> String {
    // 常见中英文停用词
    const STOP_WORDS: &[&str] = &[
        // 中文
        "的", "了", "是", "在", "我", "有", "和", "就", "不", "人", "都", "一", "一个", "上", "也",
        "很", "到", "说", "要", "去", "你", "会", "着", "没有", "看", "好", "自己", "这", "那",
        "什么", "为", "吗", "呢", "啊", "吧", "么", "呀", "哦", "这个", "那个", "我们", "他们",
        "她们", "它们", "可以", "如何", "怎么", "哪些", // 英文
        "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by",
        "from", "is", "are", "was", "were", "be", "been", "have", "has", "had", "do", "does",
        "did", "will", "would", "could", "should", "may", "might", "this", "that", "these",
        "those", "i", "you", "he", "she", "it", "we", "they", "what", "which", "who", "how",
        "when", "where", "why", "about", "as", "if", "so", "than", "then", "there",
    ];

    // 按空白和常见标点分词
    let words: Vec<&str> = query
        .split(|c: char| {
            c.is_whitespace()
                || matches!(
                    c,
                    '，' | '。'
                        | '！'
                        | '？'
                        | '、'
                        | '：'
                        | '；'
                        | ','
                        | '.'
                        | '!'
                        | '?'
                        | ':'
                        | ';'
                        | '('
                        | ')'
                        | '（'
                        | '）'
                        | '['
                        | ']'
                        | '"'
                        | '"'
                        | '\''
                        | '"'
                )
        })
        .filter(|w| !w.is_empty())
        .collect();

    let mut seen = std::collections::HashSet::new();
    let mut result = String::from(query.trim());

    for word in words {
        let lower = word.to_lowercase();
        if !STOP_WORDS.contains(&lower.as_str()) && word.chars().count() >= 2 && seen.insert(lower)
        {
            result.push(' ');
            result.push_str(word);
        }
    }

    result
}

/// 从可能包含 markdown fence 的响应中提取 JSON 对象
fn extract_json_from_response(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();

    // 直接以 { 开头
    if trimmed.starts_with('{') {
        if let Some(end) = trimmed.rfind('}') {
            return Ok(trimmed[..=end].to_string());
        }
    }

    // 尝试在 markdown fence 中找到 JSON
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            return Ok(trimmed[start..=end].to_string());
        }
    }

    Err(format!(
        "No JSON found in response: {}",
        trimmed.chars().take(200).collect::<String>()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_claude_cli_basic() {
        if get_claude_cli_path().is_none() {
            eprintln!("Skipping: Claude CLI not installed");
            return;
        }
        let result = call_claude_cli("Say 'hello'", None).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.is_empty());
    }
}
