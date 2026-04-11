//! Task List Popup Widget
//!
//! Displays a list of tasks with their status and duration.

use crate::app::{AppState, PopupType};
use crate::layout::calculate_list_popup_layout;
use crate::state::TaskStatus;
use chrono::{DateTime, Local};
use ratatui::{
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
    Frame,
};

/// Render the task list popup
pub fn render_task_popup(f: &mut Frame, area: ratatui::layout::Rect, app: &mut AppState) {
    if app.active_popup != Some(PopupType::TaskList) {
        return;
    }

    let popup_area = calculate_list_popup_layout(area, app.tasks.len().max(3), 1);

    // Clear the area behind the popup
    f.render_widget(Clear, popup_area);

    // Sort tasks: running first, then pending, then completed/failed
    let mut sorted_tasks = app.tasks.clone();
    sorted_tasks.sort_by(|a, b| {
        let priority_a = task_priority(a.status);
        let priority_b = task_priority(b.status);
        priority_a.cmp(&priority_b)
    });

    let items: Vec<ListItem> = sorted_tasks
        .iter()
        .enumerate()
        .map(|(i, task)| {
            let is_selected = i == app.popup_selected_index;
            let duration = format_task_duration(task.started_at);

            let (status_emoji, status_color) = match task.status {
                TaskStatus::Running => ("🔄", Color::Yellow),
                TaskStatus::Pending => ("⏳", Color::Cyan),
                TaskStatus::Completed => ("✅", Color::Green),
                TaskStatus::Failed => ("❌", Color::Red),
            };

            let content = format!(
                "{} {} {} | {}",
                status_emoji,
                truncate_string(&task.name, 18),
                duration,
                task.agent_id.as_deref().unwrap_or("unassigned")
            );

            let style = if is_selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(status_color)
            };

            ListItem::new(content).style(style)
        })
        .collect();

    let title = if app.tasks.is_empty() {
        " Tasks (None) "
    } else {
        &format!(" Tasks ({}) ", app.tasks.len())
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

/// Get task priority for sorting (lower = higher priority)
fn task_priority(status: TaskStatus) -> u8 {
    match status {
        TaskStatus::Running => 0,
        TaskStatus::Pending => 1,
        TaskStatus::Failed => 2,
        TaskStatus::Completed => 3,
    }
}

/// Format task duration
fn format_task_duration(started_at: DateTime<Local>) -> String {
    let now = Local::now();
    let duration = now.signed_duration_since(started_at);

    let total_seconds = duration.num_seconds().abs();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    }
}

/// Truncate a string to a maximum length
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}
