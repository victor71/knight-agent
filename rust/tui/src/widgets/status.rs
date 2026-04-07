//! Status Bar Widget
//!
//! Displays system status, current task timer, and session metrics.

use crate::app::AppState;
use crate::layout::calculate_status_layout;
use crate::state::CompressionWarningLevel;
use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Render the status bar widget
pub fn render_status(f: &mut Frame, area: ratatui::layout::Rect, app: &AppState) {
    let chunks = calculate_status_layout(area);

    // Left section: System status
    let (status_emoji, status_color) = match app.system_status.system_status {
        crate::event::SystemHealth::Healthy => ("🟢", Color::Green),
        crate::event::SystemHealth::Degraded => ("🟡", Color::Yellow),
        crate::event::SystemHealth::Error => ("🔴", Color::Red),
    };

    let system_status = vec![
        Line::from(vec![
            Span::styled(status_emoji, Style::default().fg(status_color)),
            Span::raw(" "),
            Span::styled(
                match app.system_status.system_status {
                    crate::event::SystemHealth::Healthy => "Running",
                    crate::event::SystemHealth::Degraded => "Degraded",
                    crate::event::SystemHealth::Error => "Error",
                },
                Style::default().fg(status_color),
            ),
            Span::raw(" | "),
            Span::styled("Stage: ", Style::default().fg(Color::Gray)),
            Span::styled(
                &app.system_status.stage,
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!("Modules: {}/{}", app.system_status.initialized_count, app.system_status.module_count),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
    ];

    let system_paragraph = Paragraph::new(system_status)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(system_paragraph, chunks.left);

    // Center section: Current task
    let current_task = app.tasks.iter().find(|t| matches!(t.status, crate::state::TaskStatus::Running));

    let task_info = if let Some(task) = current_task {
        vec![
            Line::from(vec![
                Span::styled("🔄 ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    truncate_string(&task.name, 25),
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled(
                    app.current_task_duration_formatted(),
                    Style::default().fg(Color::Cyan),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    format!("Running tasks: {}", app.tasks.iter().filter(|t| matches!(t.status, crate::state::TaskStatus::Running)).count()),
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
        ]
    } else {
        vec![
            Line::from(vec![
                Span::styled("⏸️  ", Style::default().fg(Color::Gray)),
                Span::styled(
                    "No active task",
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    format!("Pending tasks: {}", app.tasks.iter().filter(|t| matches!(t.status, crate::state::TaskStatus::Pending)).count()),
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
        ]
    };

    let task_paragraph = Paragraph::new(task_info)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);
    f.render_widget(task_paragraph, chunks.center);

    // Right section: Session metrics (token usage + context compression)
    let token_color = if app.session_token_usage.percentage > 80.0 {
        Color::Red
    } else if app.session_token_usage.percentage > 50.0 {
        Color::Yellow
    } else {
        Color::Green
    };

    let context_color = match app.context_compression_status.warning_level() {
        CompressionWarningLevel::Critical => Color::Red,
        CompressionWarningLevel::Warning => Color::Yellow,
        CompressionWarningLevel::Normal => Color::Green,
    };

    let compression_warning = match app.context_compression_status.warning_level() {
        CompressionWarningLevel::Critical => "🔴",
        CompressionWarningLevel::Warning => "⚠️ ",
        CompressionWarningLevel::Normal => "",
    };

    let metrics_info = vec![
        Line::from(vec![
            Span::styled("Tokens: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", app.session_token_usage.current),
                Style::default().fg(token_color),
            ),
            Span::styled(
                format!("/{} ", app.session_token_usage.limit),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("({:.0}%)", app.session_token_usage.percentage),
                Style::default().fg(token_color),
            ),
        ]),
        Line::from(vec![
            Span::styled("Context: ", Style::default().fg(Color::Gray)),
            Span::styled(
                AppState::format_bytes(app.context_compression_status.current_size),
                Style::default().fg(context_color),
            ),
            Span::styled(
                format!("/{} ", AppState::format_bytes(app.context_compression_status.threshold)),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("({:.0}%)", app.context_compression_status.percentage),
                Style::default().fg(context_color),
            ),
            Span::styled(
                compression_warning,
                Style::default().fg(context_color),
            ),
        ]),
    ];

    let metrics_paragraph = Paragraph::new(metrics_info)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Right);
    f.render_widget(metrics_paragraph, chunks.right);
}

/// Truncate a string to a maximum length
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}
