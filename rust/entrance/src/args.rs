//! Command-line arguments for Knight Agent

use clap::{Parser, Subcommand};

/// Knight Agent - AI-powered development assistant
#[derive(Parser, Debug, Clone)]
#[command(name = "knight-agent")]
#[command(about = "Knight Agent - AI-powered development assistant", long_about = None)]
#[command(version)]
pub struct Args {
    /// Run in single-process mode (for development/testing)
    #[arg(long = "in-process")]
    pub in_process: bool,

    /// Subcommand to run
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Subcommands for Knight Agent
#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    /// Run the daemon process only
    Daemon {
        /// Port to listen on (default: 8080)
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },
    /// Run a session process
    Session {
        /// Session ID to connect to
        #[arg(long)]
        session_id: String,

        /// Daemon address to connect to
        #[arg(long)]
        daemon_addr: String,
    },
}

impl Args {
    /// Check if we should run in single-process mode (only when explicitly specified)
    pub fn is_in_process_mode(&self) -> bool {
        self.in_process
    }

    /// Check if we should run as daemon
    pub fn is_daemon_mode(&self) -> bool {
        matches!(self.command, Some(Command::Daemon { .. }))
    }

    /// Check if we should run as session
    pub fn is_session_mode(&self) -> bool {
        matches!(self.command, Some(Command::Session { .. }))
    }

    /// Check if we should run in IPC mode (default when no subcommand specified)
    pub fn is_ipc_mode(&self) -> bool {
        self.command.is_none()
    }
}
