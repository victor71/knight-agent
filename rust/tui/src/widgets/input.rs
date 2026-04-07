//! Input Widget
//!
//! Displays the input line with mode indicator and cursor.

use crate::app::{AppState, InputMode};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Render the input widget
pub fn render_input(f: &mut Frame, area: ratatui::layout::Rect, app: &AppState) {
    let mode_text = match app.input_mode {
        InputMode::Normal => "[NORMAL]",
        InputMode::Insert => "[INSERT]",
        InputMode::Visual => "[VISUAL]",
    };

    let mode_style = match app.input_mode {
        InputMode::Normal => Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        InputMode::Insert => Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        InputMode::Visual => Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    };

    // Create spans with cursor
    let mut spans = vec![
        Span::styled("knight", Style::default().fg(Color::Cyan)),
        Span::styled("> ", Style::default().fg(Color::Cyan)),
    ];

    // Text before cursor
    let text_before_cursor = if app.cursor_position < app.input_buffer.len() {
        app.input_buffer[..app.cursor_position].to_string()
    } else {
        app.input_buffer.clone()
    };
    spans.push(Span::styled(text_before_cursor, Style::default().fg(Color::White)));

    // Cursor character
    let cursor_char = if app.cursor_position < app.input_buffer.len() {
        app.input_buffer.chars().nth(app.cursor_position).unwrap_or(' ')
    } else {
        ' '
    };

    if app.input_mode == InputMode::Insert {
        // In insert mode, cursor is an underline block showing the character at cursor
        spans.push(Span::styled(cursor_char.to_string(), Style::default()
            .fg(Color::Black)
            .bg(Color::White)));
        // Text after cursor
        if app.cursor_position < app.input_buffer.len() {
            spans.push(Span::styled(
                &app.input_buffer[app.cursor_position + 1..],
                Style::default().fg(Color::White),
            ));
        }
    } else {
        // In normal mode, cursor is a block cursor
        spans.push(Span::styled("█", Style::default()
            .fg(Color::Yellow)));
    }

    // Mode indicator at the end
    spans.push(Span::styled(
        format!("  {}", mode_text),
        mode_style,
    ));

    let line = Line::from(spans);
    let paragraph = Paragraph::new(vec![line])
        .block(Block::default()
            .title("Input")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)));

    f.render_widget(paragraph, area);
}
