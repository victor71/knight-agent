//! Knight Agent TUI
//!
//! Terminal User Interface for Knight Agent.

mod app;
pub mod event;
mod layout;
mod renderer;
mod state;
pub mod widgets;

pub mod test_harness;

pub use app::{AppState, PopupType};
pub use event::{AppEvent, SystemStatusSnapshot};
pub use renderer::AppTerminal;
pub use state::{
    CompressionWarningLevel, ContextCompressionStatus, OutputLine, OutputStyle,
    ProjectInfo, SessionListItem, SessionTokenUsage, TaskInfo, TaskStatus,
};

use anyhow::Result;
use crossterm::event::{self as crossterm_event, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::info;
use widgets::*;

/// Main TUI application
pub struct TuiApp {
    state: AppState,
    terminal: AppTerminal,
    tick_rate: Duration,
}

impl TuiApp {
    /// Create a new TUI application
    pub fn new() -> Result<Self> {
        let terminal = AppTerminal::new()?;

        // Create a single event channel for both sending and receiving
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let state = AppState::new(event_tx, event_rx);

        // Get initial terminal size
        let size = terminal.size();
        let state = AppState {
            terminal_size: (size.width, size.height),
            ..state
        };

        Ok(Self {
            state,
            terminal,
            tick_rate: Duration::from_millis(16), // ~60 FPS
        })
    }

    /// Get the event sender for external use
    pub fn event_tx(&self) -> mpsc::UnboundedSender<AppEvent> {
        self.state.event_tx.clone()
    }

    /// Run the TUI application
    pub async fn run(&mut self) -> Result<()> {
        let mut last_tick = Instant::now();

        loop {
            // Render the UI
            self.terminal.draw(|f| {
                let layout = layout::calculate_main_layout(f.area());

                render_header(f, layout.header, &self.state);
                render_main_output(f, layout.main, &self.state);
                render_input(f, layout.input, &self.state);
                render_status(f, layout.status, &self.state);

                // Render popups if active
                if self.state.active_popup.is_some() {
                    render_session_popup(f, f.area(), &mut self.state);
                    render_task_popup(f, f.area(), &mut self.state);
                }
            })?;

            // Handle events with timeout
            let timeout = self.tick_rate.saturating_sub(last_tick.elapsed());
            if crossterm_event::poll(timeout)? {
                if let Event::Key(key) = crossterm_event::read()? {
                    // On Windows, crossterm sends both Press and Release events.
                    // Only handle Press to avoid processing each key twice.
                    if key.kind == KeyEventKind::Press {
                        self.handle_key_event(key)?;
                    }
                }
            }

            // Send tick event
            if last_tick.elapsed() >= self.tick_rate {
                let _ = self.state.event_tx.send(AppEvent::Tick);
                last_tick = Instant::now();
            }

            // Process pending events from the same channel
            while let Ok(event) = self.state.event_rx.try_recv() {
                self.state.update(&event);

                // Check for exit condition
                if matches!(event, AppEvent::SessionSwitch(_) | AppEvent::TaskComplete(_)) {
                    // Could trigger refresh here
                }
            }

            // Check exit condition
            if self.state.input_buffer == "/quit" {
                break;
            }

            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        Ok(())
    }

    /// Handle a key event
    fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        match (key.code, key.modifiers) {
            // Alt+N: Create new session
            (KeyCode::Char('n'), KeyModifiers::ALT) => {
                self.state.event_tx.send(AppEvent::SessionListUpdate(vec![]))?;
            }
            // Alt+S: Open session switcher
            (KeyCode::Char('s'), KeyModifiers::ALT) => {
                self.state.toggle_popup(PopupType::SessionList);
            }
            // Alt+T: Open task list
            (KeyCode::Char('t'), KeyModifiers::ALT) => {
                self.state.toggle_popup(PopupType::TaskList);
            }
            // Ctrl+Q or /quit command: Quit
            (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                self.state.input_buffer = "/quit".to_string();
            }
            // Popup navigation
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
            (KeyCode::Enter, KeyModifiers::NONE) => {
                if let Some(popup) = self.state.active_popup {
                    match popup {
                        PopupType::SessionList => {
                            if let Some(session) = self.state.sessions.get(self.state.popup_selected_index) {
                                self.state.event_tx.send(AppEvent::SessionSwitch(session.id.clone()))?;
                            }
                        }
                        PopupType::TaskList => {
                            // Task selection could go here
                        }
                    }
                    self.state.close_popup();
                } else {
                    // Submit input
                    if !self.state.input_buffer.is_empty() {
                        let input = self.state.input_buffer.clone();
                        info!("TUI: User submitted input: \"{}\"", input);
                        self.state.input_buffer.clear();
                        self.state.cursor_position = 0;

                        if self.state.processing_state.is_processing {
                            // Already processing - queue the input
                            info!("TUI: Currently processing, queueing input. Queue size: {}", self.state.processing_state.input_queue.len() + 1);
                            self.state.processing_state.input_queue.push(input);
                        } else {
                            // Start processing - add to queue and start processing
                            self.state.processing_state.input_queue.push(input.clone());
                            self.state.processing_state.is_processing = true;
                            info!("TUI: Starting processing for input: \"{}\"", input);

                            // Add user message to output
                            self.state.event_tx.send(AppEvent::OutputLine(
                                crate::state::OutputLine {
                                    content: input.clone(),
                                    style: crate::state::OutputStyle::UserMessage,
                                    timestamp: chrono::Local::now(),
                                },
                            ))?;

                            // Process command or just start processing
                            if input.starts_with('/') {
                                self.handle_command(&input)?;
                                // Commands complete immediately
                                self.state.processing_state.finish_processing();
                            } else {
                                // Non-command inputs are "processing" - the animation will show
                                self.state.event_tx.send(AppEvent::StartProcessing)?;
                            }
                        }
                    }
                }
            }
            // Close popup
            (KeyCode::Esc, KeyModifiers::NONE) => {
                self.state.close_popup();
            }
            // Text input - insert at cursor position
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
            _ => {}
        }
        Ok(())
    }

    /// Handle a command
    fn handle_command(&self, command: &str) -> Result<()> {
        match command {
            "/help" | "/h" => {
                self.state.event_tx.send(AppEvent::OutputLine(
                    crate::state::OutputLine {
                        content: "Available commands: /help, /sessions, /tasks, /quit".to_string(),
                        style: crate::state::OutputStyle::SystemInfo,
                        timestamp: chrono::Local::now(),
                    },
                ))?;
            }
            "/sessions" => {
                // Refresh session list
                self.state.event_tx.send(AppEvent::SessionListUpdate(
                    self.state.sessions.clone(),
                ))?;
            }
            "/tasks" => {
                // Refresh task list
                self.state.event_tx.send(AppEvent::TaskListUpdate(
                    self.state.tasks.clone(),
                ))?;
            }
            "/quit" | "/exit" => {
                // Will be handled in main loop
            }
            _ => {
                self.state.event_tx.send(AppEvent::OutputLine(
                    crate::state::OutputLine {
                        content: format!("Unknown command: {}", command),
                        style: crate::state::OutputStyle::Error,
                        timestamp: chrono::Local::now(),
                    },
                ))?;
            }
        }
        Ok(())
    }
}

/// Run the TUI application
pub async fn run_tui(initial_status: Option<SystemStatusSnapshot>) -> Result<()> {
    let mut app = TuiApp::new()?;

    // Send initial system status if provided
    if let Some(status) = initial_status {
        let _ = app.state.event_tx.send(AppEvent::SystemStatusUpdate(status));
    }

    app.run().await?;
    Ok(())
}
