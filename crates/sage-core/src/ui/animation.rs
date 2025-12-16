//! Animation management for user interface

use colored::*;
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;

/// Animation states for different operations
#[derive(Debug, Clone, PartialEq)]
pub enum AnimationState {
    Thinking,
    ExecutingTools,
    Processing,
    Idle,
}

/// Animation manager that handles all UI animations
pub struct AnimationManager {
    current_state: Arc<Mutex<AnimationState>>,
    is_running: Arc<AtomicBool>,
    current_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl AnimationManager {
    /// Create a new animation manager
    pub fn new() -> Self {
        Self {
            current_state: Arc::new(Mutex::new(AnimationState::Idle)),
            is_running: Arc::new(AtomicBool::new(false)),
            current_task: Arc::new(Mutex::new(None)),
        }
    }

    /// Start an animation with the given state and message
    pub async fn start_animation(&self, state: AnimationState, message: &str, color: &str) {
        // Stop any existing animation first
        self.stop_animation().await;

        // Update state
        *self.current_state.lock().await = state;
        self.is_running.store(true, Ordering::SeqCst);

        // Start new animation
        let is_running = self.is_running.clone();
        let message = message.to_string();
        let color = color.to_string();

        let task = tokio::spawn(async move {
            Self::run_animation(is_running, &message, &color).await;
        });

        *self.current_task.lock().await = Some(task);
    }

    /// Stop the current animation
    pub async fn stop_animation(&self) {
        // Signal stop
        self.is_running.store(false, Ordering::SeqCst);

        // Wait for current task to finish
        if let Some(task) = self.current_task.lock().await.take() {
            task.abort();
            // Give it a moment to clean up
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        // Clear the line
        print!("\r\x1b[K");
        let _ = std::io::stdout().flush();

        // Update state
        *self.current_state.lock().await = AnimationState::Idle;
    }

    /// Get the current animation state
    pub async fn current_state(&self) -> AnimationState {
        self.current_state.lock().await.clone()
    }

    /// Check if animation is currently running
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    /// Internal animation loop with enhanced visual effects
    async fn run_animation(is_running: Arc<AtomicBool>, message: &str, color: &str) {
        // Choose frame set based on message type
        let frames: &[&str] = if message.contains("Thinking") {
            &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]
        } else if message.contains("Executing") {
            &["●", "◐", "◑", "◒", "◓", "◔", "◕", "◖", "◗", "○"]
        } else {
            &["▶", "▷", "▶", "▷", "▶", "▷", "▶", "▷", "▶", "▷"]
        };

        let mut frame_index = 0;
        let start_time = std::time::Instant::now();

        while is_running.load(Ordering::SeqCst) {
            let frame = frames[frame_index];
            let elapsed = start_time.elapsed().as_secs_f64();

            // Enhanced message formatting with better timing display
            let message_with_timer = format!("{} ({:.1}s)", message, elapsed);

            // Enhanced color scheme with bold and bright colors
            let colored_message = match color {
                "blue" => format!("{} {}", frame, message_with_timer)
                    .bright_blue()
                    .bold(),
                "green" => format!("{} {}", frame, message_with_timer)
                    .bright_green()
                    .bold(),
                "yellow" => format!("{} {}", frame, message_with_timer)
                    .bright_yellow()
                    .bold(),
                "red" => format!("{} {}", frame, message_with_timer)
                    .bright_red()
                    .bold(),
                "cyan" => format!("{} {}", frame, message_with_timer)
                    .bright_cyan()
                    .bold(),
                "magenta" => format!("{} {}", frame, message_with_timer)
                    .bright_magenta()
                    .bold(),
                _ => format!("{} {}", frame, message_with_timer)
                    .bright_white()
                    .bold(),
            };

            print!("\r{} ", colored_message);
            let _ = std::io::stdout().flush();

            frame_index = (frame_index + 1) % frames.len();

            // Slightly faster animation for more fluid feel
            tokio::time::sleep(tokio::time::Duration::from_millis(80)).await;
        }
    }
}

impl Default for AnimationManager {
    fn default() -> Self {
        Self::new()
    }
}
