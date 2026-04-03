//! Log Output
//!
//! Defines the output destinations for logs.

use std::path::PathBuf;

/// Log output destination
pub enum LogOutput {
    /// Console output with optional colored formatting
    Console {
        colored: bool,
    },
    /// File output with rotation support
    File {
        path: PathBuf,
        rotation_max_size: usize,
        rotation_max_files: usize,
        compress: bool,
    },
}

impl LogOutput {
    /// Create a new console output
    pub fn console(colored: bool) -> Self {
        Self::Console { colored }
    }

    /// Create a new file output
    pub fn file(path: PathBuf, max_size: usize, max_files: usize, compress: bool) -> Self {
        Self::File {
            path,
            rotation_max_size: max_size,
            rotation_max_files: max_files,
            compress,
        }
    }
}
