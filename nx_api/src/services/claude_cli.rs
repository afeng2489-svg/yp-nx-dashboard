//! Claude CLI 调用封装
//!
//! 通过本地 Claude Code CLI 调用 AI 模型，
//! Claude Switch 切换模型后会自动使用新模型。

use std::process::Command;
use tokio::process::Command as AsyncCommand;

/// Claude CLI 调用结果
pub type ClaudeCliResult = Result<String, String>;

/// Claude CLI 路径缓存
static CLAUDE_CLI_PATH: once_cell::sync::Lazy<Option<String>> =
    once_cell::sync::Lazy::new(find_claude_cli);

/// 自动检索本地 Claude Code CLI 路径
fn find_claude_cli() -> Option<String> {
    // 1. 先尝试 which claude
    if let Ok(path) = std::process::Command::new("which")
        .arg("claude")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    {
        if !path.is_empty() && std::path::Path::new(&path).exists() {
            tracing::debug!("[Claude CLI] 发现于: {}", path);
            return Some(path);
        }
    }

    // 2. 尝试 command -v claude
    if let Ok(path) = std::process::Command::new("command")
        .args(["-v", "claude"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    {
        if !path.is_empty() && std::path::Path::new(&path).exists() {
            tracing::debug!("[Claude CLI] 发现于: {}", path);
            return Some(path);
        }
    }

    // 3. 常见 macOS 路径
    let common_paths = [
        "/opt/homebrew/bin/claude",
        "/usr/local/bin/claude",
        "/usr/bin/claude",
    ];
    for p in &common_paths {
        if std::path::Path::new(p).exists() {
            tracing::debug!("[Claude CLI] 发现于: {}", p);
            return Some(p.to_string());
        }
    }

    // 4. 尝试从 PATH 环境变量查找
    if let Ok(path) = std::process::Command::new("zsh")
        .args(["-c", "whence -p claude"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    {
        if !path.is_empty() && std::path::Path::new(&path).exists() {
            tracing::debug!("[Claude CLI] 发现于: {}", path);
            return Some(path);
        }
    }

    tracing::warn!("[Claude CLI] 未找到 Claude Code CLI");
    None
}

/// 获取 Claude CLI 路径
pub fn get_claude_cli_path() -> Option<String> {
    (*CLAUDE_CLI_PATH).clone()
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
    let cli_path = get_claude_cli_path().ok_or("Claude CLI not found")?;

    let timeout = tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), async {
        let mut cmd = AsyncCommand::new(&cli_path);
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
                "[Claude CLI] 执行命令: cd {} && {} -p --dangerously-skip-permissions <prompt>",
                dir,
                cli_path
            );
        } else {
            tracing::info!(
                "[Claude CLI] 执行命令: {} -p --dangerously-skip-permissions <prompt>",
                cli_path
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
    let cli_path = get_claude_cli_path().ok_or("Claude CLI not found")?;

    let output = AsyncCommand::new(&cli_path)
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
        if CLAUDE_CLI_PATH.is_none() {
            eprintln!("Skipping: Claude CLI not installed");
            return;
        }
        let result = call_claude_cli("Say 'hello'", None).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.is_empty());
    }
}
