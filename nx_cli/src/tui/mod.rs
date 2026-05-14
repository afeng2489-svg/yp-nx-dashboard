//! TUI 渲染层 — 团队会话终端界面
//!
//! 基于 ratatui + crossterm，提供三面板实时渲染。

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use std::io;
use tokio::sync::mpsc;

/// 团队会话渲染器 trait — Phase 3 将实现完整渲染逻辑
pub trait TeamSessionRenderer: Send {
    fn render(&mut self, frame: &mut Frame, state: &SessionViewState);
    fn handle_input(&mut self, key: KeyCode, modifiers: KeyModifiers) -> InputAction;
}

/// 用户输入动作
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputAction {
    Quit,
    TogglePanel,
    EnterInput,
    CancelInput,
    SendMessage(String),
    TypeChar(char),
    Backspace,
    None,
}

/// 会话视图状态
#[derive(Debug, Clone, Default)]
pub struct SessionViewState {
    pub status: SessionStatus,
    /// (agent_name, role, chunk)
    pub messages: Vec<(String, String, String)>,
    /// (role, status, elapsed_secs)
    pub agent_states: Vec<(String, AgentProgress, u64)>,
    pub selected_panel: PanelFocus,
    pub input_buffer: String,
    pub is_input_mode: bool,
    /// 打字机效果：最新消息已显示的字符数
    pub typewriter_pos: usize,
}

/// 每 tick 显示的新字符数（100ms tick → ~30 chars/sec）
const TYPING_SPEED: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SessionStatus {
    #[default]
    Init,
    Running,
    WaitingForUser,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentProgress {
    Idle,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PanelFocus {
    #[default]
    Chat,
    Progress,
    Input,
}

/// 空白 TUI 渲染器 — Phase 1 骨架
pub struct SkeletonRenderer;

impl TeamSessionRenderer for SkeletonRenderer {
    fn render(&mut self, frame: &mut Frame, state: &SessionViewState) {
        let area = frame.area();

        // 主布局：左侧对话区 + 右侧进度栏
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(area);

        // 对话面板
        let chat_block = Block::default()
            .title("对话区")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan));
        let chat_text = if state.messages.is_empty() {
            Text::from("等待团队会话启动...\n\n使用 `nx team \"<任务描述>\"` 启动会话")
        } else {
            let last_idx = state.messages.len().saturating_sub(1);
            let lines: Vec<Line> = state
                .messages
                .iter()
                .enumerate()
                .map(|(i, (name, _role, chunk))| {
                    let display_text = if i == last_idx {
                        let end = state.typewriter_pos.min(chunk.len());
                        &chunk[..end]
                    } else {
                        chunk.as_str()
                    };
                    Line::from(vec![
                        Span::styled(format!("[{}] ", name), Style::default().fg(Color::Yellow)),
                        Span::raw(display_text),
                    ])
                })
                .collect();
            Text::from(lines)
        };
        let chat = Paragraph::new(chat_text).block(chat_block);
        frame.render_widget(chat, main_chunks[0]);

        // 右侧面板（进度 + 状态）
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(main_chunks[1]);

        // 进度面板
        let progress_block = Block::default()
            .title("进度")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Green));
        let progress_lines: Vec<Line> = if state.agent_states.is_empty() {
            vec![
                Line::from("○ Architect  待机"),
                Line::from("○ Developer  待机"),
                Line::from("○ Reviewer   待机"),
                Line::from("○ Tester     待机"),
            ]
        } else {
            state
                .agent_states
                .iter()
                .map(|(role, status, elapsed)| {
                    let icon = match status {
                        AgentProgress::Idle => "○",
                        AgentProgress::Running => "◉",
                        AgentProgress::Completed => "✓",
                        AgentProgress::Failed => "✗",
                    };
                    Line::from(format!("{} {:12} {}s", icon, role, elapsed))
                })
                .collect()
        };
        let progress = Paragraph::new(Text::from(progress_lines)).block(progress_block);
        frame.render_widget(progress, right_chunks[0]);

        // 状态栏
        let status_block = Block::default()
            .title("状态")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Magenta));
        let status_text = vec![
            Line::from(format!("状态: {:?}", state.status)),
            Line::from(""),
            Line::from("[Ctrl+C] 退出"),
            Line::from("[Tab] 切换面板"),
            Line::from("[Enter] 输入消息 / [Esc] 取消"),
        ];
        let status = Paragraph::new(Text::from(status_text)).block(status_block);
        frame.render_widget(status, right_chunks[1]);

        // 输入栏（底部弹出）
        if state.is_input_mode {
            let input_area = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(3)])
                .split(area)[1];
            let input_block = Block::default()
                .title("💬 输入 (Enter 发送, Esc 取消)")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Yellow));
            let cursor = "▊";
            let input_text = format!("> {}{}", state.input_buffer, cursor);
            let input = Paragraph::new(Text::from(input_text)).block(input_block);
            frame.render_widget(input, input_area);
        }
    }

    fn handle_input(&mut self, key: KeyCode, modifiers: KeyModifiers) -> InputAction {
        match key {
            KeyCode::Char('c') if modifiers == KeyModifiers::CONTROL => InputAction::Quit,
            KeyCode::Char('q') => InputAction::Quit,
            KeyCode::Enter => InputAction::EnterInput,
            KeyCode::Esc => InputAction::CancelInput,
            KeyCode::Backspace => InputAction::Backspace,
            KeyCode::Char(c) => InputAction::TypeChar(c),
            KeyCode::Tab => InputAction::TogglePanel,
            _ => InputAction::None,
        }
    }
}

/// TUI 事件 — 从后台线程发送到渲染线程
#[derive(Debug, Clone)]
pub enum TuiEvent {
    AgentOutput {
        agent_id: String,
        role: String,
        chunk: String,
    },
    AgentStarted {
        agent_id: String,
        role: String,
    },
    AgentCompleted {
        agent_id: String,
        role: String,
    },
    StatusChanged(SessionStatus),
    Quit,
}

/// 启动 TUI 界面，通过 events_rx 接收后台会话事件
pub async fn run_tui_skeleton(mut events_rx: mpsc::UnboundedReceiver<TuiEvent>) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut renderer = SkeletonRenderer;
    let state = SessionViewState::default();

    let result = run_tui_loop(&mut terminal, &mut renderer, state, &mut events_rx);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_tui_loop<R: TeamSessionRenderer>(
    terminal: &mut Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>,
    renderer: &mut R,
    mut state: SessionViewState,
    events_rx: &mut mpsc::UnboundedReceiver<TuiEvent>,
) -> Result<()> {
    loop {
        // 消费所有待处理事件
        while let Ok(event) = events_rx.try_recv() {
            match event {
                TuiEvent::AgentStarted { role, .. } => {
                    state.status = SessionStatus::Running;
                    if let Some(entry) = state.agent_states.iter_mut().find(|(r, _, _)| r == &role)
                    {
                        *entry = (role.clone(), AgentProgress::Running, 0);
                    } else {
                        state
                            .agent_states
                            .push((role.clone(), AgentProgress::Running, 0));
                    }
                }
                TuiEvent::AgentCompleted { role, .. } => {
                    if let Some(entry) = state.agent_states.iter_mut().find(|(r, _, _)| r == &role)
                    {
                        *entry = (role.clone(), AgentProgress::Completed, entry.2);
                    }
                }
                TuiEvent::AgentOutput {
                    agent_id,
                    role,
                    chunk,
                } => {
                    state.messages.push((agent_id, role, chunk));
                    // 新消息到达时重置打字机位置，开始逐字显示
                    state.typewriter_pos = 0;
                }
                TuiEvent::StatusChanged(new_status) => {
                    state.status = new_status;
                }
                TuiEvent::Quit => return Ok(()),
            }
        }

        // 更新运行中的 agent 已用时间（tick 计数）
        for entry in &mut state.agent_states {
            if entry.1 == AgentProgress::Running {
                entry.2 += 1;
            }
        }

        // 打字机效果：逐字推进最新消息的显示位置
        if let Some((_, _, text)) = state.messages.last() {
            if state.typewriter_pos < text.len() {
                state.typewriter_pos += TYPING_SPEED;
            }
        }

        terminal.draw(|frame| {
            renderer.render(frame, &state);
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match renderer.handle_input(key.code, key.modifiers) {
                    InputAction::Quit => return Ok(()),
                    InputAction::TogglePanel => {
                        state.selected_panel = match state.selected_panel {
                            PanelFocus::Chat => PanelFocus::Progress,
                            PanelFocus::Progress => PanelFocus::Input,
                            PanelFocus::Input => PanelFocus::Chat,
                        };
                    }
                    InputAction::EnterInput => {
                        if state.is_input_mode {
                            let msg = state.input_buffer.clone();
                            state.is_input_mode = false;
                            state.input_buffer.clear();
                            // Push user message into the chat
                            state.messages.push(("user".into(), "@self".into(), msg));
                        } else {
                            state.is_input_mode = true;
                        }
                    }
                    InputAction::CancelInput => {
                        state.is_input_mode = false;
                        state.input_buffer.clear();
                    }
                    InputAction::TypeChar(c) => {
                        if state.is_input_mode {
                            state.input_buffer.push(c);
                        }
                    }
                    InputAction::Backspace => {
                        if state.is_input_mode {
                            state.input_buffer.pop();
                        }
                    }
                    InputAction::SendMessage(_msg) => {}
                    InputAction::None => {}
                }
            }
        }
    }
}
