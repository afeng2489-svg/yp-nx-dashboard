//! PTY Task Watcher
//!
//! Subscribes to a PTY session's output after a task is dispatched,
//! detects completion via quiet timeout, extracts clean response,
//! and bridges to the AgentExecutionEvent system.

use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use crate::services::agent_team_service::strip_ansi;
use crate::services::claude_terminal::ClaudeTerminalSession;
use crate::services::screen_emu::ScreenEmu;
use crate::ws::agent_execution::AgentExecutionEvent;

/// Maximum execution time (30 minutes)
const MAX_EXECUTION_SECS: u64 = 1800;
/// Quiet timeout: 1500ms of silence after receiving output → task complete
const QUIET_TIMEOUT_MS: u64 = 1500;
/// Minimum execution time before any completion check (avoid startup false positives)
const MIN_EXECUTION_SECS: u64 = 15;
/// Maximum accumulated output (1 MB)
const MAX_ACCUMULATED_BYTES: usize = 1024 * 1024;
/// Minimum interval between Progress events
const PROGRESS_DEBOUNCE_MILLIS: u64 = 800;

pub async fn watch_pty_task(
    execution_id: String,
    session: Arc<ClaudeTerminalSession>,
    event_tx: broadcast::Sender<AgentExecutionEvent>,
    cancel_token: CancellationToken,
    manager: crate::ws::agent_execution::AgentExecutionManager,
) {
    let start = Instant::now();
    let mut output_rx = session.subscribe_output();
    let mut accumulated = String::new();
    let mut emu = ScreenEmu::new();
    let mut last_output_time = Instant::now();
    let mut has_received_output = false;
    let mut last_progress_time = Instant::now() - Duration::from_millis(PROGRESS_DEBOUNCE_MILLIS);
    let exec_id_for_log = execution_id.clone();

    tracing::info!("[PtyWatcher] 开始监听, execution_id: {}", exec_id_for_log);

    let mut heartbeat_interval = tokio::time::interval(Duration::from_secs(5));
    heartbeat_interval.tick().await;

    let mut check_interval = tokio::time::interval(Duration::from_millis(100));

    loop {
        tokio::select! {
            output = output_rx.recv() => {
                match output {
                    Ok(raw_bytes) => {
                        // 喂给屏幕模拟器：按 VT100 协议在内存里渲染屏幕
                        emu.feed(&raw_bytes);

                        // 取出已"凝固"的行（光标已离开，不会再被覆盖）
                        // 这就是用户实际能看到的稳定文字，干净 markdown
                        let committed = emu.drain_committed();

                        last_output_time = Instant::now();
                        has_received_output = true;

                        if !committed.is_empty() {
                            let preview: String = committed.chars().take(200).collect();
                            tracing::debug!("[PtyWatcher] committed: {:?}", preview);

                            // 累积用于最终结果提取（保持与原 strip_ansi 路径相同的下游接口）
                            if accumulated.len() + committed.len() < MAX_ACCUMULATED_BYTES {
                                accumulated.push_str(&committed);
                            } else {
                                let excess = (accumulated.len() + committed.len()) - MAX_ACCUMULATED_BYTES;
                                if excess < accumulated.len() {
                                    accumulated = accumulated[excess..].to_string();
                                }
                                accumulated.push_str(&committed);
                            }

                            let _ = event_tx.send(AgentExecutionEvent::Output {
                                execution_id: execution_id.clone(),
                                partial_output: committed,
                            });
                        }

                        // Progress detection 仍然用原始字节流（带 ANSI），更敏感
                        // claude 的 progress 标记（"⏵ Reading..."、"● Searching..."）会出现在
                        // spinner 行，那行不会被 commit，但我们想及早探测到
                        let progress_event = {
                            let now = Instant::now();
                            if now.duration_since(last_progress_time).as_millis() as u64
                                >= PROGRESS_DEBOUNCE_MILLIS
                            {
                                let raw_text = String::from_utf8_lossy(&raw_bytes);
                                let raw_clean = strip_ansi(&raw_text);
                                if let Some((action, detail)) = detect_claude_action(&raw_clean) {
                                    last_progress_time = now;
                                    Some(AgentExecutionEvent::Progress {
                                        execution_id: execution_id.clone(),
                                        action,
                                        detail,
                                    })
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        };

                        if let Some(evt) = progress_event {
                            let _ = event_tx.send(evt);
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("[PtyWatcher] 落后 {} 帧, execution_id: {}", n, exec_id_for_log);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::info!(
                            "[PtyWatcher] PTY 通道关闭, execution_id: {}",
                            exec_id_for_log
                        );
                        // Flush emu 里剩余的所有行（包括 cursor 当前所在行）
                        let remaining = emu.drain_remaining();
                        if !remaining.is_empty() {
                            accumulated.push_str(&remaining);
                            let _ = event_tx.send(AgentExecutionEvent::Output {
                                execution_id: execution_id.clone(),
                                partial_output: remaining,
                            });
                        }
                        let result = extract_result_best_effort(&accumulated);
                        emit_completed(&execution_id, result, &start, &event_tx, &manager);
                        return;
                    }
                }
            }

            _ = heartbeat_interval.tick() => {
                let _ = event_tx.send(AgentExecutionEvent::Thinking {
                    execution_id: execution_id.clone(),
                    elapsed_secs: start.elapsed().as_secs(),
                });
            }

            _ = check_interval.tick() => {
                if !has_received_output { continue; }
                let elapsed_secs = start.elapsed().as_secs();
                let quiet_ms = last_output_time.elapsed().as_millis() as u64;

                // 静默超时：收到过输出 + 至少运行了 MIN_EXECUTION_SECS + 1500ms 无新输出
                if elapsed_secs >= MIN_EXECUTION_SECS && quiet_ms >= QUIET_TIMEOUT_MS {
                    tracing::info!(
                        "[PtyWatcher] 静默超时完成 (elapsed={}s, quiet={}ms), execution_id: {}",
                        elapsed_secs,
                        quiet_ms,
                        exec_id_for_log
                    );
                    // Flush emu 里剩余的所有行（cursor 当前行也算）
                    let remaining = emu.drain_remaining();
                    if !remaining.is_empty() {
                        accumulated.push_str(&remaining);
                        let _ = event_tx.send(AgentExecutionEvent::Output {
                            execution_id: execution_id.clone(),
                            partial_output: remaining,
                        });
                    }
                    let result = extract_result_best_effort(&accumulated);
                    emit_completed(&execution_id, result, &start, &event_tx, &manager);
                    return;
                }

                if elapsed_secs >= MAX_EXECUTION_SECS {
                    tracing::warn!("[PtyWatcher] 超时 ({}s), execution_id: {}", elapsed_secs, exec_id_for_log);
                    let _ = event_tx.send(AgentExecutionEvent::Failed {
                        execution_id: execution_id.clone(),
                        error: format!("执行超时 ({}秒)", elapsed_secs),
                    });
                    manager.remove_execution(&execution_id);
                    return;
                }
            }

            _ = cancel_token.cancelled() => {
                tracing::info!("[PtyWatcher] 被取消, execution_id: {}", exec_id_for_log);
                let _ = event_tx.send(AgentExecutionEvent::Cancelled {
                    execution_id: execution_id.clone(),
                });
                manager.remove_execution(&execution_id);
                return;
            }
        }
    }
}

/// Send Completed event and clean up
fn emit_completed(
    execution_id: &str,
    result: String,
    start: &Instant,
    event_tx: &broadcast::Sender<AgentExecutionEvent>,
    manager: &crate::ws::agent_execution::AgentExecutionManager,
) {
    let duration_ms = start.elapsed().as_millis() as u64;
    let _ = event_tx.send(AgentExecutionEvent::Completed {
        execution_id: execution_id.to_string(),
        result,
        duration_ms,
    });
    manager.cache_terminal_event(AgentExecutionEvent::Completed {
        execution_id: execution_id.to_string(),
        result: String::new(),
        duration_ms,
    });
    manager.remove_execution(execution_id);
}

// ─── 结果提取 ───────────────────────────────────────────────────────────────

fn extract_result_best_effort(accumulated: &str) -> String {
    clean_tui_output(accumulated)
}

/// 清洗 TUI 渲染残留，提取可读文本
fn clean_tui_output(raw: &str) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    let mut result_lines: Vec<&str> = Vec::new();

    for line in &lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if is_decoration_line(trimmed) {
            continue;
        }

        if trimmed.contains("Claude Code") && trimmed.contains("Opus") {
            continue;
        }
        if trimmed.contains("API Usage Billing") {
            continue;
        }
        if trimmed.contains("is now available") && trimmed.contains("/model") {
            continue;
        }

        if trimmed.contains("Quick safety check") || trimmed.contains("Accessing workspace") {
            continue;
        }
        if trimmed.contains("Enter to confirm") || trimmed.contains("Esc to cancel") {
            continue;
        }
        if trimmed.contains("I trust this folder") {
            continue;
        }

        if trimmed.contains("stop hook") || trimmed.contains("Stop hook") {
            continue;
        }
        if trimmed.contains("Ran") && trimmed.contains("hooks") {
            continue;
        }

        if is_thinking_indicator(trimmed) {
            continue;
        }

        if trimmed.contains('\u{23F5}') {
            continue;
        }
        if trimmed.contains("bypass permissions") {
            continue;
        }
        if trimmed.contains("shift+tab") {
            continue;
        }
        if trimmed.contains("ctrl+o") {
            continue;
        }

        if trimmed.contains("nexusflow: command not found") {
            continue;
        }
        if trimmed.contains("UserPromptSubmit hook") {
            continue;
        }
        if trimmed.contains("Failed with non-blocking") {
            continue;
        }

        if trimmed.contains("You are the team dispatcher") {
            continue;
        }
        if trimmed.contains("## Team Context") || trimmed.contains("## User Message") {
            continue;
        }
        if trimmed.contains("## Your Decision") || trimmed.contains("## Output Format") {
            continue;
        }
        if trimmed.contains("IMPORTANT: You MUST end your response") {
            continue;
        }
        if trimmed.contains("Available Skills") {
            continue;
        }
        if trimmed.contains("skill:") && trimmed.contains("-") {
            continue;
        }

        result_lines.push(trimmed);
    }

    result_lines.join("\n").trim().to_string()
}

fn is_decoration_line(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    s.chars().all(|c| {
        matches!(
            c,
            '─' | '━'
                | '═'
                | '│'
                | '┃'
                | '║'
                | '┌'
                | '┐'
                | '└'
                | '┘'
                | '├'
                | '┤'
                | '┬'
                | '┴'
                | '┼'
                | '╔'
                | '╗'
                | '╚'
                | '╝'
                | '╠'
                | '╣'
                | '╦'
                | '╩'
                | '╬'
                | '▐'
                | '▌'
                | '▛'
                | '▜'
                | '▝'
                | '▘'
                | '▗'
                | '▖'
                | '⎿'
                | '⏺'
                | ' '
                | '\t'
                | '-'
                | '='
                | '~'
        )
    })
}

fn is_thinking_indicator(s: &str) -> bool {
    let first_char = s.chars().next().unwrap_or(' ');
    if matches!(first_char, '✳' | '✻' | '✽' | '✿' | '❋' | '✵' | '✷') {
        return true;
    }
    if s.contains("Frolicking") || s.contains("Sautéing") || s.contains("Philosophizing") {
        return true;
    }
    false
}

// ─── Progress 检测 ──────────────────────────────────────────────────────────

fn detect_claude_action(output: &str) -> Option<(String, String)> {
    let last_lines: Vec<&str> = output.lines().rev().take(3).collect();

    for line in last_lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(rest) = try_strip_prefix(trimmed, &["Reading:", "Reading file:", "Reading "]) {
            return Some(("reading".into(), rest.trim().into()));
        }
        if let Some(rest) = try_strip_prefix(trimmed, &["Editing:", "Editing file:", "Editing "]) {
            return Some(("editing".into(), rest.trim().into()));
        }
        if let Some(rest) = try_strip_prefix(trimmed, &["Writing:", "Writing file:", "Writing "]) {
            return Some(("writing".into(), rest.trim().into()));
        }
        if let Some(rest) = try_strip_prefix(trimmed, &["Running:", "Running "]) {
            return Some(("running".into(), rest.trim().into()));
        }
        if trimmed.starts_with("$ ") {
            return Some(("running".into(), trimmed[2..].into()));
        }
        if let Some(rest) =
            try_strip_prefix(trimmed, &["Searching:", "Searching for:", "Searching "])
        {
            return Some(("searching".into(), rest.trim().into()));
        }
        if trimmed.contains("Thinking") || trimmed.contains("Let me think") {
            return Some(("thinking".into(), String::new()));
        }
        if let Some(rest) = try_strip_prefix(trimmed, &["Using tool:", "Tool:", "Calling "]) {
            return Some(("tool_use".into(), rest.trim().into()));
        }
    }

    None
}

fn try_strip_prefix<'a>(s: &'a str, prefixes: &[&str]) -> Option<&'a str> {
    let lower = s.to_lowercase();
    for prefix in prefixes {
        let prefix_lower = prefix.to_lowercase();
        if lower.starts_with(&prefix_lower) {
            return Some(&s[prefix.len()..]);
        }
    }
    None
}
