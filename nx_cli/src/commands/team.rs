//! Team 命令 — 团队会话入口

use crate::config::Config;
use crate::tui::{self, TuiEvent};
use nexus_orchestrator::{
    ChainEvent, CliManager, MessageBus, SessionStore, TeamManager, TeamSessionActor,
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
}

/// 运行团队会话
pub async fn run_team(args: TeamArgs, _config: &Config) -> anyhow::Result<()> {
    let task = args.task.join(" ");

    if args.list {
        list_sessions().await?;
        return Ok(());
    }

    if let Some(session_id) = args.resume {
        resume_session(&session_id).await?;
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
            };
            let _ = forward_tx.send(tui_event);
        }
    });

    let team_clone = team.clone();
    let task_clone = task.clone();
    let tui_tx_done = tui_tx.clone();

    // Session 在后台运行，TUI 在主线程渲染（仅当 stdout 是终端时）
    let session_handle = tokio::spawn(async move {
        let result = actor
            .run_chain(&team_clone, &task_clone, Some(chain_tx))
            .await;
        let _ = tui_tx_done.send(TuiEvent::Quit);
        result
    });

    let is_tty = std::io::stdout().is_terminal();
    if is_tty {
        tui::run_tui_skeleton(tui_rx).await?;
    } else {
        // 非终端模式（如 Tauri 启动）：直接等待会话完成
        tracing::info!("stdout 不是终端，以无头模式运行");
        drop(tui_rx);
    }

    let result = session_handle.await??;

    // 持久化保存
    if let Ok(store) = open_store() {
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
        let preview = {
            let text = &agent_result.text;
            if text.len() > 500 {
                // 找 500 字节以内最近的合法 UTF-8 字符边界
                let mut end = 500;
                while end > 0 && !text.is_char_boundary(end) {
                    end -= 1;
                }
                format!("{}...", &text[..end])
            } else {
                text.clone()
            }
        };
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

fn store_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".nx").join("team_sessions.db")
}

fn open_store() -> anyhow::Result<SessionStore> {
    let path = store_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(SessionStore::open(&path)?)
}

async fn list_sessions() -> anyhow::Result<()> {
    let store = open_store()?;
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

async fn resume_session(session_id: &str) -> anyhow::Result<()> {
    let store = open_store()?;
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
        let preview = {
            let text = &r.text;
            if text.len() > 300 {
                let mut end = 300;
                while end > 0 && !text.is_char_boundary(end) {
                    end -= 1;
                }
                format!("{}...", &text[..end])
            } else {
                text.clone()
            }
        };
        println!("{}\n", preview);
    }
    Ok(())
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn store_path_respects_home_env() {
        env::set_var("HOME", "/tmp/test_home");
        let path = store_path();
        assert_eq!(path, PathBuf::from("/tmp/test_home/.nx/team_sessions.db"));
    }

    #[test]
    fn store_path_defaults_to_dot_when_no_home() {
        env::remove_var("HOME");
        let path = store_path();
        assert_eq!(path, PathBuf::from("./.nx/team_sessions.db"));
    }

    #[test]
    fn store_path_contains_nx_hidden_dir() {
        env::set_var("HOME", "/Users/test");
        let path = store_path();
        let path_str = path.to_string_lossy();
        assert!(path_str.contains("/.nx/"));
        assert!(path_str.ends_with("team_sessions.db"));
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
}
