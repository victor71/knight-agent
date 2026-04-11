//! TUI Test Harness
//!
//! A testing utility for verifying TUI state changes during development.
//! This is NOT meant to replace integration tests with rexpect, but serves
//! as a quick validation tool during development.
//!
//! # Usage
//!
//! ```rust,ignore
//! use tui::test_harness::{TuiTestHarness, Key};
//!
//! let mut harness = TuiTestHarness::new();
//!
//! // Type some text
//! harness.type_text("hello");
//! assert_eq!(harness.input_buffer(), "hello");
//!
//! // Press Enter to submit
//! harness.press_enter();
//! assert!(harness.output_contains("正在处理中"));
//!
//! // Check popup state
//! harness.press_alt_s();
//! assert!(harness.has_popup(PopupType::SessionList));
//! ```

use crate::app::AppState;
use crate::event::AppEvent;
use crate::PopupType;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use std::sync::Mutex;
use tokio::sync::mpsc;

/// A test harness for verifying TUI state changes.
///
/// This harness allows rapid development-time verification of TUI behavior
/// without requiring a real terminal or integration test infrastructure.
pub struct TuiTestHarness {
    state: AppState,
    /// Events that were sent during the test
    sent_events: Mutex<Vec<AppEvent>>,
}

impl TuiTestHarness {
    /// Create a new test harness
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let state = AppState::new(tx, rx);
        Self {
            state,
            sent_events: Mutex::new(Vec::new()),
        }
    }

    /// Get the current input buffer
    pub fn input_buffer(&self) -> &str {
        &self.state.input_buffer
    }

    /// Get the current cursor position
    pub fn cursor_position(&self) -> usize {
        self.state.cursor_position
    }

    /// Get all output lines
    pub fn output_lines(&self) -> &[crate::state::OutputLine] {
        &self.state.output_lines
    }

    /// Check if output contains a substring
    pub fn output_contains(&self, text: &str) -> bool {
        self.state
            .output_lines
            .iter()
            .any(|line| line.content.contains(text))
    }

    /// Get the last output line content
    pub fn last_output(&self) -> Option<&str> {
        self.state.output_lines.last().map(|l| l.as_str())
    }

    /// Check if a popup is active
    pub fn has_popup(&self, popup: PopupType) -> bool {
        self.state.active_popup == Some(popup)
    }

    /// Get active popup type
    pub fn active_popup(&self) -> Option<PopupType> {
        self.state.active_popup
    }

    /// Get popup selected index
    pub fn popup_selected_index(&self) -> usize {
        self.state.popup_selected_index
    }

    /// Get the number of sent events
    pub fn sent_event_count(&self) -> usize {
        self.sent_events.lock().unwrap().len()
    }

    // ============ Key Event Simulation ============

    /// Simulate typing text character by character
    pub fn type_text(&mut self, text: &str) {
        for c in text.chars() {
            self.handle_key_code(KeyCode::Char(c), KeyModifiers::NONE);
        }
    }

    /// Simulate pressing Enter
    pub fn press_enter(&mut self) {
        self.handle_key_code(KeyCode::Enter, KeyModifiers::NONE);
    }

    /// Simulate pressing Backspace
    pub fn press_backspace(&mut self) {
        self.handle_key_code(KeyCode::Backspace, KeyModifiers::NONE);
    }

    /// Simulate pressing Escape
    pub fn press_escape(&mut self) {
        self.handle_key_code(KeyCode::Esc, KeyModifiers::NONE);
    }

    /// Simulate pressing ArrowUp
    pub fn press_up(&mut self) {
        self.handle_key_code(KeyCode::Up, KeyModifiers::NONE);
    }

    /// Simulate pressing ArrowDown
    pub fn press_down(&mut self) {
        self.handle_key_code(KeyCode::Down, KeyModifiers::NONE);
    }

    /// Simulate pressing ArrowLeft
    pub fn press_left(&mut self) {
        self.handle_key_code(KeyCode::Left, KeyModifiers::NONE);
    }

    /// Simulate pressing ArrowRight
    pub fn press_right(&mut self) {
        self.handle_key_code(KeyCode::Right, KeyModifiers::NONE);
    }

    /// Simulate Alt+S
    pub fn press_alt_s(&mut self) {
        self.handle_key_code(KeyCode::Char('s'), KeyModifiers::ALT);
    }

    /// Simulate Alt+T
    pub fn press_alt_t(&mut self) {
        self.handle_key_code(KeyCode::Char('t'), KeyModifiers::ALT);
    }

    /// Simulate Ctrl+Q
    pub fn press_ctrl_q(&mut self) {
        self.handle_key_code(KeyCode::Char('q'), KeyModifiers::CONTROL);
    }

    /// Handle a key code with given modifiers
    fn handle_key_code(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        let key = KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        };
        self.handle_key_event(key);
    }

    /// Handle a key event - mirrors TuiApp::handle_key_event logic
    fn handle_key_event(&mut self, key: KeyEvent) {
        match (key.code, key.modifiers) {
            (KeyCode::Char('s'), KeyModifiers::ALT) => {
                self.state.toggle_popup(PopupType::SessionList);
            }
            (KeyCode::Char('t'), KeyModifiers::ALT) => {
                self.state.toggle_popup(PopupType::TaskList);
            }
            (KeyCode::Up, KeyModifiers::NONE) => {
                if self.state.active_popup.is_some() {
                    if self.state.popup_selected_index > 0 {
                        self.state.popup_selected_index -= 1;
                    }
                }
            }
            (KeyCode::Down, KeyModifiers::NONE) => {
                if self.state.active_popup.is_some() {
                    let max_index = match self.state.active_popup {
                        Some(PopupType::SessionList) => self.state.sessions.len().saturating_sub(1),
                        Some(PopupType::TaskList) => self.state.tasks.len().saturating_sub(1),
                        None => 0,
                    };
                    if self.state.popup_selected_index < max_index {
                        self.state.popup_selected_index += 1;
                    }
                }
            }
            (KeyCode::Esc, KeyModifiers::NONE) => {
                self.state.close_popup();
            }
            (KeyCode::Char(c), KeyModifiers::NONE) => {
                let chars: Vec<char> = self.state.input_buffer.chars().collect();
                let pos = self.state.cursor_position.min(chars.len());
                let mut new_chars = chars;
                new_chars.insert(pos, c);
                self.state.input_buffer = new_chars.iter().collect();
                self.state.cursor_position = pos + 1;
            }
            (KeyCode::Backspace, KeyModifiers::NONE) => {
                if self.state.cursor_position > 0 {
                    let chars: Vec<char> = self.state.input_buffer.chars().collect();
                    let pos = self.state.cursor_position;
                    if pos > 0 {
                        let mut new_chars = chars;
                        new_chars.remove(pos - 1);
                        self.state.input_buffer = new_chars.iter().collect();
                        self.state.cursor_position = pos - 1;
                    }
                }
            }
            (KeyCode::Left, KeyModifiers::NONE) => {
                if self.state.cursor_position > 0 {
                    self.state.cursor_position -= 1;
                }
            }
            (KeyCode::Right, KeyModifiers::NONE) => {
                let char_count = self.state.input_buffer.chars().count();
                if self.state.cursor_position < char_count {
                    self.state.cursor_position += 1;
                }
            }
            (KeyCode::Enter, KeyModifiers::NONE) => {
                if let Some(popup) = self.state.active_popup {
                    match popup {
                        PopupType::SessionList => {
                            if let Some(session) =
                                self.state.sessions.get(self.state.popup_selected_index)
                            {
                                let event = AppEvent::SessionSwitch(session.id.clone());
                                self.record_event(event);
                            }
                        }
                        PopupType::TaskList => {}
                    }
                    self.state.close_popup();
                } else if !self.state.input_buffer.is_empty() {
                    let input = self.state.input_buffer.clone();
                    self.state.input_buffer.clear();
                    self.state.cursor_position = 0;

                    if self.state.processing_state.is_processing {
                        // Already processing - queue the input
                        self.state.processing_state.input_queue.push(input);
                    } else {
                        // Start processing - add to queue and start processing
                        self.state.processing_state.input_queue.push(input.clone());
                        self.state.processing_state.is_processing = true;

                        // Add user message to output
                        let event = AppEvent::OutputLine(crate::state::OutputLine {
                            content: input.clone(),
                            style: crate::state::OutputStyle::UserMessage,
                            timestamp: chrono::Local::now(),
                            ..Default::default()
                        });
                        self.record_event(event);

                        if input.starts_with('/') {
                            // Command handling - commands complete immediately
                            match input.as_str() {
                                "/help" | "/h" => {
                                    self.record_event(AppEvent::OutputLine(crate::state::OutputLine {
                                        content: "Available commands: /help, /sessions, /tasks, /quit".to_string(),
                                        style: crate::state::OutputStyle::SystemInfo,
                                        timestamp: chrono::Local::now(),
                                    ..Default::default()
                                    }));
                                }
                                "/quit" | "/exit" => {}
                                _ => {
                                    self.record_event(AppEvent::OutputLine(
                                        crate::state::OutputLine {
                                            content: format!("Unknown command: {}", input),
                                            style: crate::state::OutputStyle::Error,
                                            timestamp: chrono::Local::now(),
                                            ..Default::default()
                                        },
                                    ));
                                }
                            }
                            // Commands complete immediately
                            self.state.processing_state.finish_processing();
                        } else {
                            // Non-command inputs are "processing"
                            self.record_event(AppEvent::StartProcessing);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn record_event(&mut self, event: AppEvent) {
        // Update internal state based on event
        self.state.update(&event);
        // Record the event
        self.sent_events.lock().unwrap().push(event);
    }

    /// Add a session for testing popup selection
    pub fn add_session(&mut self, id: &str, name: &str) {
        self.state.sessions.push(crate::state::SessionListItem {
            id: id.to_string(),
            name: name.to_string(),
            status: "Active".to_string(),
            created_at: chrono::Local::now(),
            message_count: 0,
        });
    }

    /// Clear all sessions
    pub fn clear_sessions(&mut self) {
        self.state.sessions.clear();
    }

    /// Simulate receiving an agent response
    pub fn simulate_agent_response(&mut self, content: &str) {
        // Stop processing when response comes back
        self.state.processing_state.finish_processing();

        let event = AppEvent::OutputLine(crate::state::OutputLine {
            content: content.to_string(),
            style: crate::state::OutputStyle::AgentMessage,
            timestamp: chrono::Local::now(),
            ..Default::default()
        });
        self.record_event(event);
    }

    /// Check if currently processing
    pub fn is_processing(&self) -> bool {
        self.state.processing_state.is_processing
    }

    /// Get queue size
    pub fn queued_input_count(&self) -> usize {
        self.state.processing_state.input_queue.len()
    }

    /// Finish current processing manually (simulates timeout or completion)
    pub fn finish_processing(&mut self) {
        self.state.processing_state.finish_processing();
    }
}

impl Default for TuiTestHarness {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Test Scenarios - Development-Time Verification
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_ascii_text() {
        let mut h = TuiTestHarness::new();
        h.type_text("hello");
        assert_eq!(h.input_buffer(), "hello");
        assert_eq!(h.cursor_position(), 5);
    }

    #[test]
    fn test_type_chinese_text() {
        let mut h = TuiTestHarness::new();
        h.type_text("你好");
        assert_eq!(h.input_buffer(), "你好");
        assert_eq!(h.cursor_position(), 2);
    }

    #[test]
    fn test_backspace() {
        let mut h = TuiTestHarness::new();
        h.type_text("hello");
        h.press_backspace();
        h.press_backspace();
        assert_eq!(h.input_buffer(), "hel");
        assert_eq!(h.cursor_position(), 3);
    }

    #[test]
    fn test_enter_submits_input() {
        let mut h = TuiTestHarness::new();
        h.type_text("hello");
        h.press_enter();

        assert!(h.output_contains("hello"));
        // Processing state is set, but the animation is rendered by the UI
        // based on is_processing flag, not an OutputLine
        assert!(h.is_processing());
        assert_eq!(h.input_buffer(), "");
        assert_eq!(h.cursor_position(), 0);
    }

    #[test]
    fn test_command_input() {
        let mut h = TuiTestHarness::new();
        h.type_text("/help");
        h.press_enter();

        assert!(h.output_contains("Available commands"));
        // Commands don't show "正在处理中"
        assert!(!h.output_contains("正在处理中"));
    }

    #[test]
    fn test_popup_toggle() {
        let mut h = TuiTestHarness::new();

        assert!(!h.has_popup(PopupType::SessionList));

        h.press_alt_s();
        assert!(h.has_popup(PopupType::SessionList));

        h.press_alt_t();
        assert!(!h.has_popup(PopupType::SessionList));
        assert!(h.has_popup(PopupType::TaskList));

        h.press_escape();
        assert!(h.active_popup().is_none());
    }

    #[test]
    fn test_popup_navigation() {
        let mut h = TuiTestHarness::new();
        h.add_session("1", "Session 1");
        h.add_session("2", "Session 2");
        h.add_session("3", "Session 3");

        h.press_alt_s();
        assert_eq!(h.popup_selected_index(), 0);

        h.press_down();
        assert_eq!(h.popup_selected_index(), 1);

        h.press_down();
        assert_eq!(h.popup_selected_index(), 2);

        h.press_up();
        assert_eq!(h.popup_selected_index(), 1);
    }

    #[test]
    fn test_cursor_navigation() {
        let mut h = TuiTestHarness::new();
        h.type_text("hello");

        assert_eq!(h.cursor_position(), 5);

        h.press_left();
        h.press_left();
        assert_eq!(h.cursor_position(), 3);

        h.press_right();
        assert_eq!(h.cursor_position(), 4);
    }

    #[test]
    fn test_agent_response() {
        let mut h = TuiTestHarness::new();
        h.simulate_agent_response("我正在处理你的请求...");
        assert!(h.output_contains("我正在处理你的请求"));
    }

    #[test]
    fn test_mixed_chinese_and_ascii() {
        let mut h = TuiTestHarness::new();
        h.type_text("hello你好world");
        assert_eq!(h.input_buffer(), "hello你好world");
        // 5 + 2 + 5 = 12 characters
        assert_eq!(h.cursor_position(), 12);
    }

    #[test]
    fn test_quit_command() {
        let mut h = TuiTestHarness::new();
        h.type_text("/quit");
        h.press_enter();

        // /quit is recognized but handled specially
        assert!(h.output_contains("/quit"));
    }

    #[test]
    fn test_input_output_consistency_ascii() {
        let mut h = TuiTestHarness::new();
        h.type_text("hello");
        h.press_enter();

        // Output should contain the exact input
        assert!(h.output_contains("hello"), "Output should contain 'hello'");
        // Should not contain any extra duplicates
        assert_eq!(
            h.output_lines()
                .iter()
                .filter(|l| l.content == "hello".to_string())
                .count(),
            1,
            "Should have exactly one 'hello' line"
        );
    }

    #[test]
    fn test_input_output_consistency_chinese() {
        let mut h = TuiTestHarness::new();
        h.type_text("你好世界");
        h.press_enter();

        // Output should contain the exact Chinese input
        assert!(
            h.output_contains("你好世界"),
            "Output should contain '你好世界'"
        );
        // Should not contain duplicated characters like "你好好世界世界"
        assert_eq!(
            h.output_lines()
                .iter()
                .filter(|l| l.content == "你好世界".to_string())
                .count(),
            1,
            "Should have exactly one '你好世界' line"
        );
    }

    #[test]
    fn test_input_output_consistency_mixed() {
        let mut h = TuiTestHarness::new();
        h.type_text("hello你好world");
        h.press_enter();

        // Output should contain exact input
        assert!(
            h.output_contains("hello你好world"),
            "Output should contain mixed text"
        );
        // Check no duplication - count occurrences of "hello你好world"
        assert_eq!(
            h.output_lines()
                .iter()
                .filter(|l| l.content == "hello你好world".to_string())
                .count(),
            1,
            "Should have exactly one 'hello你好world' line"
        );
    }

    #[test]
    fn test_no_character_duplication_in_output() {
        let mut h = TuiTestHarness::new();

        // Type text with potential problematic patterns
        h.type_text("aa");
        h.press_enter();
        assert!(h.output_contains("aa"));
        assert!(h.is_processing());

        // Second input is queued
        h.type_text("对对");
        h.press_enter();
        assert!(h.is_processing());
        assert_eq!(h.queued_input_count(), 2);

        // Verify "aa" is in output (first input)
        assert!(h.output_contains("aa"));
    }

    #[test]
    fn test_event_channel_integration() {
        // This test verifies that events flow through the channel correctly
        // and are received by the state
        use crate::state::OutputLine;
        use crate::state::OutputStyle;

        let mut h = TuiTestHarness::new();

        // Send an event directly through the channel
        let event = AppEvent::OutputLine(OutputLine {
            content: "Direct channel test".to_string(),
            style: OutputStyle::UserMessage,
            timestamp: chrono::Local::now(),
            ..Default::default()
        });

        // Send via event_tx (simulating what the TUI does)
        h.state.event_tx.send(event).unwrap();

        // Process events - in the real TUI this happens in the run() loop
        while let Ok(evt) = h.state.event_rx.try_recv() {
            h.state.update(&evt);
        }

        // Verify the event was processed
        assert!(h.output_contains("Direct channel test"));
    }

    #[test]
    fn test_multiple_inputs_and_outputs() {
        let mut h = TuiTestHarness::new();

        // First input
        h.type_text("first message");
        h.press_enter();
        assert!(h.output_contains("first message"));
        assert!(h.is_processing());

        // Second input while processing - goes to queue
        h.type_text("second message");
        h.press_enter();
        // Second input is queued while first is still processing
        assert!(h.is_processing());
        assert_eq!(h.queued_input_count(), 2);

        // First message should still be in output
        assert!(h.output_contains("first message"));
    }

    #[test]
    fn test_processing_state_queued_input() {
        let mut h = TuiTestHarness::new();

        // First input starts processing - it goes into the queue
        h.type_text("first");
        h.press_enter();
        assert!(h.is_processing());
        assert_eq!(h.queued_input_count(), 1, "First input is in the queue");

        // Second input while processing should queue (now 2 in queue)
        h.type_text("second");
        h.press_enter();
        assert!(h.is_processing());
        assert_eq!(h.queued_input_count(), 2, "Second input should be queued");

        // Third input while processing should queue (now 3 in queue)
        h.type_text("third");
        h.press_enter();
        assert_eq!(h.queued_input_count(), 3, "Third input should be queued");
    }

    #[test]
    fn test_processing_state_finish_processing() {
        let mut h = TuiTestHarness::new();

        // Start processing - first goes into queue
        h.type_text("first");
        h.press_enter();
        assert!(h.is_processing());
        assert_eq!(h.queued_input_count(), 1);

        // Queue another
        h.type_text("second");
        h.press_enter();
        assert_eq!(h.queued_input_count(), 2);

        // Finish processing - removes first, "second" is still queued
        h.finish_processing();
        assert!(h.is_processing());
        assert_eq!(
            h.queued_input_count(),
            1,
            "Second input should remain queued"
        );

        // Finish again - removes second, queue empty
        h.finish_processing();
        assert!(!h.is_processing());
        assert_eq!(h.queued_input_count(), 0);
    }

    #[test]
    fn test_agent_response_stops_processing() {
        let mut h = TuiTestHarness::new();

        // Start processing
        h.type_text("hello");
        h.press_enter();
        assert!(h.is_processing());

        // Simulate agent response
        h.simulate_agent_response("Hi! How can I help?");
        assert!(!h.is_processing());
    }
}
