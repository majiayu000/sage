//! Streaming Printer - Progressive text output
//!
//! Handles streaming text output with proper formatting and chunk handling.

use std::io::{self, Write};

/// Streaming printer for progressive text output
pub struct StreamingPrinter {
    /// Buffer of content already printed
    printed_len: usize,
    /// Whether we've started the stream
    started: bool,
}

impl StreamingPrinter {
    /// Create a new streaming printer
    pub fn new() -> Self {
        Self {
            printed_len: 0,
            started: false,
        }
    }

    /// Start the streaming output with a prefix
    pub fn start(&mut self) {
        if !self.started {
            // Print the ● prefix in bright white
            print!("\x1b[97m● \x1b[0m");
            io::stdout().flush().ok();
            self.started = true;
        }
    }

    /// Append a chunk to the stream
    pub fn append(&mut self, chunk: &str) {
        if !self.started {
            self.start();
        }

        // Print the chunk directly (no buffering, instant feedback)
        print!("{}", chunk);
        io::stdout().flush().ok();
        self.printed_len += chunk.len();
    }

    /// Finish the stream with a newline
    pub fn finish(&mut self) {
        if self.started {
            println!();
            self.started = false;
            self.printed_len = 0;
        }
    }

    /// Get the number of characters printed
    pub fn len(&self) -> usize {
        self.printed_len
    }

    /// Check if anything has been printed
    pub fn is_empty(&self) -> bool {
        self.printed_len == 0
    }
}

impl Default for StreamingPrinter {
    fn default() -> Self {
        Self::new()
    }
}

/// Print a complete assistant response (non-streaming)
pub fn print_assistant_response(text: &str) {
    // ● prefix in bright white
    println!("\x1b[97m● {}\x1b[0m", text);
}

/// Print a tool call in Claude Code style
pub fn print_tool_call(name: &str, args: Option<&str>) {
    if let Some(args) = args {
        println!("\x1b[35m● {}(\"{}\")\x1b[0m", name, args);
    } else {
        println!("\x1b[35m● {}\x1b[0m", name);
    }
}

/// Print a tool result with indentation
pub fn print_tool_result(result: &str, success: bool) {
    let color = if success { "245" } else { "196" }; // Gray for success, red for error
    // Truncate long results
    let display = if result.len() > 200 {
        format!("{}...", &result[..197])
    } else {
        result.to_string()
    };
    println!("\x1b[38;5;{}m  ⎿ {}\x1b[0m", color, display);
}

/// Print an error message
pub fn print_error(message: &str) {
    println!("\x1b[31m● Error: {}\x1b[0m", message);
}

/// Print a user message
pub fn print_user_message(text: &str) {
    println!("\x1b[33m\x1b[1m> \x1b[0m\x1b[97m{}\x1b[0m", text);
}

/// Print thinking block (collapsed summary)
pub fn print_thinking(text: &str) {
    let lines: Vec<&str> = text.lines().take(3).collect();
    let has_more = text.lines().count() > 3;

    println!("\x1b[35m● Thinking...\x1b[0m");
    for line in lines {
        println!("\x1b[35m\x1b[2m  {}\x1b[0m", line);
    }
    if has_more {
        println!("\x1b[38;5;245m  ...\x1b[0m");
    }
}
