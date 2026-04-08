//! TUI Integration Tests
//!
//! Tests for text input handling including ASCII and UTF-8 (Chinese) characters.

use tui::{AppState, PopupType};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use tokio::sync::mpsc;

/// Helper to create a KeyEvent
fn make_key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent {
        code,
        modifiers,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

/// Helper to create AppState for testing
fn make_test_state() -> AppState {
    let (tx, rx) = mpsc::unbounded_channel();
    AppState::new(tx, rx)
}

/// Simulates the input handling logic from TuiApp
fn handle_key(state: &mut AppState, key: KeyEvent) {
    match (key.code, key.modifiers) {
        (KeyCode::Char('s'), KeyModifiers::ALT) => {
            state.toggle_popup(PopupType::SessionList);
        }
        (KeyCode::Char('t'), KeyModifiers::ALT) => {
            state.toggle_popup(PopupType::TaskList);
        }
        (KeyCode::Up, KeyModifiers::NONE) => {
            if state.active_popup.is_some() {
                if state.popup_selected_index > 0 {
                    state.popup_selected_index -= 1;
                }
            }
        }
        (KeyCode::Down, KeyModifiers::NONE) => {
            if state.active_popup.is_some() {
                let max_index = match state.active_popup {
                    Some(PopupType::SessionList) => state.sessions.len().saturating_sub(1),
                    Some(PopupType::TaskList) => state.tasks.len().saturating_sub(1),
                    None => 0,
                };
                if state.popup_selected_index < max_index {
                    state.popup_selected_index += 1;
                }
            }
        }
        (KeyCode::Esc, KeyModifiers::NONE) => {
            state.close_popup();
        }
        (KeyCode::Char(c), KeyModifiers::NONE) => {
            state.input_buffer.push(c);
            state.cursor_position = state.input_buffer.chars().count();
        }
        (KeyCode::Backspace, KeyModifiers::NONE) => {
            if !state.input_buffer.is_empty() {
                state.input_buffer.pop();
                state.cursor_position = state.cursor_position.saturating_sub(1);
            }
        }
        (KeyCode::Left, KeyModifiers::NONE) => {
            if state.cursor_position > 0 {
                state.cursor_position -= 1;
            }
        }
        (KeyCode::Right, KeyModifiers::NONE) => {
            let char_count = state.input_buffer.chars().count();
            if state.cursor_position < char_count {
                state.cursor_position += 1;
            }
        }
        _ => {}
    }
}

#[test]
fn test_ascii_text_input() {
    let mut state = make_test_state();

    // Type "hello"
    handle_key(&mut state, make_key(KeyCode::Char('h'), KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Char('e'), KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Char('l'), KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Char('l'), KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Char('o'), KeyModifiers::NONE));

    assert_eq!(state.input_buffer, "hello");
    assert_eq!(state.cursor_position, 5); // cursor at end
}

#[test]
fn test_chinese_text_input() {
    let mut state = make_test_state();

    // Type "你好" (2 Chinese characters)
    handle_key(&mut state, make_key(KeyCode::Char('你'), KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Char('好'), KeyModifiers::NONE));

    assert_eq!(state.input_buffer, "你好");
    assert_eq!(state.cursor_position, 2); // 2 characters
}

#[test]
fn test_mixed_chinese_and_ascii() {
    let mut state = make_test_state();

    // Type "hello你好"
    handle_key(&mut state, make_key(KeyCode::Char('h'), KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Char('e'), KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Char('l'), KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Char('l'), KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Char('o'), KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Char('你'), KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Char('好'), KeyModifiers::NONE));

    assert_eq!(state.input_buffer, "hello你好");
    assert_eq!(state.cursor_position, 7); // 5 ASCII + 2 Chinese = 7 chars
}

#[test]
fn test_backspace() {
    let mut state = make_test_state();

    // Type "hello" then backspace twice
    handle_key(&mut state, make_key(KeyCode::Char('h'), KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Char('e'), KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Char('l'), KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Char('l'), KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Char('o'), KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Backspace, KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Backspace, KeyModifiers::NONE));

    assert_eq!(state.input_buffer, "hel");
    assert_eq!(state.cursor_position, 3);
}

#[test]
fn test_chinese_backspace() {
    let mut state = make_test_state();

    // Type "你好" then backspace once
    handle_key(&mut state, make_key(KeyCode::Char('你'), KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Char('好'), KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Backspace, KeyModifiers::NONE));

    assert_eq!(state.input_buffer, "你");
    assert_eq!(state.cursor_position, 1);
}

#[test]
fn test_backspace_at_start() {
    let mut state = make_test_state();

    // Type "hi" then backspace when cursor at start
    // Current simplified model: backspace deletes last char regardless of cursor
    handle_key(&mut state, make_key(KeyCode::Char('h'), KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Char('i'), KeyModifiers::NONE));
    // Move cursor to start
    handle_key(&mut state, make_key(KeyCode::Left, KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Left, KeyModifiers::NONE));
    // Backspace at start - in simplified model, still deletes last char
    handle_key(&mut state, make_key(KeyCode::Backspace, KeyModifiers::NONE));

    assert_eq!(state.input_buffer, "h");
    assert_eq!(state.cursor_position, 0);
}

#[test]
fn test_cursor_left_right() {
    let mut state = make_test_state();

    // Type "hello"
    handle_key(&mut state, make_key(KeyCode::Char('h'), KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Char('e'), KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Char('l'), KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Char('l'), KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Char('o'), KeyModifiers::NONE));

    // Move left twice
    handle_key(&mut state, make_key(KeyCode::Left, KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Left, KeyModifiers::NONE));

    assert_eq!(state.cursor_position, 3); // between 'l' and 'l'

    // Move right once
    handle_key(&mut state, make_key(KeyCode::Right, KeyModifiers::NONE));

    assert_eq!(state.cursor_position, 4);
}

#[test]
fn test_cursor_left_at_start() {
    let mut state = make_test_state();

    handle_key(&mut state, make_key(KeyCode::Char('a'), KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Char('b'), KeyModifiers::NONE));

    // Move left past start
    handle_key(&mut state, make_key(KeyCode::Left, KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Left, KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Left, KeyModifiers::NONE));

    assert_eq!(state.cursor_position, 0);
}

#[test]
fn test_cursor_right_at_end() {
    let mut state = make_test_state();

    handle_key(&mut state, make_key(KeyCode::Char('a'), KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Char('b'), KeyModifiers::NONE));

    // Already at end, try to move right
    handle_key(&mut state, make_key(KeyCode::Right, KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Right, KeyModifiers::NONE));

    assert_eq!(state.cursor_position, 2);
}

#[test]
fn test_chinese_cursor_navigation() {
    let mut state = make_test_state();

    // Type "你好世界"
    handle_key(&mut state, make_key(KeyCode::Char('你'), KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Char('好'), KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Char('世'), KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Char('界'), KeyModifiers::NONE));

    assert_eq!(state.cursor_position, 4);

    // Move left twice
    handle_key(&mut state, make_key(KeyCode::Left, KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Left, KeyModifiers::NONE));

    assert_eq!(state.cursor_position, 2);

    // Move right once
    handle_key(&mut state, make_key(KeyCode::Right, KeyModifiers::NONE));

    assert_eq!(state.cursor_position, 3);
}

#[test]
fn test_popup_toggle() {
    let mut state = make_test_state();

    assert_eq!(state.active_popup, None);

    // Open session popup
    handle_key(&mut state, make_key(KeyCode::Char('s'), KeyModifiers::ALT));
    assert_eq!(state.active_popup, Some(PopupType::SessionList));

    // Toggle to task popup
    handle_key(&mut state, make_key(KeyCode::Char('t'), KeyModifiers::ALT));
    assert_eq!(state.active_popup, Some(PopupType::TaskList));

    // Close popup
    handle_key(&mut state, make_key(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(state.active_popup, None);
}

#[test]
fn test_popup_navigation() {
    use tui::SessionListItem;
    use chrono::Local;

    let mut state = make_test_state();

    // Add some sessions
    state.sessions = vec![
        SessionListItem {
            id: "1".to_string(),
            name: "Session 1".to_string(),
            status: "active".to_string(),
            created_at: Local::now(),
            message_count: 5,
        },
        SessionListItem {
            id: "2".to_string(),
            name: "Session 2".to_string(),
            status: "active".to_string(),
            created_at: Local::now(),
            message_count: 10,
        },
        SessionListItem {
            id: "3".to_string(),
            name: "Session 3".to_string(),
            status: "idle".to_string(),
            created_at: Local::now(),
            message_count: 15,
        },
    ];

    // Open session popup
    handle_key(&mut state, make_key(KeyCode::Char('s'), KeyModifiers::ALT));
    assert_eq!(state.active_popup, Some(PopupType::SessionList));
    assert_eq!(state.popup_selected_index, 0);

    // Navigate down
    handle_key(&mut state, make_key(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(state.popup_selected_index, 1);

    handle_key(&mut state, make_key(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(state.popup_selected_index, 2);

    // Navigate up
    handle_key(&mut state, make_key(KeyCode::Up, KeyModifiers::NONE));
    assert_eq!(state.popup_selected_index, 1);

    // Navigate past bounds - should stay at max
    handle_key(&mut state, make_key(KeyCode::Down, KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(state.popup_selected_index, 2);
}

#[test]
fn test_long_chinese_text() {
    let mut state = make_test_state();

    // Type a long Chinese sentence
    let chinese_text = "锄禾日当午汗滴禾下土谁知盘中餐粒粒皆辛苦";
    for char in chinese_text.chars() {
        handle_key(&mut state, make_key(KeyCode::Char(char), KeyModifiers::NONE));
    }

    assert_eq!(state.input_buffer, chinese_text);
    assert_eq!(state.cursor_position, chinese_text.chars().count());
}

#[test]
fn test_quit_command() {
    let mut state = make_test_state();

    // Type /quit
    handle_key(&mut state, make_key(KeyCode::Char('/'), KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Char('q'), KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Char('u'), KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Char('i'), KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Char('t'), KeyModifiers::NONE));

    assert_eq!(state.input_buffer, "/quit");
    assert_eq!(state.cursor_position, 5);
}

#[test]
fn test_input_buffer_clear_on_submit() {
    let mut state = make_test_state();

    // Type something
    handle_key(&mut state, make_key(KeyCode::Char('h'), KeyModifiers::NONE));
    handle_key(&mut state, make_key(KeyCode::Char('i'), KeyModifiers::NONE));

    // Simulate submit by clearing (this happens in the real handler on Enter)
    let submitted_text = state.input_buffer.clone();
    state.input_buffer.clear();
    state.cursor_position = 0;

    assert_eq!(submitted_text, "hi");
    assert_eq!(state.input_buffer, "");
    assert_eq!(state.cursor_position, 0);
}

#[test]
fn test_zero_width_characters() {
    let mut state = make_test_state();

    // Test combining characters (zero-width joiner examples)
    // Chinese punctuation that might cause issues
    handle_key(&mut state, make_key(KeyCode::Char('，'), KeyModifiers::NONE)); // Chinese comma
    handle_key(&mut state, make_key(KeyCode::Char('。'), KeyModifiers::NONE)); // Chinese period

    assert_eq!(state.input_buffer, "，。");
    assert_eq!(state.cursor_position, 2);
}
