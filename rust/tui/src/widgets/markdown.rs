//! Markdown renderer for TUI
//!
//! Parses markdown and converts it to ratatui-compatible styled text.

use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Render markdown content to styled lines
pub fn render_markdown(content: &str) -> Vec<Line<'static>> {
    // Limit recursion depth to prevent stack overflow
    thread_local! {
        static RECURSION_DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }

    let depth = RECURSION_DEPTH.with(|d| d.get());
    if depth > 10 {
        // Too deep, render as plain text with simple styling
        return vec![Line::from(vec![Span::styled(
            content.to_string(),
            Style::default().fg(Color::White),
        )])];
    }

    RECURSION_DEPTH.with(|d| d.set(depth + 1));
    let result = render_markdown_inner(content);
    RECURSION_DEPTH.with(|d| d.set(depth));
    result
}

fn render_markdown_inner(content: &str) -> Vec<Line<'static>> {
    // First, check if content contains tables and handle them separately
    if content.contains('|') && content.contains("---") {
        return render_markdown_with_tables(content);
    }

    let parser = Parser::new(content);
    let mut lines = Vec::new();
    let mut current_line: Vec<Span<'static>> = Vec::new();
    let mut in_code_block = false;
    let mut in_link = false;
    let mut link_url: Option<String> = None;
    let mut list_level = 0;
    let mut quote_level = 0;
    let mut heading_level: Option<u32> = None;

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    heading_level = Some(level as u32);
                }
                Tag::Paragraph => {
                    // Start of paragraph - add indent for lists/quotes
                    let prefix = "  ".repeat(list_level + quote_level);
                    if !prefix.is_empty() {
                        current_line.push(Span::styled(prefix, Style::default()));
                    }
                }
                Tag::BlockQuote(_) => {
                    quote_level += 1;
                }
                Tag::CodeBlock(kind) => {
                    in_code_block = true;
                    // Add code block header
                    let lang = match kind {
                        pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.to_string(),
                        pulldown_cmark::CodeBlockKind::Indented => "text".to_string(),
                    };
                    lines.push(Line::from(vec![Span::styled(
                        format!("```{}", if lang.is_empty() { "" } else { &lang }),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::ITALIC),
                    )]));
                }
                Tag::List(_) => {
                    // List container - nothing to add
                }
                Tag::Item => {
                    list_level += 1;
                    let prefix = "  ".repeat(list_level - 1 + quote_level);
                    current_line.push(Span::styled(
                        format!("{}• ", prefix),
                        Style::default().fg(Color::Cyan),
                    ));
                }
                Tag::Emphasis => {
                    // Italic text handled in Text event
                }
                Tag::Strong => {
                    // Bold text handled in Text event
                }
                Tag::Link { dest_url, .. } => {
                    in_link = true;
                    link_url = Some(dest_url.to_string());
                }
                Tag::Strikethrough => {
                    // Strikethrough handled in Text event
                }
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Heading(_) => {
                    // End of heading - finalize line
                    if !current_line.is_empty() {
                        lines.push(Line::from(current_line.clone()));
                        current_line.clear();
                    }
                    lines.push(Line::from("")); // Empty line after heading
                    heading_level = None;
                }
                TagEnd::Paragraph => {
                    // End of paragraph
                    if !current_line.is_empty() {
                        lines.push(Line::from(current_line.clone()));
                        current_line.clear();
                    }
                    lines.push(Line::from("")); // Empty line after paragraph
                }
                TagEnd::BlockQuote(_) => {
                    quote_level -= 1;
                    if !current_line.is_empty() {
                        lines.push(Line::from(current_line.clone()));
                        current_line.clear();
                    }
                }
                TagEnd::CodeBlock => {
                    in_code_block = false;
                    lines.push(Line::from(vec![Span::styled(
                        "```",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::ITALIC),
                    )]));
                    lines.push(Line::from("")); // Empty line after code block
                }
                TagEnd::List(_) => {
                    // End of list - nothing to do
                }
                TagEnd::Item => {
                    list_level -= 1;
                    if !current_line.is_empty() {
                        lines.push(Line::from(current_line.clone()));
                        current_line.clear();
                    }
                }
                TagEnd::Link => {
                    in_link = false;
                    if let Some(url) = link_url.take() {
                        // Show URL in parentheses
                        current_line.push(Span::styled(
                            format!(" ({})", url),
                            Style::default()
                                .fg(Color::DarkGray)
                                .add_modifier(Modifier::ITALIC),
                        ));
                    }
                }
                _ => {}
            },
            Event::Text(text) => {
                let text = text.to_string();
                if in_code_block {
                    // Code block content
                    lines.push(Line::from(vec![Span::styled(
                        text.clone(),
                        Style::default().fg(Color::Cyan),
                    )]));
                } else if in_link {
                    current_line.push(Span::styled(
                        text.clone(),
                        Style::default()
                            .fg(Color::Blue)
                            .add_modifier(Modifier::UNDERLINED),
                    ));
                } else if let Some(level) = heading_level {
                    // Heading text with appropriate style
                    current_line.push(Span::styled(
                        text.clone(),
                        heading_style(level),
                    ));
                } else {
                    // Regular text
                    current_line.push(Span::styled(text.clone(), Style::default().fg(Color::White)));
                }
            }
            Event::Code(text) => {
                // Inline code
                current_line.push(Span::styled(
                    format!("{}{}{}", '`', text, '`'),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::ITALIC),
                ));
            }
            Event::SoftBreak | Event::HardBreak => {
                if !current_line.is_empty() {
                    lines.push(Line::from(current_line.clone()));
                    current_line.clear();
                }
            }
            Event::Rule => {
                // Horizontal rule
                lines.push(Line::from(vec![Span::styled(
                    "───".repeat(20),
                    Style::default().fg(Color::DarkGray),
                )]));
            }
            _ => {
                // Other events not implemented
            }
        }
    }

    // Don't forget the last line
    if !current_line.is_empty() {
        lines.push(Line::from(current_line));
    }

    lines
}

/// Get heading style based on level
fn heading_style(level: u32) -> Style {
    let color = match level {
        1 => Color::Red,
        2 => Color::Yellow,
        3 => Color::Green,
        4 => Color::Cyan,
        5 => Color::Blue,
        _ => Color::Magenta,
    };
    Style::default()
        .fg(color)
        .add_modifier(Modifier::BOLD)
}

/// Parse markdown and extract plain text (for simple display)
pub fn markdown_to_plain_text(content: &str) -> String {
    let parser = Parser::new(content);
    let mut result = String::new();

    for event in parser {
        match event {
            Event::Text(text) | Event::Code(text) => {
                result.push_str(&text);
            }
            Event::SoftBreak => result.push(' '),
            Event::HardBreak => result.push('\n'),
            Event::Rule => result.push_str("\n───\n"),
            _ => {}
        }
    }

    result
}

/// Check if content contains markdown syntax
pub fn is_markdown(content: &str) -> bool {
    content.contains('#')
        || content.contains('*')
        || content.contains('`')
        || content.contains('[')
        || content.contains("```")
        || content.contains('>')
        || content.contains("---")
        || content.contains('|')
}

/// Render markdown with table support (simplified)
fn render_markdown_with_tables(content: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    // Simple table renderer - detect table rows and render them
    for line in content.lines() {
        if line.trim().starts_with('|') && line.trim().ends_with('|') {
            // This is a table row
            let cells: Vec<&str> = line
                .trim()
                .trim_start_matches('|')
                .trim_end_matches('|')
                .split('|')
                .map(|s| s.trim())
                .collect();

            // Check if this is a separator row
            if cells.iter().any(|c| c.starts_with("---") || c.starts_with("===")) {
                // Separator row - render as line
                lines.push(Line::from(vec![Span::styled(
                    "─".repeat(40),
                    Style::default().fg(Color::DarkGray),
                )]));
            } else {
                // Regular table row - render cells
                let mut spans = Vec::new();
                for (i, cell) in cells.iter().enumerate() {
                    if i > 0 {
                        spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
                    }
                    spans.push(Span::styled(
                        cell.to_string(),
                        Style::default().fg(Color::White),
                    ));
                }
                lines.push(Line::from(spans));
            }
        } else if line.trim().is_empty() {
            lines.push(Line::from(""));
        } else {
            // Regular markdown line
            let rendered = render_markdown(line);
            lines.extend(rendered);
        }
    }

    lines
}

/// Render a table from collected cells
fn render_table(
    cells: &[Vec<Span<'static>>],
    column_widths: &[usize],
    lines: &mut Vec<Line<'static>>,
) {
    if cells.is_empty() || column_widths.is_empty() {
        return;
    }

    let num_columns = column_widths.len();
    let num_rows = cells.len() / num_columns;

    // Add empty line before table
    if !lines.is_empty() {
        let last_line = lines.last().unwrap();
        if !last_line.spans.is_empty() {
            lines.push(Line::from(""));
        }
    }

    // Render each row
    for row in 0..num_rows {
        let mut row_spans = Vec::new();
        let mut separator_spans = Vec::new();

        for col in 0..num_columns {
            let cell_index = row * num_columns + col;
            let cell: &[Span<'static>] = if cell_index < cells.len() {
                &cells[cell_index]
            } else {
                &[]
            };

            let cell_text: String = cell.iter().map(|s| {
                match &s.content {
                    cow => cow.to_string()
                }
            }).collect();
            let width = column_widths[col];

            // Pad cell content to column width
            let padded = format!("{:<width$}", cell_text, width = width);

            // Add cell style (header vs body)
            let style = if row == 0 {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            row_spans.push(Span::styled(padded.clone(), style));

            // Add separator after header or before each row
            let separator = format!("{:-<width$}", "", width = width);
            separator_spans.push(Span::styled(separator, Style::default().fg(Color::DarkGray)));

            // Add column separator
            if col < num_columns - 1 {
                row_spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
                separator_spans.push(Span::styled("─┼─", Style::default().fg(Color::DarkGray)));
            }
        }

        lines.push(Line::from(row_spans));

        // Add separator line after header
        if row == 0 {
            lines.push(Line::from(separator_spans));
        }
    }

    // Add empty line after table
    lines.push(Line::from(""));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_paragraph() {
        let markdown = "Hello world";
        let lines = render_markdown(markdown);
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_heading() {
        let markdown = "# Heading 1\n## Heading 2";
        let lines = render_markdown(markdown);
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_code_block() {
        let markdown = "```rust\nfn hello() {}\n```";
        let lines = render_markdown(markdown);
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_inline_code() {
        let markdown = "Use `print()` for output";
        let lines = render_markdown(markdown);
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_list() {
        let markdown = "- Item 1\n- Item 2";
        let lines = render_markdown(markdown);
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_link() {
        let markdown = "[OpenAI](https://openai.com)";
        let lines = render_markdown(markdown);
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_markdown_detection() {
        assert!(is_markdown("# Heading"));
        assert!(is_markdown("*italic*"));
        assert!(is_markdown("**bold**"));
        assert!(is_markdown("`code`"));
        assert!(is_markdown("[link](url)"));
        assert!(is_markdown("```rust\ncode\n```"));
        assert!(is_markdown("| col1 | col2 |"));
        assert!(!is_markdown("plain text"));
    }

    #[test]
    fn test_table() {
        let markdown = "| Header 1 | Header 2 |\n|----------|----------|\n| Cell 1   | Cell 2   |";
        let lines = render_markdown(markdown);
        assert!(!lines.is_empty());
    }
}
