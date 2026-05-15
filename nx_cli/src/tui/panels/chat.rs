use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use unicode_width::UnicodeWidthChar;

use super::super::SessionViewState;

pub fn render_chat_panel(frame: &mut Frame, area: Rect, state: &SessionViewState) {
    let block = Block::default()
        .title("对话区")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));
    let inner_width = area.width.saturating_sub(2) as usize;
    let text = if state.messages.is_empty() {
        Text::from("等待团队会话启动...\n\n使用 `nx team \"<任务描述>\"` 启动会话")
    } else {
        let last_idx = state.messages.len().saturating_sub(1);
        // Show last N messages that fit on screen
        let visible_start = state
            .messages
            .len()
            .saturating_sub(area.height.saturating_sub(2) as usize);
        let lines: Vec<Line> = state
            .messages
            .iter()
            .enumerate()
            .skip(visible_start)
            .map(|(i, (name, _role, chunk))| {
                let display_text = if i == last_idx {
                    let end = state.typewriter_pos.min(chunk.len());
                    &chunk[..end]
                } else {
                    chunk.as_str()
                };
                let wrapped = wrap_by_width(display_text, inner_width.max(20));
                let mut spans = vec![Span::styled(
                    format!("[{}] ", name),
                    Style::default().fg(Color::Yellow),
                )];
                spans.extend(wrapped.into_iter().map(Span::raw));
                Line::from(spans)
            })
            .collect();
        Text::from(lines)
    };
    frame.render_widget(
        Paragraph::new(text).block(block).wrap(Wrap { trim: false }),
        area,
    );
}

/// Wrap text to fit within `max_width` terminal columns, respecting CJK double-width characters.
fn wrap_by_width(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 || text.is_empty() {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let mut current = String::with_capacity(max_width + 16);
    let mut current_width: usize = 0;
    for ch in text.chars() {
        if ch == '\n' {
            lines.push(current);
            current = String::with_capacity(max_width + 16);
            current_width = 0;
            continue;
        }
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(1);
        if current_width + ch_width > max_width {
            lines.push(current);
            current = String::with_capacity(max_width + 16);
            current_width = 0;
        }
        current.push(ch);
        current_width += ch_width;
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_ascii_fits_in_width() {
        let result = wrap_by_width("hello", 10);
        assert_eq!(result, vec!["hello"]);
    }

    #[test]
    fn wrap_ascii_exact_width() {
        let result = wrap_by_width("hello", 5);
        assert_eq!(result, vec!["hello"]);
    }

    #[test]
    fn wrap_ascii_overflows() {
        let result = wrap_by_width("hello world", 5);
        assert_eq!(result, vec!["hello", " worl", "d"]);
    }

    #[test]
    fn wrap_chinese_two_cols_per_char() {
        // "你好" = 2 CJK chars × 2 cols = 4 cols
        let result = wrap_by_width("你好世界", 4);
        assert_eq!(result, vec!["你好", "世界"]);
    }

    #[test]
    fn wrap_chinese_three_cols() {
        // 3 cols = fits 1 CJK char (2 cols) + 1 ASCII (1 col)
        let result = wrap_by_width("你好世界", 3);
        assert_eq!(result, vec!["你", "好", "世", "界"]);
    }

    #[test]
    fn wrap_mixed_cjk_and_ascii() {
        // "a你b" = 1 + 2 + 1 = 4 cols
        let result = wrap_by_width("a你b好", 4);
        assert_eq!(result, vec!["a你b", "好"]);
    }

    #[test]
    fn wrap_respects_newlines() {
        let result = wrap_by_width("hello\nworld", 10);
        assert_eq!(result, vec!["hello", "world"]);
    }

    #[test]
    fn wrap_empty_text() {
        let result = wrap_by_width("", 10);
        assert_eq!(result, vec![""]);
    }

    #[test]
    fn wrap_zero_width() {
        let result = wrap_by_width("hello", 0);
        assert_eq!(result, vec!["hello"]);
    }

    #[test]
    fn wrap_cjk_not_split_mid_char() {
        // "你好" = 6 bytes, 2 chars, 4 cols. Wrapping at 3 cols should NOT split a char.
        let result = wrap_by_width("你好世界", 3);
        for line in &result {
            // Each line should be valid UTF-8 and complete chars
            assert!(line.chars().count() <= 2);
        }
    }
}
