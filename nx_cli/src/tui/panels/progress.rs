use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::super::{AgentProgress, SessionViewState};

pub fn render_progress_panel(frame: &mut Frame, area: Rect, state: &SessionViewState) {
    let block = Block::default()
        .title("进度")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Green));
    let lines: Vec<Line> = if state.agent_states.is_empty() {
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
            .map(|(role, status, ticks)| {
                let icon = match status {
                    AgentProgress::Idle => "○",
                    AgentProgress::Running => "◉",
                    AgentProgress::Completed => "✓",
                    AgentProgress::Failed => "✗",
                };
                let elapsed = ticks_to_duration(*ticks);
                Line::from(format!("{} {:12} {}", icon, role, elapsed))
            })
            .collect()
    };
    frame.render_widget(Paragraph::new(Text::from(lines)).block(block), area);
}

/// Convert tick count (100ms per tick) to human-readable duration
fn ticks_to_duration(ticks: u64) -> String {
    let total_secs = ticks as f64 * 0.1;
    if total_secs < 60.0 {
        format!("{:.1}s", total_secs)
    } else {
        let mins = (total_secs / 60.0) as u64;
        let secs = total_secs % 60.0;
        format!("{}m{:.0}s", mins, secs)
    }
}
