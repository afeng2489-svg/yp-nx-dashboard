//! Claude CLI 调用封装
//!
//! 通过本地 Claude Code CLI 调用 AI 模型，
//! Claude Switch 切换模型后会自动使用新模型。

use tokio::process::Command;

/// Claude CLI 调用结果
pub type ClaudeCliResult = Result<String, String>;

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
pub async fn call_claude_cli_with_timeout(prompt: &str, timeout_secs: u64, working_directory: Option<&str>) -> ClaudeCliResult {
    let timeout = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        async {
            let mut cmd = Command::new("claude");
            cmd.args(["-p", "--dangerously-skip-permissions", "--no-session-persistence", prompt]);

            // 如果提供了 working_directory，设置当前工作目录
            if let Some(dir) = working_directory {
                cmd.current_dir(dir);
                tracing::info!("[Claude CLI] 执行命令: cd {} && claude -p --dangerously-skip-permissions <prompt>", dir);
            } else {
                tracing::info!("[Claude CLI] 执行命令: claude -p --dangerously-skip-permissions <prompt>");
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
        }
    ).await;

    match timeout {
        Ok(result) => result,
        Err(_) => Err("Claude CLI timed out".to_string())
    }
}

/// 调用 Claude CLI，返回带工具调用的完整响应
/// 适用于需要解析 Claude 的 tool_use 等结构的场景
pub async fn call_claude_cli_with_tools(prompt: &str) -> ClaudeCliResult {
    let output = Command::new("claude")
        .args(["-p", "--dangerously-skip-permissions", "--no-session-persistence", "--verbose", prompt])
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

    serde_json::from_str::<nx_memory::StructuredMemory>(&json_str)
        .map_err(|e| format!("JSON parse error: {} from: {}", e, json_str.chars().take(200).collect::<String>()))
}

/// 使用 Claude CLI 扩展搜索查询为关键词集合
///
/// 超时 10 秒，短查询（< 10 字符）直接返回
pub async fn expand_query_for_search(query: &str) -> Result<String, String> {
    if query.len() < 10 {
        return Ok(query.to_string());
    }

    let query_truncated: String = query.chars().take(300).collect();
    let prompt = format!(
        "Extract 5-8 search keywords from this query. Return ONLY the keywords separated by spaces, no explanation, no numbering.\nQuery: {}",
        query_truncated
    );

    let expanded = call_claude_cli_with_timeout(&prompt, 10, None).await?;

    // 合并原始 query 和扩展关键词
    Ok(format!("{} {}", query, expanded.trim()))
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
        let result = call_claude_cli("Say 'hello'", None).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.is_empty());
    }
}
