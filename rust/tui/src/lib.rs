//! Knight Agent TUI
//!
//! Terminal User Interface for Knight Agent.

mod app;
pub mod client;
pub mod event;
mod layout;
mod renderer;
mod state;
pub mod widgets;

pub mod test_harness;

pub use app::{AppState, PopupType};
pub use client::{DaemonClient, DirectDaemonClient, DaemonClientError, DaemonClientResult};
pub use client::ipc::IpcDaemonClient;
pub use router::HandleInputResult;
pub use event::{AppEvent, SystemStatusSnapshot};
pub use renderer::AppTerminal;
pub use state::{
    CompressionWarningLevel, ContextCompressionStatus, OutputLine, OutputStyle,
    ProjectInfo, SessionListItem, SessionTokenUsage, TaskInfo, TaskStatus,
};

use anyhow::Result;
use crossterm::event::{self as crossterm_event, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{info, warn, debug};
use widgets::*;

/// Main TUI application
pub struct TuiApp {
    state: AppState,
    terminal: AppTerminal,
    tick_rate: Duration,
    /// Daemon client for communication
    daemon_client: Option<Arc<dyn DaemonClient>>,
    /// Current session ID
    session_id: String,
    /// Pending agent input to process (set by sync event handler, processed by async loop)
    pending_agent_input: Option<String>,
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
            daemon_client: None,
            session_id: "default".to_string(),
            pending_agent_input: None,
        })
    }

    /// Set the daemon client
    pub fn with_daemon_client(mut self, daemon_client: Arc<dyn DaemonClient>) -> Self {
        self.daemon_client = Some(daemon_client);
        self
    }

    /// Set the session ID
    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.session_id = session_id;
        self
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
                if matches!(event, AppEvent::Exit) {
                    info!("Exit event received, shutting down daemon...");
                    // Try to shutdown daemon gracefully
                    if let Some(ref client) = self.daemon_client {
                        let client = client.clone();
                        tokio::spawn(async move {
                            if let Err(e) = client.shutdown().await {
                                warn!("Daemon shutdown error: {:?}", e);
                            } else {
                                info!("Daemon shutdown complete");
                            }
                        });
                    }
                    break;
                }
                if matches!(event, AppEvent::SessionSwitch(_) | AppEvent::TaskComplete(_)) {
                    // Could trigger refresh here
                }
            }

            // Process pending agent input if any (spawn as background task to avoid blocking)
            if let Some(input) = self.pending_agent_input.take() {
                // Take ownership of what we need for the spawned task
                let daemon_client = self.daemon_client.clone();
                let session_id = self.session_id.clone();
                let event_tx = self.state.event_tx.clone();

                // Spawn background task so event loop can continue processing/rendering
                tokio::spawn(async move {
                    if let Err(e) = Self::route_to_agent_bg(daemon_client, session_id, input, event_tx).await {
                        warn!("route_to_agent error: {:?}", e);
                    }
                });
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

                        // Check for quit command BEFORE processing
                        if input == "/quit" || input == "/exit" {
                            info!("TUI: Quit command received, exiting...");
                            // Return a special error to signal quit
                            return Err(anyhow::anyhow!("Quit command"));
                        }

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
                                    ..Default::default()
                                },
                            ))?;

                            // Process command or route to agent
                            if input.starts_with('/') {
                                self.handle_command(&input)?;
                                // Commands complete immediately
                                self.state.processing_state.finish_processing();
                            } else {
                                // Non-command inputs - route to agent via router/agent_runtime
                                self.state.event_tx.send(AppEvent::StartProcessing)?;
                                // Set pending agent input - will be processed in the async event loop
                                self.pending_agent_input = Some(input);
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
            // Accept any character regardless of modifiers (handles Caps Lock/Shift)
            (KeyCode::Char(c), _) => {
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

    /// Route non-command input to agent via daemon client
    async fn route_to_agent(&mut self, input: String) -> Result<()> {
        if let Some(ref daemon_client) = self.daemon_client {
            info!("[DEBUG] route_to_agent: session_id={}, input={}", self.session_id, input);
            let result = daemon_client.handle_input(input.clone(), self.session_id.clone()).await;

            match result {
                Ok(result) => {
                    info!("Daemon client result: to_agent={}", result.to_agent);

                    if result.to_agent {
                        // Create streaming callback that sends StreamChunk events
                        let event_tx_clone = self.state.event_tx.clone();
                        let stream_callback: Box<dyn Fn(String) -> bool + Send + Sync> = Box::new(move |chunk: String| -> bool {
                            // Send stream chunk to TUI
                            let _ = event_tx_clone.send(AppEvent::StreamChunk(chunk));
                            true  // Continue streaming
                        });

                        // Forward to agent - use daemon client's send_message_streaming
                        match daemon_client.send_message_streaming(&self.session_id, input, Some(stream_callback)).await {
                            Ok(response) => {
                                info!("Agent response received: \"{}\"", response);
                                // Send complete agent response to output (for final display)
                                self.state.event_tx.send(AppEvent::OutputLine(
                                    crate::state::OutputLine {
                                        content: response,
                                        style: crate::state::OutputStyle::AgentMessage,
                                        timestamp: chrono::Local::now(),
                                    ..Default::default()
                                    },
                                ))?;
                            }
                            Err(e) => {
                                warn!("Daemon client send_message error: {:?}", e);
                                self.state.event_tx.send(AppEvent::OutputLine(
                                    crate::state::OutputLine {
                                        content: format!("Error: {:?}", e),
                                        style: crate::state::OutputStyle::Error,
                                        timestamp: chrono::Local::now(),
                                    ..Default::default()
                                    },
                                ))?;
                            }
                        }
                    } else {
                        // Daemon client handled it - show response if any
                        if !result.response.message.is_empty() {
                            self.state.event_tx.send(AppEvent::OutputLine(
                                crate::state::OutputLine {
                                    content: result.response.message,
                                    style: crate::state::OutputStyle::SystemInfo,
                                    timestamp: chrono::Local::now(),
                                    ..Default::default()
                                },
                            ))?;
                        }
                    }
                }
                Err(e) => {
                    warn!("Daemon client error: {:?}", e);
                    self.state.event_tx.send(AppEvent::OutputLine(
                        crate::state::OutputLine {
                            content: format!("Error: {:?}", e),
                            style: crate::state::OutputStyle::Error,
                            timestamp: chrono::Local::now(),
                                    ..Default::default()
                        },
                    ))?;
                }
            }
        } else {
            warn!("No daemon client configured");
            self.state.event_tx.send(AppEvent::OutputLine(
                crate::state::OutputLine {
                    content: "No daemon client configured".to_string(),
                    style: crate::state::OutputStyle::Error,
                    timestamp: chrono::Local::now(),
                                    ..Default::default()
                },
            ))?;
        }

        // Stop processing after handling
        self.state.event_tx.send(AppEvent::StopProcessing)?;
        self.state.processing_state.finish_processing();
        Ok(())
    }

    /// Route non-command input to agent via daemon client (background task version)
    async fn route_to_agent_bg(
        daemon_client: Option<Arc<dyn DaemonClient>>,
        session_id: String,
        input: String,
        event_tx: mpsc::UnboundedSender<AppEvent>,
    ) -> Result<()> {
        if let Some(ref client) = daemon_client {
            info!("[DEBUG] route_to_agent_bg: session_id={}, input={}", session_id, input);
            let result = client.handle_input(input.clone(), session_id.clone()).await;

            match result {
                Ok(result) => {
                    info!("Daemon client result: to_agent={}, should_exit={}", result.to_agent, result.should_exit);

                    // Check if command signals exit (e.g., /quit)
                    if result.should_exit {
                        let _ = event_tx.send(AppEvent::Exit);
                        let _ = event_tx.send(AppEvent::StopProcessing);
                        return Ok(());
                    }

                    if result.to_agent {
                        // Create streaming callback that sends StreamChunk events
                        let event_tx_clone = event_tx.clone();
                        let stream_callback: Box<dyn Fn(String) -> bool + Send + Sync> = Box::new(move |chunk: String| -> bool {
                            // Send stream chunk to TUI
                            let _ = event_tx_clone.send(AppEvent::StreamChunk(chunk));
                            true  // Continue streaming
                        });

                        // Forward to agent - use daemon client's send_message_streaming
                        match client.send_message_streaming(&session_id, input, Some(stream_callback)).await {
                            Ok(response) => {
                                info!("Agent response received: \"{}\"", response);
                                // Note: Don't send OutputLine here since streaming already displayed chunks
                                // The stream callback already sent all chunks via StreamChunk events
                                // Just log that streaming completed
                                debug!("Streaming completed, final response length: {}", response.len());
                            }
                            Err(e) => {
                                warn!("Daemon client send_message error: {:?}", e);
                                let _ = event_tx.send(AppEvent::OutputLine(
                                    crate::state::OutputLine {
                                        content: format!("Error: {:?}", e),
                                        style: crate::state::OutputStyle::Error,
                                        timestamp: chrono::Local::now(),
                                    ..Default::default()
                                    },
                                ));
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Daemon client error: {:?}", e);
                    let _ = event_tx.send(AppEvent::OutputLine(
                        crate::state::OutputLine {
                            content: format!("Error: {:?}", e),
                            style: crate::state::OutputStyle::Error,
                            timestamp: chrono::Local::now(),
                                    ..Default::default()
                        },
                    ));
                }
            }
        } else {
            warn!("No daemon client configured");
            let _ = event_tx.send(AppEvent::OutputLine(
                crate::state::OutputLine {
                    content: "No daemon client configured".to_string(),
                    style: crate::state::OutputStyle::Error,
                    timestamp: chrono::Local::now(),
                                    ..Default::default()
                },
            ));
        }

        // Stop processing after handling
        let _ = event_tx.send(AppEvent::StopProcessing);
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
                                    ..Default::default()
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
                                    ..Default::default()
                    },
                ))?;
            }
        }
        Ok(())
    }
}

/// Run the TUI application
pub async fn run_tui(
    initial_status: Option<SystemStatusSnapshot>,
    daemon_client: Option<Arc<dyn DaemonClient>>,
    session_id: Option<String>,
) -> Result<()> {
    let mut app = TuiApp::new()?;

    // Configure daemon client if provided
    if let Some(d) = daemon_client {
        app = app.with_daemon_client(d);
    }
    if let Some(s) = session_id {
        info!("[DEBUG] Setting session_id to: {}", s);
        app = app.with_session_id(s);
    } else {
        info!("[DEBUG] session_id is None, using default");
    }

    // Send initial system status if provided
    if let Some(status) = initial_status {
        let _ = app.state.event_tx.send(AppEvent::SystemStatusUpdate(status));
    }

    app.run().await?;
    Ok(())
}
