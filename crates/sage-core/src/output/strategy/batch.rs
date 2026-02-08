//! Batch output strategy.

use super::OutputStrategy;
use crate::ui::Icons;
use colored::Colorize;
use std::io::{self, Write};
use std::sync::Mutex;

/// Batch output - collects all content and displays at the end
///
/// Useful for:
/// - Environments where streaming isn't supported
/// - When you want to apply formatting (like markdown rendering) to complete content
/// - Logging or recording purposes
#[derive(Debug, Default)]
pub struct BatchOutput {
    content_buffer: Mutex<String>,
    tool_outputs: Mutex<Vec<String>>,
}

impl BatchOutput {
    pub fn new() -> Self {
        Self {
            content_buffer: Mutex::new(String::new()),
            tool_outputs: Mutex::new(Vec::new()),
        }
    }
}

impl OutputStrategy for BatchOutput {
    fn on_content_start(&self) {
        if let Ok(mut buffer) = self.content_buffer.lock() {
            buffer.clear();
        }
    }

    fn on_content_chunk(&self, chunk: &str) {
        if let Ok(mut buffer) = self.content_buffer.lock() {
            buffer.push_str(chunk);
        }
    }

    fn on_content_end(&self) {
        if let Ok(buffer) = self.content_buffer.lock() {
            if !buffer.is_empty() {
                println!();
                println!("{} {}", Icons::message().bright_white(), buffer);
            }
        }
    }

    fn on_tool_start(&self, name: &str, params: &str) {
        println!();
        print!(
            "{} {}",
            Icons::message().bright_blue(),
            name.bright_white().bold()
        );
        if !params.is_empty() {
            println!("({})", params.dimmed());
        } else {
            println!();
        }
    }

    fn on_tool_result(&self, success: bool, output: Option<&str>, error: Option<&str>) {
        if success {
            if let Some(out) = output {
                if let Ok(mut outputs) = self.tool_outputs.lock() {
                    outputs.push(out.to_string());
                }
                let preview: String = out.lines().take(3).collect::<Vec<_>>().join("\n    ");
                if !preview.trim().is_empty() {
                    let truncated = crate::utils::truncate_with_ellipsis(&preview, 200);
                    println!("  {} {}", Icons::result().dimmed(), truncated.dimmed());
                }
            }
        } else {
            let err_msg = error.unwrap_or("Unknown error");
            let first_line = err_msg.lines().next().unwrap_or(err_msg);
            let truncated = crate::utils::truncate_with_ellipsis(first_line, 80);
            println!("  {} {}", Icons::result().red(), truncated.red());
        }
        let _ = io::stdout().flush();
    }

    fn on_thinking(&self, message: &str) {
        println!("{} {}", Icons::cogitate().dimmed(), message.dimmed());
    }

    fn on_thinking_stop(&self) {}

    fn get_collected_content(&self) -> Option<String> {
        self.content_buffer.lock().ok().map(|b| b.clone())
    }
}
