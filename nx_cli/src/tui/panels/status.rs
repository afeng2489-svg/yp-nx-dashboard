use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::super::{SessionStatus, SessionViewState};

pub fn render_status_panel(frame: &mut Frame, area: Rect, state: &SessionViewState) {
    let (status_text, status_color) = match state.status {
        SessionStatus::Init => ("初始化", Color::Cyan),
        SessionStatus::Running => ("运行中", Color::Green),
        SessionStatus::WaitingForUser => ("等待输入", Color::Yellow),
        SessionStatus::Completed => ("已完成", Color::Green),
        SessionStatus::Failed => ("失败", Color::Red),
    };
    let block = Block::default()
        .title("状态")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Magenta));
    let msg_count = state.messages.len();
    let agent_count = state.agent_states.len();
    let text = vec![
        Line::styled(
            format!("会话: {}", status_text),
            Style::default().fg(status_color),
        ),
        Line::from(""),
        Line::from(format!("消息: {}", msg_count)),
        Line::from(format!("角色: {}", agent_count)),
        Line::from(""),
        Line::from("[Ctrl+C] 退出  [Tab] 切换面板"),
        Line::from("[Enter] 输入消息  [Esc] 取消"),
    ];
    frame.render_widget(Paragraph::new(Text::from(text)).block(block), area);
}
