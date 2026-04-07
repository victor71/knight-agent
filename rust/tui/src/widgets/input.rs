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
        InputMode::Normal => "NORMAL",
        InputMode::Insert => "INSERT",
        InputMode::Visual => "VISUAL",
    };

    let mode_style = match app.input_mode {
        InputMode::Normal => Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        InputMode::Insert => Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        InputMode::Visual => Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    };

    let prompt = vec![
        Line::from(vec![
            Span::styled("knight", Style::default().fg(Color::Cyan)),
            Span::styled(">", Style::default().fg(Color::Cyan)),
            Span::raw(" "),
            Span::styled(mode_text, mode_style),
            Span::raw(" "),
            Span::styled(&app.input_buffer, Style::default().fg(Color::White)),
            // Cursor (rendered as a block)
            if app.input_mode == InputMode::Insert {
                Span::styled(" ", Style::default().fg(Color::White).bg(Color::White))
            } else {
                Span::raw("")
            },
        ]),
    ];

    let paragraph = Paragraph::new(prompt)
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(paragraph, area);
}
