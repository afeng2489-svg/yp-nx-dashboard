//! PTY Task Watcher
//!
//! Subscribes to a PTY session's output after a task is dispatched,
//! detects when claude finishes (prompt `>` with quiet period),
//! extracts the response, and bridges to the AgentExecutionEvent system.

use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use crate::services::claude_terminal::ClaudeTerminalSession;
use crate::services::agent_team_service::strip_ansi;
use crate::ws::agent_execution::AgentExecutionEvent;

/// Maximum execution time (30 minutes, matching run_claude_interactive)
const MAX_EXECUTION_SECS: u64 = 1800;
/// Quiet period threshold before checking for prompt completion
const QUIET_THRESHOLD_SECS: u64 = 5;
/// Minimum execution time before allowing completion detection (avoid false positives)
const MIN_EXECUTION_SECS: u64 = 10;
/// Maximum accumulated output to keep (1 MB clean text)
const MAX_ACCUMULATED_BYTES: usize = 1024 * 1024;
/// Minimum interval between sending Progress events (avoid flooding)
const PROGRESS_DEBOUNCE_MILLIS: u64 = 800;

/// Watch a PTY session after dispatching a task, bridging output to AgentExecutionEvent.
///
/// This is spawned in a `tokio::spawn` and runs until:
/// - Completion is detected (claude prompt `>` after quiet period)
/// - Cancellation via CancellationToken
/// - Timeout exceeded
/// - PTY output channel closed (session died)
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
    let mut last_output_time = Instant::now();
    let mut has_received_output = false;
    let mut last_progress_time = Instant::now() - Duration::from_millis(PROGRESS_DEBOUNCE_MILLIS);
    let exec_id_for_log = execution_id.clone();

    tracing::info!("[PtyWatcher] 开始监听 PTY 输出, execution_id: {}", exec_id_for_log);

    // Thinking heartbeat — every 5 seconds
    let mut heartbeat_interval = tokio::time::interval(Duration::from_secs(5));
    heartbeat_interval.tick().await; // skip first immediate

    // Completion check interval — every 1 second
    let mut check_interval = tokio::time::interval(Duration::from_secs(1));

    loop {
        tokio::select! {
            // PTY output
            output = output_rx.recv() => {
                match output {
                    Ok(raw_bytes) => {
                        let text = String::from_utf8_lossy(&raw_bytes);
                        let clean = strip_ansi(&text);

                        // Debug: log first 200 chars of each output chunk to understand claude CLI format
                        if !clean.trim().is_empty() {
                            let preview: String = clean.chars().take(200).collect();
                            tracing::info!("[PtyWatcher] OUTPUT chunk: {:?}", preview);
                        }

                        // Update tracking
                        last_output_time = Instant::now();
                        has_received_output = true;

                        // Accumulate (with cap)
                        if accumulated.len() + clean.len() < MAX_ACCUMULATED_BYTES {
                            accumulated.push_str(&clean);
                        } else {
                            // Keep the tail for prompt detection
                            let excess = (accumulated.len() + clean.len()) - MAX_ACCUMULATED_BYTES;
                            if excess < accumulated.len() {
                                accumulated = accumulated[excess..].to_string();
                            }
                            accumulated.push_str(&clean);
                        }

                        // Detect structured progress from claude output (before clean is moved)
                        let progress_event = {
                            let now = Instant::now();
                            if now.duration_since(last_progress_time).as_millis() as u64 >= PROGRESS_DEBOUNCE_MILLIS {
                                if let Some((action, detail)) = detect_claude_action(&clean) {
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

                        // Forward as AgentExecutionEvent::Output
                        let _ = event_tx.send(AgentExecutionEvent::Output {
                            execution_id: execution_id.clone(),
                            partial_output: clean,
                        });

                        // Send progress event if detected
                        if let Some(evt) = progress_event {
                            let _ = event_tx.send(evt);
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("[PtyWatcher] 落后 {} 帧, execution_id: {}", n, exec_id_for_log);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::info!("[PtyWatcher] PTY 输出通道关闭, execution_id: {}", exec_id_for_log);
                        let elapsed = start.elapsed().as_millis() as u64;
                        let _ = event_tx.send(AgentExecutionEvent::Completed {
                            execution_id: execution_id.clone(),
                            result: accumulated.clone(),
                            duration_ms: elapsed,
                        });
                        manager.cache_terminal_event(AgentExecutionEvent::Completed {
                            execution_id: execution_id.clone(),
                            result: String::new(),
                            duration_ms: elapsed,
                        });
                        manager.remove_execution(&execution_id);
                        return;
                    }
                }
            }

            // Thinking heartbeat
            _ = heartbeat_interval.tick() => {
                let elapsed = start.elapsed().as_secs();
                let _ = event_tx.send(AgentExecutionEvent::Thinking {
                    execution_id: execution_id.clone(),
                    elapsed_secs: elapsed,
                });
            }

            // Completion check
            _ = check_interval.tick() => {
                if !has_received_output { continue; }
                let elapsed_secs = start.elapsed().as_secs();
                let quiet_secs = last_output_time.elapsed().as_secs();

                if quiet_secs >= QUIET_THRESHOLD_SECS && elapsed_secs >= MIN_EXECUTION_SECS {
                    // Check if last non-empty line is a claude prompt ">"
                    if is_claude_prompt(&accumulated) {
                        tracing::info!(
                            "[PtyWatcher] 检测到完成 (elapsed={}s, quiet={}s), execution_id: {}",
                            elapsed_secs, quiet_secs, exec_id_for_log
                        );
                        let result = extract_response(&accumulated);
                        let duration_ms = start.elapsed().as_millis() as u64;

                        let _ = event_tx.send(AgentExecutionEvent::Completed {
                            execution_id: execution_id.clone(),
                            result,
                            duration_ms,
                        });
                        manager.cache_terminal_event(AgentExecutionEvent::Completed {
                            execution_id: execution_id.clone(),
                            result: String::new(),
                            duration_ms,
                        });
                        manager.remove_execution(&execution_id);
                        return;
                    }
                }

                // Total timeout
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

            // Cancellation
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

/// Check if the last non-empty line of accumulated output is a claude prompt `>`
fn is_claude_prompt(accumulated: &str) -> bool {
    // Find the last non-empty line
    let last_line = accumulated
        .lines()
        .last()
        .map(|l| l.trim())
        .unwrap_or("");

    // Claude CLI prompt is a single ">" possibly followed by whitespace
    // It could also be "> " (with trailing space)
    last_line == ">" || last_line == "> "
}

/// Extract the response text from accumulated PTY output.
/// Strips the echoed input (first line) and the final prompt line.
fn extract_response(accumulated: &str) -> String {
    let lines: Vec<&str> = accumulated.lines().collect();
    if lines.len() <= 2 {
        return accumulated.trim().to_string();
    }

    // Remove last line (prompt ">") and trim
    let response_lines = &lines[..lines.len() - 1];
    let result = response_lines.join("\n").trim().to_string();

    if result.is_empty() {
        accumulated.trim().to_string()
    } else {
        result
    }
}

/// Detect structured actions from claude CLI output.
/// Returns (action, detail) if a recognizable pattern is found.
///
/// Claude CLI output patterns:
/// - "Reading: path/to/file" or "Reading file: ..."
/// - "Editing: path/to/file" or "Editing file: ..."
/// - "Writing: path/to/file" or "Writing file: ..."
/// - "Running: command" or "$ command"
/// - "Searching: ..." or "Searching for: ..."
/// - "Thinking..." or "Let me think"
fn detect_claude_action(output: &str) -> Option<(String, String)> {
    // Only scan the last few lines (most recent output)
    let last_lines: Vec<&str> = output.lines().rev().take(3).collect();

    for line in last_lines {
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }

        // Pattern: "Reading: /path/to/file" or "Reading file: ..."
        if let Some(rest) = try_strip_prefix(trimmed, &["Reading:", "Reading file:", "Reading "]) {
            return Some(("reading".into(), rest.trim().into()));
        }

        // Pattern: "Editing: /path/to/file" or "Editing file: ..."
        if let Some(rest) = try_strip_prefix(trimmed, &["Editing:", "Editing file:", "Editing "]) {
            return Some(("editing".into(), rest.trim().into()));
        }

        // Pattern: "Writing: /path/to/file" or "Writing file: ..."
        if let Some(rest) = try_strip_prefix(trimmed, &["Writing:", "Writing file:", "Writing "]) {
            return Some(("writing".into(), rest.trim().into()));
        }

        // Pattern: "Running: command" or "$ command"
        if let Some(rest) = try_strip_prefix(trimmed, &["Running:", "Running "]) {
            return Some(("running".into(), rest.trim().into()));
        }
        if trimmed.starts_with("$ ") {
            return Some(("running".into(), trimmed[2..].into()));
        }

        // Pattern: "Searching: ..." or "Searching for: ..."
        if let Some(rest) = try_strip_prefix(trimmed, &["Searching:", "Searching for:", "Searching "]) {
            return Some(("searching".into(), rest.trim().into()));
        }

        // Pattern: "Thinking..." or "Let me think"
        if trimmed.contains("Thinking") || trimmed.contains("Let me think") {
            return Some(("thinking".into(), String::new()));
        }

        // Pattern: claude tool use indicators
        // "tool_use" or "Using tool:" patterns
        if let Some(rest) = try_strip_prefix(trimmed, &["Using tool:", "Tool:", "Calling "]) {
            return Some(("tool_use".into(), rest.trim().into()));
        }
    }

    None
}

/// Try to strip one of several prefixes from a string (case-insensitive).
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
