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
pub async fn call_claude_cli(prompt: &str) -> ClaudeCliResult {
    call_claude_cli_with_timeout(prompt, 60).await  // 改为60秒，方便调试
}

/// 调用 Claude CLI 执行 prompt，带超时
pub async fn call_claude_cli_with_timeout(prompt: &str, timeout_secs: u64) -> ClaudeCliResult {
    let timeout = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        async {
            let output = Command::new("claude")
                .args(["-p", prompt])
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
        .args(["-p", "--verbose", prompt])
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_claude_cli_basic() {
        let result = call_claude_cli("Say 'hello'").await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.is_empty());
    }
}
