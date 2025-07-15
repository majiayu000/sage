//! Progress display utilities with animations and effects

use colored::*;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

/// Spinner animation for thinking progress
#[allow(dead_code)] // May be used in future features
pub struct ThinkingSpinner {
    running: Arc<AtomicBool>,
    message: String,
}

#[allow(dead_code)] // May be used in future features
impl ThinkingSpinner {
    pub fn new(message: &str) -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            message: message.to_string(),
        }
    }

    /// Start the spinner animation
    pub async fn start(&self) {
        let running = self.running.clone();
        let message = self.message.clone();
        
        running.store(true, Ordering::Relaxed);
        
        tokio::spawn(async move {
            let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let mut frame_idx = 0;
            
            while running.load(Ordering::Relaxed) {
                print!("\r{} {} {}", 
                    frames[frame_idx].cyan().bold(),
                    message.blue().bold(),
                    "...".dimmed()
                );
                io::stdout().flush().unwrap();
                
                frame_idx = (frame_idx + 1) % frames.len();
                sleep(Duration::from_millis(100)).await;
            }
            
            // Clear the line
            print!("\r{}\r", " ".repeat(80));
            io::stdout().flush().unwrap();
        });
    }

    /// Stop the spinner animation
    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

/// Display tool execution with fancy effects
#[allow(dead_code)] // May be used in future features
pub fn show_tool_execution(tool_names: &[String]) {
    println!();
    println!("{}", "╭─────────────────────────────────────────╮".cyan());
    println!("{} {} {}", 
        "│".cyan(),
        "🔧 EXECUTING TOOLS".yellow().bold(),
        "│".cyan()
    );
    println!("{}", "├─────────────────────────────────────────┤".cyan());
    
    for (i, tool) in tool_names.iter().enumerate() {
        let icon = match tool.as_str() {
            "bash" => "🖥️ ",
            "str_replace_based_edit_tool" => "✏️ ",
            "json_edit_tool" => "📝",
            "task_done" => "✅",
            "sequentialthinking" => "🧠",
            _ => "🔧",
        };
        
        println!("{} {} {} {}", 
            "│".cyan(),
            format!("{}.", i + 1).dimmed(),
            format!("{} {}", icon, tool).green().bold(),
            "│".cyan()
        );
    }
    
    println!("{}", "╰─────────────────────────────────────────╯".cyan());
}

/// Display tool execution results with status
#[allow(dead_code)] // May be used in future features
pub fn show_tool_results(successful: usize, total: usize) {
    let status_icon = if successful == total {
        "✅".to_string()
    } else if successful > 0 {
        "⚠️ ".to_string()
    } else {
        "❌".to_string()
    };
    
    let status_text = if successful == total {
        format!("All {} tools completed successfully!", total).green().bold()
    } else {
        format!("{}/{} tools completed successfully", successful, total).yellow().bold()
    };
    
    println!();
    println!("{} {}", status_icon, status_text);
    println!("{}", "─".repeat(50).dimmed());
    println!();
}

/// Display AI response with fancy formatting
#[allow(dead_code)] // May be used in future features
pub fn show_ai_response(content: &str, step: u32, max_steps: u32) {
    println!();
    println!("{}", format!("╭─ 🤖 AI RESPONSE (Step {}/{}) ─╮", step, max_steps).magenta().bold());
    
    // Truncate content if too long
    let display_content = if content.len() > 300 {
        format!("{}...", &content[..297])
    } else {
        content.to_string()
    };
    
    // Split into lines and format
    for line in display_content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        println!("{} {}", "│".magenta(), line.white());
    }
    
    println!("{}", "╰─────────────────────────────────────────╯".magenta());
    println!();
}

/// Display step header with progress bar
#[allow(dead_code)] // May be used in future features
pub fn show_step_header(step: u32, max_steps: u32) {
    let progress = (step as f32 / max_steps as f32 * 20.0) as usize;
    let progress_bar = format!("{}{}",
        "█".repeat(progress).green(),
        "░".repeat(20 - progress).dimmed()
    );
    
    println!();
    println!("{}", "═".repeat(60).blue());
    println!("{} {} {} {}", 
        "🚀".to_string(),
        format!("STEP {}/{}", step, max_steps).blue().bold(),
        format!("[{}]", progress_bar),
        format!("{}%", (step as f32 / max_steps as f32 * 100.0) as u32).blue()
    );
    println!("{}", "═".repeat(60).blue());
}
