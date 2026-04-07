//! Session List Popup Widget
//!
//! Displays a list of sessions for switching.

use crate::app::{AppState, PopupType};
use crate::layout::calculate_list_popup_layout;
use ratatui::{
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
    Frame,
};

/// Render the session list popup
pub fn render_session_popup(f: &mut Frame, area: ratatui::layout::Rect, app: &mut AppState) {
    if app.active_popup != Some(PopupType::SessionList) {
        return;
    }

    let popup_area = calculate_list_popup_layout(area, app.sessions.len().max(3), 1);

    // Clear the area behind the popup
    f.render_widget(Clear, popup_area);

    let items: Vec<ListItem> = app
        .sessions
        .iter()
        .enumerate()
        .map(|(i, session)| {
            let is_current = session.id == app.session_info.id;
            let is_selected = i == app.popup_selected_index;

            let prefix = if is_current {
                "● "
            } else {
                "  "
            };

            let content = format!(
                "{}{} | Messages: {} | {}",
                prefix,
                truncate_string(&session.name, 20),
                session.message_count,
                session.created_at.format("%Y-%m-%d")
            );

            let style = if is_selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else if is_current {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            ListItem::new(content).style(style)
        })
        .collect();

    let title = if app.sessions.is_empty() {
        " Sessions (None) "
    } else {
        " Sessions "
    };

    let list = List::new(items)
        .block(Block::default().title(title).borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    let mut list_state = ListState::default();
    list_state.select(Some(app.popup_selected_index));

    f.render_stateful_widget(list, popup_area, &mut list_state);
}

/// Truncate a string to a maximum length
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}
