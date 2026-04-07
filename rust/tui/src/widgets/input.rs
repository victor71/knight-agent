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
    // Create spans with cursor
    let mut spans = vec![
        Span::styled("knight", Style::default().fg(Color::Cyan)),
        Span::styled("> ", Style::default().fg(Color::Cyan)),
    ];

    // Use char indices to properly handle multi-byte UTF-8 characters (like Chinese)
    let char_count = app.input_buffer.chars().count();
    let cursor_char_index = app.cursor_position.min(char_count);

    // Collect chars before cursor
    let text_before_cursor: String = app.input_buffer.chars().take(cursor_char_index).collect();
    spans.push(Span::styled(text_before_cursor, Style::default().fg(Color::White)));

    // Cursor character with inverted colors (block cursor style)
    let cursor_char = app.input_buffer.chars().nth(cursor_char_index).unwrap_or(' ');
    spans.push(Span::styled(cursor_char.to_string(), Style::default()
        .fg(Color::Black)
        .bg(Color::White)));

    // Text after cursor
    let text_after_cursor: String = app.input_buffer.chars().skip(cursor_char_index + 1).collect();
    spans.push(Span::styled(text_after_cursor, Style::default().fg(Color::White)));

    let line = Line::from(spans);
    let paragraph = Paragraph::new(vec![line])
        .block(Block::default()
            .title("Input")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)));

    f.render_widget(paragraph, area);
}
