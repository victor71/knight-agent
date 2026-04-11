//! Header Widget
//!
//! Displays project information, session info, and system stats.

use crate::app::AppState;
use crate::layout::calculate_header_layout;
use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

/// Render the header widget
pub fn render_header(f: &mut Frame, area: ratatui::layout::Rect, app: &AppState) {
    let chunks = calculate_header_layout(area);

    // Left section: Project info
    let project_info = vec![
        Line::from(vec![
            Span::styled("📦 ", Style::default().fg(Color::Cyan)),
            Span::styled(
                &app.project_info.path,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Branch: ", Style::default().fg(Color::Gray)),
            Span::styled(
                app.project_info.git_branch.as_deref().unwrap_or("none"),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::ITALIC),
            ),
        ]),
        Line::from(vec![
            Span::styled("Agents: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", app.agents.len()),
                Style::default().fg(Color::Cyan),
            ),
            Span::raw(" | "),
            Span::styled("Tasks: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", app.tasks.len()),
                Style::default().fg(Color::Cyan),
            ),
        ]),
    ];

    let project_paragraph =
        Paragraph::new(project_info).block(Block::default().borders(Borders::ALL));
    f.render_widget(project_paragraph, chunks.left);

    // Center section: Session info
    let session_info = vec![
        Line::from(vec![
            Span::styled("Session: ", Style::default().fg(Color::Gray)),
            Span::styled(
                &app.session_info.id[..8.min(app.session_info.id.len())],
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Messages: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", app.session_info.message_count),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::styled("[", Style::default().fg(Color::Gray)),
            Span::styled(
                "+ New",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("] "),
            Span::styled("[", Style::default().fg(Color::Gray)),
            Span::styled(
                "- Switch",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("]", Style::default().fg(Color::Gray)),
            Span::raw(" (Alt+N/S)"),
        ]),
    ];

    let session_paragraph = Paragraph::new(session_info)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);
    f.render_widget(session_paragraph, chunks.center);

    // Right section: Time and system stats
    let time = app.current_time.format("%H:%M:%S").to_string();
    let uptime_secs = app.system_status.uptime.as_secs();
    let uptime_mins = uptime_secs / 60;
    let uptime_hours = uptime_mins / 60;

    let stats_info = vec![
        Line::from(vec![Span::styled(
            time,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("CPU: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:.0}%", app.system_status.cpu_usage),
                Style::default().fg(Color::Cyan),
            ),
            Span::raw(" | "),
            Span::styled("Mem: ", Style::default().fg(Color::Gray)),
            Span::styled(
                AppState::format_bytes(app.system_status.memory_usage),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::styled("Uptime: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!(
                    "{:02}:{:02}:{:02}",
                    uptime_hours,
                    uptime_mins % 60,
                    uptime_secs % 60
                ),
                Style::default().fg(Color::Cyan),
            ),
        ]),
    ];

    let stats_paragraph = Paragraph::new(stats_info)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Right)
        .wrap(Wrap { trim: true });
    f.render_widget(stats_paragraph, chunks.right);
}
