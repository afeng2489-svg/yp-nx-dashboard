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
    Frame, Terminal,
};
use std::io;
use tokio::sync::mpsc;

mod panels;
use panels::{
    render_chat_panel, render_input_hint, render_input_panel, render_progress_panel,
    render_status_panel,
};

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

        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(area);

        render_chat_panel(frame, main_chunks[0], state);

        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(main_chunks[1]);

        render_progress_panel(frame, right_chunks[0], state);
        render_status_panel(frame, right_chunks[1], state);

        let bottom_bar = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(area)[1];
        if state.is_input_mode {
            render_input_panel(frame, bottom_bar, state);
        } else {
            render_input_hint(frame, bottom_bar, state);
        }
    }

    fn handle_input(&mut self, key: KeyCode, modifiers: KeyModifiers) -> InputAction {
        match (key, modifiers) {
            (KeyCode::Char('c'), KeyModifiers::CONTROL)
            | (KeyCode::Char('d'), KeyModifiers::CONTROL)
            | (KeyCode::F(10), _) => InputAction::Quit,
            (KeyCode::Char('q'), _) => InputAction::Quit,
            (KeyCode::Enter, _) => InputAction::EnterInput,
            (KeyCode::Esc, _) => InputAction::CancelInput,
            (KeyCode::Backspace, _) => InputAction::Backspace,
            (KeyCode::Char(c), _) => InputAction::TypeChar(c),
            (KeyCode::Tab, _) => InputAction::TogglePanel,
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
/// 如果提供了 user_input_tx，用户输入的消息会转发到该通道
pub async fn run_tui_skeleton(
    mut events_rx: mpsc::UnboundedReceiver<TuiEvent>,
    user_input_tx: Option<mpsc::UnboundedSender<String>>,
) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut renderer = SkeletonRenderer;
    let state = SessionViewState::default();

    let result = run_tui_loop(
        &mut terminal,
        &mut renderer,
        state,
        &mut events_rx,
        user_input_tx,
    );

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

#[cfg(test)]
mod tests;

fn run_tui_loop<R: TeamSessionRenderer>(
    terminal: &mut Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>,
    renderer: &mut R,
    mut state: SessionViewState,
    events_rx: &mut mpsc::UnboundedReceiver<TuiEvent>,
    user_input_tx: Option<mpsc::UnboundedSender<String>>,
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
                    InputAction::Quit => {
                        // Ctrl+C in input mode: cancel input instead of quitting
                        if state.is_input_mode {
                            state.is_input_mode = false;
                            state.input_buffer.clear();
                        } else {
                            return Ok(());
                        }
                    }
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
                            // 如果有用户输入转发通道，发送消息
                            if let Some(ref tx) = user_input_tx {
                                let _ = tx.send(msg.clone());
                            }
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
