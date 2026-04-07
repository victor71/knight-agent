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

    // Build content: prompt + text with cursor as block character
    let mut content = String::new();

    // Add prompt
    content.push_str("knight> ");

    // Add characters before cursor
    for (i, c) in app.input_buffer.chars().enumerate() {
        if i == app.cursor_position {
            // Insert cursor before this character
            content.push('█');
        }
        content.push(c);
    }

    // If cursor is at end, add cursor there
    if app.cursor_position >= char_count {
        content.push('█');
    }

    let line = Line::from(vec![Span::raw(content)]);
    let paragraph = Paragraph::new(vec![line])
        .block(Block::default()
            .title("Input")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)));

    f.render_widget(paragraph, area);
}
