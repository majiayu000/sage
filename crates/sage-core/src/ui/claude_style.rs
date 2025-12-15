//! Claude Code style display components
//!
//! Provides display formatting similar to Claude Code's terminal UI.

use std::io::{self, Write};

/// Claude Code style display formatter
#[derive(Debug, Default)]
pub struct ClaudeStyleDisplay {
    /// Whether to use colors
    #[allow(dead_code)]
    use_colors: bool,
}

impl ClaudeStyleDisplay {
    /// Create a new display instance
    pub fn new() -> Self {
        Self { use_colors: true }
    }

    /// Create a display without colors
    pub fn without_colors() -> Self {
        Self { use_colors: false }
    }

    /// Print a text delta (streaming content)
    pub fn print_delta(&self, delta: &str) -> io::Result<()> {
        print!("{}", delta);
        io::stdout().flush()
    }

    /// Print a complete message
    pub fn print_message(&self, message: &str) {
        println!("{}", message);
    }
}

/// Response formatter for consistent output styling
#[derive(Debug, Default)]
pub struct ResponseFormatter {
    buffer: String,
}

impl ResponseFormatter {
    /// Create a new formatter
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// Append content to the buffer
    pub fn append(&mut self, content: &str) {
        self.buffer.push_str(content);
    }

    /// Get the formatted output
    pub fn output(&self) -> &str {
        &self.buffer
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

/// Simple progress indicator for long-running operations
#[derive(Debug)]
pub struct SimpleProgressIndicator {
    message: String,
    active: bool,
}

impl SimpleProgressIndicator {
    /// Create a new progress indicator
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            active: false,
        }
    }

    /// Start showing progress
    pub fn start(&mut self) {
        self.active = true;
        print!("{}", self.message);
        let _ = io::stdout().flush();
    }

    /// Update progress message
    pub fn update(&mut self, message: impl Into<String>) {
        self.message = message.into();
        if self.active {
            print!("\r{}", self.message);
            let _ = io::stdout().flush();
        }
    }

    /// Stop progress indicator
    pub fn stop(&mut self) {
        if self.active {
            println!();
            self.active = false;
        }
    }

    /// Check if active
    pub fn is_active(&self) -> bool {
        self.active
    }
}

impl Default for SimpleProgressIndicator {
    fn default() -> Self {
        Self::new("")
    }
}
