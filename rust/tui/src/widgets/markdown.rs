//! Markdown renderer for TUI
//!
//! Parses markdown and converts it to ratatui-compatible styled text.

use pulldown_cmark::{Event, Parser};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Render markdown content to styled lines
pub fn render_markdown(content: &str) -> Vec<Line<'static>> {
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
                pulldown_cmark::Tag::Heading { level, .. } => {
                    heading_level = Some(level as u32);
                }
                pulldown_cmark::Tag::Paragraph => {
                    // Start of paragraph - add indent for lists/quotes
                    let prefix = "  ".repeat(list_level + quote_level);
                    if !prefix.is_empty() {
                        current_line.push(Span::styled(prefix, Style::default()));
                    }
                }
                pulldown_cmark::Tag::BlockQuote(_) => {
                    quote_level += 1;
                }
                pulldown_cmark::Tag::CodeBlock(kind) => {
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
                pulldown_cmark::Tag::List(_) => {
                    // List container - nothing to add
                }
                pulldown_cmark::Tag::Item => {
                    list_level += 1;
                    let prefix = "  ".repeat(list_level - 1 + quote_level);
                    current_line.push(Span::styled(
                        format!("{}• ", prefix),
                        Style::default().fg(Color::Cyan),
                    ));
                }
                pulldown_cmark::Tag::Emphasis => {
                    // Italic text handled in Text event
                }
                pulldown_cmark::Tag::Strong => {
                    // Bold text handled in Text event
                }
                pulldown_cmark::Tag::Link { dest_url, .. } => {
                    in_link = true;
                    link_url = Some(dest_url.to_string());
                }
                pulldown_cmark::Tag::Strikethrough => {
                    // Strikethrough handled in Text event
                }
                _ => {}
            },
            Event::End(tag) => match tag {
                pulldown_cmark::TagEnd::Heading(_) => {
                    // End of heading - finalize line
                    if !current_line.is_empty() {
                        lines.push(Line::from(current_line.clone()));
                        current_line.clear();
                    }
                    lines.push(Line::from("")); // Empty line after heading
                    heading_level = None;
                }
                pulldown_cmark::TagEnd::Paragraph => {
                    // End of paragraph
                    if !current_line.is_empty() {
                        lines.push(Line::from(current_line.clone()));
                        current_line.clear();
                    }
                    lines.push(Line::from("")); // Empty line after paragraph
                }
                pulldown_cmark::TagEnd::BlockQuote(_) => {
                    quote_level -= 1;
                    if !current_line.is_empty() {
                        lines.push(Line::from(current_line.clone()));
                        current_line.clear();
                    }
                }
                pulldown_cmark::TagEnd::CodeBlock => {
                    in_code_block = false;
                    lines.push(Line::from(vec![Span::styled(
                        "```",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::ITALIC),
                    )]));
                    lines.push(Line::from("")); // Empty line after code block
                }
                pulldown_cmark::TagEnd::List(_) => {
                    // End of list - nothing to do
                }
                pulldown_cmark::TagEnd::Item => {
                    list_level -= 1;
                    if !current_line.is_empty() {
                        lines.push(Line::from(current_line.clone()));
                        current_line.clear();
                    }
                }
                pulldown_cmark::TagEnd::Link => {
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
        assert!(!is_markdown("plain text"));
    }
}
