//! TUI Terminal Renderer
//!
//! Wraps the ratatui Terminal and handles terminal state.

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Stdout};
use std::panic;

/// Application terminal wrapper
pub struct AppTerminal {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl AppTerminal {
    /// Create a new terminal
    pub fn new() -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        // Set up panic handler to restore terminal
        let original_hook = panic::take_hook();
        panic::set_hook(Box::new(move |panic_info| {
            let _ = disable_raw_mode();
            let _ = execute!(io::stdout(), LeaveAlternateScreen);
            original_hook(panic_info);
        }));

        Ok(Self { terminal })
    }

    /// Draw to the terminal
    pub fn draw<F>(&mut self, f: F) -> Result<()>
    where
        F: FnOnce(&mut ratatui::Frame),
    {
        self.terminal.draw(f)?;
        Ok(())
    }

    /// Get terminal size
    pub fn size(&self) -> ratatui::layout::Size {
        self.terminal.size().unwrap_or(ratatui::layout::Size {
            width: 80,
            height: 24,
        })
    }

    /// Clear the terminal
    pub fn clear(&mut self) -> Result<()> {
        self.terminal.clear()?;
        Ok(())
    }

    /// Restore terminal state (called on drop)
    pub fn restore(&mut self) -> Result<()> {
        disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen)?;
        Ok(())
    }
}

impl Drop for AppTerminal {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}
