use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::super::SessionViewState;

pub fn render_input_panel(frame: &mut Frame, area: Rect, state: &SessionViewState) {
    if !state.is_input_mode {
        return;
    }
    let block = Block::default()
        .title("输入 (Enter 发送, Esc 取消)")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Yellow));
    let cursor = if state.input_buffer.is_empty() {
        "▊"
    } else {
        ""
    };
    let display = format!("> {}{}", state.input_buffer, cursor);
    frame.render_widget(Paragraph::new(Text::from(display)).block(block), area);
}

pub fn render_input_hint(frame: &mut Frame, area: Rect, state: &SessionViewState) {
    if state.is_input_mode {
        return;
    }
    let hint = match state.status {
        super::super::SessionStatus::WaitingForUser => {
            Line::styled("按 Enter 输入消息...", Style::default().fg(Color::Yellow))
        }
        _ => Line::styled(
            "按 Enter 进入输入模式",
            Style::default().fg(Color::DarkGray),
        ),
    };
    frame.render_widget(
        Paragraph::new(Text::from(vec![hint])).block(Block::default().borders(Borders::NONE)),
        area,
    );
}
