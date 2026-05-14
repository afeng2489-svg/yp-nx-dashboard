//! Team 命令 — 团队会话入口

use crate::config::Config;
use crate::tui::{self, TuiEvent};
use nexus_orchestrator::{
    resolve_team_db_path, ChainEvent, CliManager, MessageBus, SessionStore, TeamManager,
    TeamSessionActor, UserMessage,
};
use std::io::IsTerminal;
use std::path::PathBuf;
use tokio::sync::mpsc;

/// Team 命令参数
pub struct TeamArgs {
    pub task: Vec<String>,
    pub roles: Option<String>,
    pub resume: Option<String>,
    pub list: bool,
    pub model: Option<String>,
    pub project: Option<String>,
}

/// 运行团队会话
pub async fn run_team(args: TeamArgs, _config: &Config) -> anyhow::Result<()> {
    let task = args.task.join(" ");

    if args.list {
        list_sessions(args.project.as_deref()).await?;
        return Ok(());
    }

    if let Some(session_id) = args.resume {
        resume_session(&session_id, args.project.as_deref()).await?;
        return Ok(());
    }

    if task.is_empty() {
        anyhow::bail!("请提供任务描述，例如: nx team \"write a hello world rust program\"");
    }

    let roles: Vec<String> = args
        .roles
        .as_deref()
        .map(|r| r.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    tracing::info!(
        "启动团队会话: task=\"{}\", roles={:?}, model={:?}",
        task,
        roles,
        args.model
    );

    println!("🤖 启动 Team 会话");
    println!("任务: {}\n", task);

    // 构建基础设施
    let message_bus = std::sync::Arc::new(MessageBus::new());
    let team_manager = TeamManager::new(message_bus.clone());
    let cli_manager = std::sync::Arc::new(CliManager::new());

    // 创建团队
    let team_id = team_manager.create_dev_team("NexusFlow Team");
    let team = team_manager
        .get_team(team_id)
        .ok_or_else(|| anyhow::anyhow!("创建团队失败"))?;

    // 如果有自定义角色，重新创建定制团队
    let team = if !roles.is_empty() {
        build_custom_team(&team_manager, &roles)?
    } else {
        team
    };

    println!("团队成员:");
    for member in team.members.values() {
        let model = TeamSessionActor::role_to_model(member.role);
        println!("  - {:?} ({}) — {}", member.role, member.name, model);
    }
    println!();

    // 创建会话执行器
    let actor = TeamSessionActor::new(cli_manager, message_bus);

    // 创建事件通道：ChainEvent → TuiEvent
    let (tui_tx, tui_rx) = mpsc::unbounded_channel::<TuiEvent>();
    let (chain_tx, mut chain_rx) = mpsc::unbounded_channel::<ChainEvent>();

    // 转发任务：ChainEvent → TuiEvent
    let forward_tx = tui_tx.clone();
    tokio::spawn(async move {
        while let Some(event) = chain_rx.recv().await {
            let tui_event = match event {
                ChainEvent::AgentStarted { role, name, .. } => TuiEvent::AgentStarted {
                    agent_id: name.clone(),
                    role: format!("{:?}", role),
                },
                ChainEvent::AgentCompleted {
                    role,
                    name,
                    summary,
                    ..
                } => {
                    let _ = forward_tx.send(TuiEvent::AgentOutput {
                        agent_id: name.clone(),
                        role: format!("{:?}", role),
                        chunk: summary,
                    });
                    TuiEvent::AgentCompleted {
                        agent_id: name,
                        role: format!("{:?}", role),
                    }
                }
                ChainEvent::ChainCompleted { .. } => {
                    TuiEvent::StatusChanged(crate::tui::SessionStatus::Completed)
                }
                ChainEvent::ChainFailed { error } => {
                    let _ = forward_tx.send(TuiEvent::AgentOutput {
                        agent_id: "system".into(),
                        role: "error".into(),
                        chunk: error,
                    });
                    TuiEvent::StatusChanged(crate::tui::SessionStatus::Failed)
                }
                ChainEvent::AgentWaitingForInput { role, name, .. } => {
                    let _ = forward_tx.send(TuiEvent::StatusChanged(
                        crate::tui::SessionStatus::WaitingForUser,
                    ));
                    TuiEvent::AgentStarted {
                        agent_id: name,
                        role: format!("{:?}", role),
                    }
                }
                ChainEvent::AgentReceivedInput { message, .. } => TuiEvent::AgentOutput {
                    agent_id: "user".into(),
                    role: "input".into(),
                    chunk: message,
                },
            };
            let _ = forward_tx.send(tui_event);
        }
    });

    // 创建用户消息通道：TUI → TeamSessionActor
    let (user_tx, user_rx) = mpsc::unbounded_channel::<UserMessage>();
    // TUI 发送 String，需要桥接到 UserMessage
    let (tui_input_tx, mut tui_input_rx) = mpsc::unbounded_channel::<String>();

    // 桥接任务：TUI String → UserMessage → session actor
    tokio::spawn(async move {
        while let Some(input) = tui_input_rx.recv().await {
            let msg = UserMessage::parse(&input);
            let _ = user_tx.send(msg);
        }
    });

    let team_clone = team.clone();
    let task_clone = task.clone();
    let tui_tx_done = tui_tx.clone();

    // Session 在后台运行，TUI 在主线程渲染（仅当 stdout 是终端时）
    let session_handle = tokio::spawn(async move {
        let result = actor
            .run_chain(&team_clone, &task_clone, Some(chain_tx), Some(user_rx))
            .await;
        let _ = tui_tx_done.send(TuiEvent::Quit);
        result
    });

    let is_tty = std::io::stdout().is_terminal();
    if is_tty {
        tui::run_tui_skeleton(tui_rx, Some(tui_input_tx)).await?;
    } else {
        // 非终端模式（如 Tauri 启动）：直接等待会话完成
        tracing::info!("stdout 不是终端，以无头模式运行");
        drop(tui_rx);
    }

    let result = session_handle.await??;

    // 持久化保存
    if let Ok(store) = open_store(args.project.as_deref()) {
        if let Err(e) = store.save(&result) {
            tracing::warn!("保存会话失败: {}", e);
        }
    }

    println!(
        "\n✅ 团队会话完成 ({:.1}s)",
        result.total_duration_ms as f64 / 1000.0
    );
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    for agent_result in &result.agent_results {
        let status = if agent_result.attempts > 1 {
            format!(" ({} 次尝试)", agent_result.attempts)
        } else {
            String::new()
        };
        println!(
            "{} [{:?}] — {:.1}s{}",
            agent_result.agent_name,
            agent_result.role,
            agent_result.duration_ms as f64 / 1000.0,
            status
        );
        let preview = preview_text(&agent_result.text, 500);
        println!("{}\n", preview);
    }
    Ok(())
}

fn build_custom_team(
    manager: &TeamManager,
    roles: &[String],
) -> anyhow::Result<nexus_orchestrator::Team> {
    if roles.is_empty() {
        anyhow::bail!("团队至少需要一个角色");
    }
    let team_id = manager.create_team("NexusFlow Custom Team");

    use nexus_orchestrator::{AgentRole, CliProvider, TeamMember};
    let mut members = Vec::new();
    for role_name in roles {
        let role = AgentRole::from_str(role_name);
        let member = TeamMember::new(role, role_name, CliProvider::Claude);
        members.push(member);
    }

    let mut ids = Vec::new();
    for member in &members {
        let id = member.id;
        ids.push(id);
        manager.add_member(team_id, member.clone())?;
    }

    // 串行链：每个角色是下一个的前驱
    for window in ids.windows(2) {
        manager.add_dependency(team_id, window[0], window[1])?;
    }

    if let Some(&first) = ids.first() {
        manager.set_leader(team_id, first)?;
    }

    manager
        .get_team(team_id)
        .ok_or_else(|| anyhow::anyhow!("创建自定义团队失败"))
}

/// 截断文本到指定长度，保留合法 UTF-8 字符边界
fn preview_text(text: &str, max_len: usize) -> String {
    if text.len() > max_len {
        let mut end = max_len;
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}...", &text[..end])
    } else {
        text.to_string()
    }
}

fn store_path(project: Option<&str>) -> PathBuf {
    match project {
        Some(p) => {
            let path = PathBuf::from(p).join(".nx").join("team_sessions.db");
            let _ = std::fs::create_dir_all(path.parent().unwrap());
            path
        }
        None => resolve_team_db_path(),
    }
}

fn open_store(project: Option<&str>) -> anyhow::Result<SessionStore> {
    let path = store_path(project);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(SessionStore::open(&path)?)
}

async fn list_sessions(project: Option<&str>) -> anyhow::Result<()> {
    let store = open_store(project)?;
    let sessions = store.list(20)?;

    println!("📋 历史团队会话");
    println!("===============\n");

    if sessions.is_empty() {
        println!("暂无历史会话。使用 `nx team \"<任务>\"` 启动一个吧。");
        return Ok(());
    }

    for s in &sessions {
        let duration = if s.duration_ms >= 1000 {
            format!("{:.1}s", s.duration_ms as f64 / 1000.0)
        } else {
            format!("{}ms", s.duration_ms)
        };
        println!(
            "{} | {} agents | {} | {} | {}",
            s.created_at, s.agent_count, duration, s.status, s.task
        );
        println!("  id: {}\n", s.execution_id);
    }
    Ok(())
}

async fn resume_session(session_id: &str, project: Option<&str>) -> anyhow::Result<()> {
    let store = open_store(project)?;
    let result = store
        .get(session_id)?
        .ok_or_else(|| anyhow::anyhow!("会话不存在: {}", session_id))?;

    println!("▶️  会话详情: {}\n", session_id);
    println!("任务: {}", result.task);
    println!("总耗时: {:.1}s\n", result.total_duration_ms as f64 / 1000.0);

    for r in &result.agent_results {
        println!(
            "{} [{:?}] — {:.1}s ({} 次尝试)",
            r.agent_name,
            r.role,
            r.duration_ms as f64 / 1000.0,
            r.attempts
        );
        let preview = preview_text(&r.text, 300);
        println!("{}\n", preview);
    }
    Ok(())
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_path_returns_absolute_path() {
        let path = store_path(None);
        assert!(path.is_absolute());
        // 路径应以 .nx/team_sessions.db 结尾
        assert!(path.ends_with(".nx/team_sessions.db"));
    }

    #[test]
    fn store_path_is_absolute() {
        let path = store_path(None);
        assert!(path.is_absolute());
    }

    #[test]
    fn store_path_has_correct_components() {
        let path = store_path(None);
        assert_eq!(path.file_name().unwrap(), "team_sessions.db");
        assert!(path.parent().unwrap().ends_with(".nx"));
    }

    #[test]
    fn store_path_is_deterministic() {
        // 重复调用应返回相同路径（不依赖外部状态）
        let a = store_path(None);
        let b = store_path(None);
        assert_eq!(a, b);
        assert!(a.is_absolute());
    }

    #[test]
    fn store_path_with_explicit_project() {
        let path = store_path(Some("/tmp/test-project"));
        assert!(path.is_absolute());
        assert_eq!(
            path,
            PathBuf::from("/tmp/test-project/.nx/team_sessions.db")
        );
    }

    #[test]
    fn build_custom_team_creates_correct_roles() {
        let message_bus = std::sync::Arc::new(nexus_orchestrator::MessageBus::new());
        let manager = TeamManager::new(message_bus);
        let roles = vec!["architect".into(), "developer".into(), "tester".into()];

        let team = build_custom_team(&manager, &roles).unwrap();
        assert_eq!(team.members.len(), 3);

        let member_names: Vec<&str> = team.members.values().map(|m| m.name.as_str()).collect();
        assert!(member_names.contains(&"architect"));
        assert!(member_names.contains(&"developer"));
        assert!(member_names.contains(&"tester"));
    }

    #[test]
    fn build_custom_team_sets_leader() {
        let message_bus = std::sync::Arc::new(nexus_orchestrator::MessageBus::new());
        let manager = TeamManager::new(message_bus);
        let roles = vec!["architect".into()];

        let team = build_custom_team(&manager, &roles).unwrap();
        // Leader is stored in hierarchy via set_leader()
        let leader_id = team
            .members
            .keys()
            .find(|id| team.hierarchy.contains_key(id));
        assert!(
            leader_id.is_some(),
            "Team should have a leader in hierarchy"
        );
        let leader = leader_id.unwrap();
        assert!(team.members.contains_key(leader));
    }

    #[test]
    fn build_custom_team_empty_roles_returns_error() {
        let message_bus = std::sync::Arc::new(nexus_orchestrator::MessageBus::new());
        let manager = TeamManager::new(message_bus);
        let roles: Vec<String> = vec![];

        let result = build_custom_team(&manager, &roles);
        assert!(result.is_err());
    }

    #[test]
    fn build_custom_team_maintains_chain_order() {
        let message_bus = std::sync::Arc::new(nexus_orchestrator::MessageBus::new());
        let manager = TeamManager::new(message_bus);
        let roles = vec![
            "architect".into(),
            "developer".into(),
            "reviewer".into(),
            "tester".into(),
        ];

        let team = build_custom_team(&manager, &roles).unwrap();
        // Leader is the first role (architect) - stored in hierarchy
        let leader_id = team
            .members
            .iter()
            .find(|(_, m)| m.name == "architect")
            .map(|(id, _)| *id)
            .expect("architect should exist");
        assert!(team.hierarchy.contains_key(&leader_id));
        assert_eq!(team.members[&leader_id].name, "architect");
    }

    #[test]
    fn build_custom_team_single_role_works() {
        let message_bus = std::sync::Arc::new(nexus_orchestrator::MessageBus::new());
        let manager = TeamManager::new(message_bus);
        let roles = vec!["tester".into()];

        let team = build_custom_team(&manager, &roles).unwrap();
        assert_eq!(team.members.len(), 1);
        let member = team.members.values().next().unwrap();
        assert_eq!(member.name, "tester");
    }

    #[test]
    fn build_custom_team_many_roles() {
        let message_bus = std::sync::Arc::new(nexus_orchestrator::MessageBus::new());
        let manager = TeamManager::new(message_bus);
        let roles = vec![
            "architect".into(),
            "developer".into(),
            "reviewer".into(),
            "tester".into(),
            "devops".into(),
            "pm".into(),
        ];

        let team = build_custom_team(&manager, &roles).unwrap();
        assert_eq!(team.members.len(), 6);
        // All member names present
        let names: Vec<&str> = team.members.values().map(|m| m.name.as_str()).collect();
        for role in &[
            "architect",
            "developer",
            "reviewer",
            "tester",
            "devops",
            "pm",
        ] {
            assert!(names.contains(role), "missing role: {}", role);
        }
    }

    #[test]
    fn build_custom_team_member_names_are_roles() {
        let message_bus = std::sync::Arc::new(nexus_orchestrator::MessageBus::new());
        let manager = TeamManager::new(message_bus);
        let roles = vec!["alpha".into(), "beta".into(), "gamma".into()];

        let team = build_custom_team(&manager, &roles).unwrap();
        let names: Vec<&str> = team.members.values().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"alpha"));
        assert!(names.contains(&"beta"));
        assert!(names.contains(&"gamma"));
    }

    // ── open_store ────────────────────────────────────────────────────

    /// 在临时隔离目录中运行 open_store 测试，避免并行测试竞争
    fn with_isolated_store<F>(f: F)
    where
        F: FnOnce(),
    {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "nx_store_test_{}",
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();
        f();
        let _ = std::env::set_current_dir(&original);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn open_store_succeeds() {
        with_isolated_store(|| {
            let result = open_store(None);
            assert!(result.is_ok());
        });
    }

    #[test]
    fn open_store_idempotent() {
        with_isolated_store(|| {
            assert!(open_store(None).is_ok());
            assert!(open_store(None).is_ok());
        });
    }

    // ── preview_text ──────────────────────────────────────────────────

    #[test]
    fn preview_text_short_text_no_truncation() {
        let result = preview_text("hello", 500);
        assert_eq!(result, "hello");
    }

    #[test]
    fn preview_text_exact_length_no_truncation() {
        let result = preview_text("hello", 5);
        assert_eq!(result, "hello");
    }

    #[test]
    fn preview_text_truncates_with_suffix() {
        let text = "a".repeat(100);
        let result = preview_text(&text, 10);
        assert_eq!(result.len(), 13); // "a...x10" + "..."
        assert!(result.ends_with("..."));
    }

    #[test]
    fn preview_text_ut8_boundary_respected() {
        // 每个中文字符 3 字节。"你好世界" = 12 bytes
        // max_len=7 → end=7 不在边界，回退到 end=6（"好"的结束）
        let text = "你好世界"; // 12 bytes
        let result = preview_text(text, 7);
        assert_eq!(result, "你好...");
    }

    #[test]
    fn preview_text_ut8_exact_boundary() {
        // 每个中文字符 3 字节。max_len=6 正好在"好"的结束边界
        let text = "你好世界";
        let result = preview_text(text, 6);
        assert_eq!(result, "你好...");
    }

    #[test]
    fn preview_text_empty() {
        let result = preview_text("", 10);
        assert_eq!(result, "");
    }

    #[test]
    fn preview_text_max_len_zero() {
        let result = preview_text("hello", 0);
        assert_eq!(result, "...");
    }

    #[test]
    fn preview_text_ascii_exact_truncation() {
        let text = "hello world";
        let result = preview_text(text, 5);
        assert_eq!(result, "hello...");
    }

    #[test]
    fn preview_text_emoji_boundary() {
        // "👋" 是 4 字节。max_len=4 正好在 emoji 边界上
        let text = "👋hello";
        let result = preview_text(text, 4);
        assert_eq!(result, "👋...");
    }

    #[test]
    fn preview_text_mixed_encoding_boundary() {
        // "a👋b" = 1 + 4 + 1 = 6 bytes. max_len=5 不在边界，回退到 end=4（emoji后面）
        let text = "a👋b";
        let result = preview_text(text, 5);
        assert_eq!(result, "a👋...");
    }

    #[test]
    fn preview_text_different_max_lengths() {
        let text = "a".repeat(1000);
        let len100 = preview_text(&text, 100);
        let len200 = preview_text(&text, 200);
        assert!(len100.len() < len200.len());
        assert!(len100.ends_with("..."));
        assert!(len200.ends_with("..."));
    }

    // ── build_custom_team edge cases ───────────────────────────────────

    #[test]
    fn build_custom_team_duplicate_roles() {
        let message_bus = std::sync::Arc::new(nexus_orchestrator::MessageBus::new());
        let manager = TeamManager::new(message_bus);
        let roles = vec!["architect".into(), "architect".into(), "architect".into()];

        let team = build_custom_team(&manager, &roles).unwrap();
        // Duplicate roles are allowed — each creates a separate member
        assert_eq!(team.members.len(), 3);
    }

    #[test]
    fn build_custom_team_role_with_special_characters() {
        let message_bus = std::sync::Arc::new(nexus_orchestrator::MessageBus::new());
        let manager = TeamManager::new(message_bus);
        let roles = vec![
            "role-123".into(),
            "role_name".into(),
            "ROLE".into(),
            "123".into(),
        ];

        let team = build_custom_team(&manager, &roles).unwrap();
        assert_eq!(team.members.len(), 4);
        // Names are preserved verbatim
        let names: Vec<&str> = team.members.values().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"role-123"));
        assert!(names.contains(&"role_name"));
        assert!(names.contains(&"ROLE"));
        assert!(names.contains(&"123"));
    }

    // ── run_team error paths ───────────────────────────────────────────

    #[tokio::test]
    async fn run_team_empty_task_returns_error() {
        let result = run_team(
            TeamArgs {
                task: vec![],
                roles: None,
                resume: None,
                list: false,
                model: None,
                project: None,
            },
            &Config::default(),
        )
        .await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("任务描述"));
    }

    #[tokio::test]
    async fn run_team_empty_task_syntax_in_error() {
        // 验证空任务给出的错误含指导信息
        let result = run_team(
            TeamArgs {
                task: vec![],
                roles: None,
                resume: None,
                list: false,
                model: None,
                project: None,
            },
            &Config::default(),
        )
        .await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("nx team"),
            "error should mention usage: {}",
            err
        );
    }

    #[tokio::test]
    async fn run_team_list_creates_store_and_shows_empty() {
        // list_sessions 会自动创建 .nx 目录并显示"暂无历史会话"
        let result = run_team(
            TeamArgs {
                task: vec![],
                roles: None,
                resume: None,
                list: true,
                model: None,
                project: None,
            },
            &Config::default(),
        )
        .await;
        // Should succeed because list handles the case gracefully
        assert!(result.is_ok());
    }

    // ── preview_text additional edge cases ──────────────────────────────

    #[test]
    fn preview_text_cjk_mixed_with_ascii() {
        // "a你b好c" = 1 + 3 + 1 + 3 + 1 = 9 bytes. max_len=7
        // byte 7 在 '好' 中间 → 回退到 byte 5 ('好' 起始)
        let text = "a你b好c";
        let result = preview_text(text, 7);
        assert_eq!(result, "a你b...");
    }

    #[test]
    fn preview_text_exact_emoji_boundary() {
        // "👍" = 4 bytes. max_len 正好落在中间 → 必须回退到 0
        let text = "👍hi";
        let result = preview_text(text, 2);
        assert_eq!(result, "...");
    }

    #[test]
    fn preview_text_max_len_one() {
        // max_len=1, byte 1 is ascii boundary → keeps 1 char
        let text = "hello";
        let result = preview_text(text, 1);
        assert_eq!(result, "h...");
    }

    #[test]
    fn preview_text_max_len_larger_than_text() {
        let text = "short";
        let result = preview_text(text, 100);
        assert_eq!(result, "short");
    }

    #[test]
    fn preview_text_unicode_only_boundaries() {
        // 全是 3 字节中文字符：max_len 在 1-2 回退到 0，3 正好一个字符
        let text = "你好世界"; // 12 bytes
        assert_eq!(preview_text(text, 1), "...");
        assert_eq!(preview_text(text, 2), "...");
        assert_eq!(preview_text(text, 3), "你...");
        assert_eq!(preview_text(text, 4), "你..."); // 4 < 6, back to 3
        assert_eq!(preview_text(text, 5), "你...");
        assert_eq!(preview_text(text, 6), "你好...");
    }

    #[test]
    fn preview_text_multibyte_char_split_emoji() {
        // "👨‍👩‍👧‍👦" 是一个多字节 emoji 序列. max_len=4 回退到 0
        let text = "👨‍👩‍👧‍👦family"; // complex emoji
        let result = preview_text(text, 4);
        // Should not panic, should safe-truncate
        assert!(result.ends_with("..."));
    }

    // ── build_custom_team edge cases ───────────────────────────────────

    #[test]
    fn build_custom_team_large_number_of_roles() {
        let message_bus = std::sync::Arc::new(nexus_orchestrator::MessageBus::new());
        let manager = TeamManager::new(message_bus);
        let roles: Vec<String> = (0..50).map(|i| format!("role-{}", i)).collect();

        let team = build_custom_team(&manager, &roles).unwrap();
        assert_eq!(team.members.len(), 50);
        // Chain should maintain order
        let leader_id = team
            .members
            .values()
            .find(|m| m.name == "role-0")
            .unwrap()
            .id;
        assert!(team.hierarchy.contains_key(&leader_id));
    }

    // ── run_team error paths ───────────────────────────────────────────

    #[tokio::test]
    async fn run_team_resume_nonexistent_session() {
        let result = run_team(
            TeamArgs {
                task: vec![],
                roles: None,
                resume: Some("00000000-0000-0000-0000-000000000000".into()),
                list: false,
                model: None,
                project: None,
            },
            &Config::default(),
        )
        .await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        // Should mention that the session doesn't exist
        assert!(err.contains("不存在") || err.contains("exist") || err.contains("not found"));
    }

    #[tokio::test]
    async fn run_team_resume_with_empty_id() {
        let result = run_team(
            TeamArgs {
                task: vec![],
                roles: None,
                resume: Some(String::new()),
                list: false,
                model: None,
                project: None,
            },
            &Config::default(),
        )
        .await;
        assert!(result.is_err());
    }

    // ── store_path integration ─────────────────────────────────────────

    #[test]
    fn store_path_ends_with_correct_suffix() {
        let path = store_path(None);
        assert!(path.is_absolute());
        assert!(path.ends_with(".nx/team_sessions.db"));
    }

    #[test]
    fn store_path_parent_is_nx_dir() {
        let path = store_path(None);
        assert!(path.parent().unwrap().ends_with(".nx"));
    }
}
