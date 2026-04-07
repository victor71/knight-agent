//! TUI Layout System
//!
//! Defines the layout for the TUI application.

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Layout chunks for the main UI
#[derive(Debug, Clone, Copy)]
pub struct LayoutChunks {
    pub header: Rect,
    pub main: Rect,
    pub input: Rect,
    pub status: Rect,
}

/// Calculate the main layout
pub fn calculate_main_layout(area: Rect) -> LayoutChunks {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(3),  // Header
                Constraint::Min(0),     // Main output
                Constraint::Length(3),  // Input (needs 3 lines: border + title + content)
                Constraint::Length(4),  // Status bar (borders + 2 content lines)
            ]
            .as_ref(),
        )
        .split(area);

    LayoutChunks {
        header: chunks[0],
        main: chunks[1],
        input: chunks[2],
        status: chunks[3],
    }
}

/// Layout chunks for the header (left: project info, center: session info, right: time/stats)
#[derive(Debug, Clone, Copy)]
pub struct HeaderChunks {
    pub left: Rect,
    pub center: Rect,
    pub right: Rect,
}

/// Calculate the header layout
pub fn calculate_header_layout(area: Rect) -> HeaderChunks {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage(40), // Left: project info
                Constraint::Percentage(35), // Center: session info
                Constraint::Percentage(25), // Right: time/stats
            ]
            .as_ref(),
        )
        .split(area);

    HeaderChunks {
        left: chunks[0],
        center: chunks[1],
        right: chunks[2],
    }
}

/// Layout chunks for the status bar
#[derive(Debug, Clone, Copy)]
pub struct StatusChunks {
    pub left: Rect,
    pub center: Rect,
    pub right: Rect,
}

/// Calculate the status bar layout
pub fn calculate_status_layout(area: Rect) -> StatusChunks {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage(35), // Left: system status
                Constraint::Percentage(40), // Center: current task
                Constraint::Percentage(25), // Right: metrics
            ]
            .as_ref(),
        )
        .split(area);

    StatusChunks {
        left: chunks[0],
        center: chunks[1],
        right: chunks[2],
    }
}

/// Calculate popup layout (centered)
pub fn calculate_popup_layout(area: Rect, width: u16, height: u16) -> Rect {
    let popup_width = width.min(area.width.saturating_sub(4));
    let popup_height = height.min(area.height.saturating_sub(4));

    let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_height)) / 2;

    Rect::new(x, y, popup_width, popup_height)
}

/// Calculate popup layout for a list with item count
pub fn calculate_list_popup_layout(area: Rect, item_count: usize, item_height: u16) -> Rect {
    let max_height = area.height.saturating_sub(4);
    let content_height = (item_count as u16 * item_height).min(max_height);
    let height = content_height + 2; // +2 for borders

    calculate_popup_layout(area, 60, height)
}
