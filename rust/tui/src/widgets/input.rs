//! Input Widget
//!
//! Displays the input line with cursor.

use crate::app::AppState;
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Render the input widget
pub fn render_input(f: &mut Frame, area: ratatui::layout::Rect, app: &AppState) {
    let char_count = app.input_buffer.chars().count();
    let cursor_pos = app.cursor_position.min(char_count);

    let line = if app.input_buffer.is_empty() {
        Line::from(vec![
            Span::styled("knight> ", Style::default().fg(Color::Cyan)),
            Span::styled("█", Style::default().fg(Color::Yellow)),
        ])
    } else if cursor_pos == 0 {
        Line::from(vec![
            Span::styled("knight> ", Style::default().fg(Color::Cyan)),
            Span::styled("█", Style::default().fg(Color::White).bg(Color::Black)),
            Span::styled(app.input_buffer.as_str(), Style::default().fg(Color::White)),
        ])
    } else if cursor_pos >= char_count {
        Line::from(vec![
            Span::styled("knight> ", Style::default().fg(Color::Cyan)),
            Span::styled(app.input_buffer.as_str(), Style::default().fg(Color::White)),
            Span::styled("█", Style::default().fg(Color::Yellow)),
        ])
    } else {
        let chars: Vec<char> = app.input_buffer.chars().collect();
        let before: String = chars[..cursor_pos].iter().collect();
        let after: String = chars[cursor_pos..].iter().collect();
        Line::from(vec![
            Span::styled("knight> ", Style::default().fg(Color::Cyan)),
            Span::styled(before, Style::default().fg(Color::White)),
            Span::styled("█", Style::default().fg(Color::White).bg(Color::Black)),
            Span::styled(after, Style::default().fg(Color::White)),
        ])
    };

    let paragraph = Paragraph::new(vec![line])
        .block(Block::default()
            .title("Input")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)));

    f.render_widget(paragraph, area);
}