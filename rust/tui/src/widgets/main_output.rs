//! Main Output Widget
//!
//! Displays the main output area with rich text and code highlighting.

use crate::app::AppState;
use crate::state::OutputStyle;
use crate::widgets::markdown;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

/// Prefix for agent messages
const AGENT_PREFIX: &str = "Agent: ";

/// Render the main output widget
pub fn render_main_output(f: &mut Frame, area: ratatui::layout::Rect, app: &AppState) {
    let mut lines = Vec::new();

    for output_line in &app.output_lines {
        let styled_line = style_output_line(output_line);
        lines.push(styled_line);
    }

    // If no output, show placeholder
    if lines.is_empty() && !app.processing_state.is_processing {
        lines.push(Line::from(vec![
            Span::styled(
                "Knight Agent TUI",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" - "),
            Span::styled(
                "Type a message or command to get started",
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    }

    // Add processing animation if active
    if let Some(spinner) = app.get_processing_animation() {
        // Count queued inputs if any
        let queue_info = if app.processing_state.input_queue.len() > 1 {
            format!(" ({} queued)", app.processing_state.input_queue.len())
        } else {
            String::new()
        };
        lines.push(Line::from(vec![
            Span::styled(spinner, Style::default().fg(Color::Cyan)),
            Span::styled(" Processing...", Style::default().fg(Color::Gray)),
            Span::styled(queue_info, Style::default().fg(Color::DarkGray)),
        ]));
    }

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Output "),
        )
        .scroll((app.output_scroll as u16, 0))
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

/// Style an output line based on its style type
fn style_output_line(output_line: &crate::state::OutputLine) -> Line<'_> {
    match &output_line.style {
        OutputStyle::UserMessage => Line::from(vec![
            Span::styled(
                format!("{} ", output_line.timestamp.format("%H:%M:%S")),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                "User: ",
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
            Span::styled(&output_line.content, Style::default().fg(Color::White)),
        ]),
        OutputStyle::AgentMessage => {
            // Build agent label with optional agent_id
            let agent_label = if let Some(ref agent_id) = output_line.agent_id {
                format!("{}[{}]: ", AGENT_PREFIX.trim(), agent_id)
            } else {
                AGENT_PREFIX.to_string()
            };

            // Check if content contains markdown
            if markdown::is_markdown(&output_line.content) {
                // For markdown content, render as styled text
                let rendered = markdown::render_markdown(&output_line.content);
                if !rendered.is_empty() {
                    // Add timestamp and Agent label to first line
                    let mut result = Vec::new();
                    for (i, mut line) in rendered.into_iter().enumerate() {
                        if i == 0 {
                            line.spans.insert(0, Span::styled(
                                format!("{} ", output_line.timestamp.format("%H:%M:%S")),
                                Style::default().fg(Color::DarkGray),
                            ));
                            line.spans.insert(1, Span::styled(
                                agent_label.clone(),
                                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                            ));
                        }
                        result.push(line);
                    }
                    // Return first line (multiline handling would need more complex logic)
                    result.into_iter().next().unwrap_or_else(|| Line::from(output_line.content.clone()))
                } else {
                    // Fallback to plain text with Agent label
                    Line::from(vec![
                        Span::styled(
                            format!("{} ", output_line.timestamp.format("%H:%M:%S")),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(
                            agent_label.clone(),
                            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(output_line.content.clone(), Style::default().fg(Color::White)),
                    ])
                }
            } else {
                // Simple syntax highlighting for code blocks (legacy)
                if output_line.content.starts_with("```") {
                    // Code block
                    let lang = output_line.content
                        .strip_prefix("```")
                        .and_then(|s| s.lines().next())
                        .unwrap_or("code");
                    Line::from(vec![
                        Span::styled(
                            format!("{} ", output_line.timestamp.format("%H:%M:%S")),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(
                            agent_label.clone(),
                            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("```{} ", lang),
                            Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC),
                        ),
                    ])
                } else if output_line.content.starts_with("    ") || output_line.content.starts_with("\t") {
                    // Indented code line
                    Line::from(vec![
                        Span::styled(
                            format!("{} ", output_line.timestamp.format("%H:%M:%S")),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(
                            agent_label.clone(),
                            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled("  ", Style::default()),
                        Span::styled(output_line.content.trim().to_string(), Style::default().fg(Color::Cyan)),
                    ])
                } else {
                    // Regular text with Agent label
                    Line::from(vec![
                        Span::styled(
                            format!("{} ", output_line.timestamp.format("%H:%M:%S")),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(
                            agent_label.clone(),
                            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(output_line.content.clone(), Style::default().fg(Color::White)),
                    ])
                }
            }
        }
        OutputStyle::SystemInfo => Line::from(vec![
            Span::styled(
                format!("{} ", output_line.timestamp.format("%H:%M:%S")),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled("ℹ️  ", Style::default().fg(Color::Blue)),
            Span::styled(output_line.content.clone(), Style::default().fg(Color::Gray)),
        ]),
        OutputStyle::Error => Line::from(vec![
            Span::styled(
                format!("{} ", output_line.timestamp.format("%H:%M:%S")),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled("❌ ", Style::default().fg(Color::Red)),
            Span::styled(
                output_line.content.clone(),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
        ]),
        OutputStyle::Code(lang) => Line::from(vec![
            Span::styled(
                format!("{} ", output_line.timestamp.format("%H:%M:%S")),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("[{}] ", lang),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC),
            ),
            Span::styled(output_line.content.clone(), Style::default().fg(Color::Cyan)),
        ]),
        OutputStyle::Status(status) => Line::from(vec![
            Span::styled(
                format!("{} ", output_line.timestamp.format("%H:%M:%S")),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(status, Style::default()),
            Span::styled(" ", Style::default()),
            Span::styled(output_line.content.clone(), Style::default().fg(Color::Gray)),
        ]),
        OutputStyle::Processing => Line::from(vec![
            Span::styled(
                format!("{} ", output_line.timestamp.format("%H:%M:%S")),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(output_line.content.clone(), Style::default().fg(Color::Cyan)),
        ]),
        OutputStyle::Thinking => Line::from(vec![
            Span::styled(
                format!("{} ", output_line.timestamp.format("%H:%M:%S")),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled("💭 ", Style::default().fg(Color::Yellow)),
            Span::styled(
                output_line.content.clone(),
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
            ),
        ]),
    }
}
