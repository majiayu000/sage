//! Animation management for user interface
//!
//! Unified progress display system that shows:
//! - Current operation with spinner animation
//! - Elapsed time
//! - Context info (step number, tool details, etc.)

use colored::*;
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use tokio::sync::RwLock;

/// Animation states for different operations
#[derive(Debug, Clone, PartialEq)]
pub enum AnimationState {
    Thinking,
    ExecutingTools,
    Processing,
    Idle,
}

/// Context information for richer animation display
#[derive(Debug, Clone, Default)]
pub struct AnimationContext {
    /// Current step number (e.g., 3)
    pub step: Option<u32>,
    /// Max steps if known (e.g., 10)
    pub max_steps: Option<u32>,
    /// Tool-specific detail (e.g., "git status", "config.rs")
    pub detail: Option<String>,
}

impl AnimationContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_step(mut self, step: u32) -> Self {
        self.step = Some(step);
        self
    }

    pub fn with_max_steps(mut self, max: u32) -> Self {
        self.max_steps = Some(max);
        self
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Format context as suffix string
    fn format_suffix(&self) -> String {
        let mut parts = Vec::new();

        // Add step info
        if let Some(step) = self.step {
            if let Some(max) = self.max_steps {
                parts.push(format!("Step {}/{}", step, max));
            } else {
                parts.push(format!("Step {}", step));
            }
        }

        // Add detail
        if let Some(ref detail) = self.detail {
            // Truncate long details (UTF-8 safe)
            let truncated = crate::utils::truncate_with_ellipsis(detail, 40);
            parts.push(truncated);
        }

        if parts.is_empty() {
            String::new()
        } else {
            format!(" · {}", parts.join(" · "))
        }
    }
}

/// Animation manager that handles all UI animations
pub struct AnimationManager {
    current_state: Arc<RwLock<AnimationState>>,
    is_running: Arc<AtomicBool>,
    current_task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    /// Current step number (shared with animation loop)
    current_step: Arc<AtomicU32>,
    /// Max steps (shared with animation loop)
    max_steps: Arc<AtomicU32>,
    /// Current detail string (shared with animation loop)
    current_detail: Arc<RwLock<Option<String>>>,
}

impl AnimationManager {
    /// Create a new animation manager
    pub fn new() -> Self {
        Self {
            current_state: Arc::new(RwLock::new(AnimationState::Idle)),
            is_running: Arc::new(AtomicBool::new(false)),
            current_task: Arc::new(RwLock::new(None)),
            current_step: Arc::new(AtomicU32::new(0)),
            max_steps: Arc::new(AtomicU32::new(0)),
            current_detail: Arc::new(RwLock::new(None)),
        }
    }

    /// Update step number (can be called while animation is running)
    pub fn set_step(&self, step: u32) {
        self.current_step.store(step, Ordering::SeqCst);
    }

    /// Update max steps (can be called while animation is running)
    pub fn set_max_steps(&self, max: Option<u32>) {
        self.max_steps.store(max.unwrap_or(0), Ordering::SeqCst);
    }

    /// Update detail string (can be called while animation is running)
    pub async fn set_detail(&self, detail: Option<String>) {
        *self.current_detail.write().await = detail;
    }

    /// Start an animation with the given state and message
    pub async fn start_animation(&self, state: AnimationState, message: &str, color: &str) {
        // Stop any existing animation first
        self.stop_animation().await;

        // Update state
        *self.current_state.write().await = state;
        self.is_running.store(true, Ordering::SeqCst);

        // Start new animation with shared context
        let is_running = self.is_running.clone();
        let message = message.to_string();
        let color = color.to_string();
        let current_step = self.current_step.clone();
        let max_steps = self.max_steps.clone();
        let current_detail = self.current_detail.clone();

        let task = tokio::spawn(async move {
            Self::run_animation(
                is_running,
                &message,
                &color,
                current_step,
                max_steps,
                current_detail,
            )
            .await;
        });

        *self.current_task.write().await = Some(task);
    }

    /// Start animation with initial context
    pub async fn start_with_context(
        &self,
        state: AnimationState,
        message: &str,
        color: &str,
        context: AnimationContext,
    ) {
        // Set context before starting
        if let Some(step) = context.step {
            self.current_step.store(step, Ordering::SeqCst);
        }
        if let Some(max) = context.max_steps {
            self.max_steps.store(max, Ordering::SeqCst);
        }
        *self.current_detail.write().await = context.detail;

        // Start animation
        self.start_animation(state, message, color).await;
    }

    /// Stop the current animation
    pub async fn stop_animation(&self) {
        // Signal stop
        self.is_running.store(false, Ordering::SeqCst);

        // Wait for current task to finish
        if let Some(task) = self.current_task.write().await.take() {
            task.abort();
            // Give it a moment to clean up
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        // Clear the line
        print!("\r\x1b[K");
        let _ = std::io::stdout().flush();

        // Update state
        *self.current_state.write().await = AnimationState::Idle;
    }

    /// Get the current animation state
    pub async fn current_state(&self) -> AnimationState {
        self.current_state.read().await.clone()
    }

    /// Check if animation is currently running
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    /// Internal animation loop with context display
    async fn run_animation(
        is_running: Arc<AtomicBool>,
        message: &str,
        color: &str,
        current_step: Arc<AtomicU32>,
        max_steps: Arc<AtomicU32>,
        current_detail: Arc<RwLock<Option<String>>>,
    ) {
        // Choose frame set based on message type
        let frames: &[&str] = if message.contains("Thinking") {
            &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]
        } else if message.contains("Running") || message.contains("Executing") {
            &["◐", "◓", "◑", "◒"]
        } else {
            &["▸", "▹", "▸", "▹"]
        };

        let mut frame_index = 0;
        let start_time = std::time::Instant::now();

        while is_running.load(Ordering::SeqCst) {
            let frame = frames[frame_index];
            let elapsed = start_time.elapsed().as_secs_f64();

            // Build context suffix
            let step = current_step.load(Ordering::SeqCst);
            let max = max_steps.load(Ordering::SeqCst);
            let detail = current_detail.read().await.clone();

            let context = AnimationContext {
                step: if step > 0 { Some(step) } else { None },
                max_steps: if max > 0 { Some(max) } else { None },
                detail,
            };
            let suffix = context.format_suffix();

            // Format: "⠋ Thinking (2.7s) · Step 3/10 · detail"
            let full_message = format!("{} ({:.1}s){}", message, elapsed, suffix);

            // Apply color
            let colored_output = match color {
                "blue" => format!("{} {}", frame, full_message).bright_blue().bold(),
                "green" => format!("{} {}", frame, full_message).bright_green().bold(),
                "yellow" => format!("{} {}", frame, full_message).bright_yellow().bold(),
                "red" => format!("{} {}", frame, full_message).bright_red().bold(),
                "cyan" => format!("{} {}", frame, full_message).bright_cyan().bold(),
                "magenta" => format!("{} {}", frame, full_message).bright_magenta().bold(),
                _ => format!("{} {}", frame, full_message).bright_white().bold(),
            };

            // Add 2-space indent for consistent UI alignment
            print!("\r  {}", colored_output);
            let _ = std::io::stdout().flush();

            frame_index = (frame_index + 1) % frames.len();

            // Animation speed
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }
}

impl Default for AnimationManager {
    fn default() -> Self {
        Self::new()
    }
}
