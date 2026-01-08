//! Progress tracking and display for long-running tasks
//!
//! This module provides visual feedback for users during agent execution,
//! addressing the "stuck" perception problem in open-ended tasks.

use colored::*;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Execution phase for visual feedback
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionPhase {
    /// Exploring codebase, searching files
    Exploring,
    /// Analyzing code, comparing, reading
    Analyzing,
    /// Synthesizing results, generating output
    Synthesizing,
    /// Executing tools
    Executing,
    /// Waiting for user input
    WaitingForInput,
}

impl ExecutionPhase {
    /// Get display name for the phase
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Exploring => "Exploring",
            Self::Analyzing => "Analyzing",
            Self::Synthesizing => "Synthesizing",
            Self::Executing => "Executing",
            Self::WaitingForInput => "Waiting",
        }
    }

    /// Get icon for the phase
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Exploring => "󰍉",  // search
            Self::Analyzing => "",   // analyze
            Self::Synthesizing => "", // merge
            Self::Executing => "",   // run
            Self::WaitingForInput => "󰏤", // pause
        }
    }

    /// Get color for the phase
    pub fn color(&self) -> &'static str {
        match self {
            Self::Exploring => "cyan",
            Self::Analyzing => "yellow",
            Self::Synthesizing => "green",
            Self::Executing => "magenta",
            Self::WaitingForInput => "blue",
        }
    }

    /// Infer phase from tool name
    pub fn from_tool_name(tool_name: &str) -> Self {
        match tool_name.to_lowercase().as_str() {
            "glob" | "grep" | "web_search" => Self::Exploring,
            "read" | "lsp" => Self::Analyzing,
            "write" | "edit" | "notebook_edit" => Self::Synthesizing,
            "bash" | "task" => Self::Executing,
            "ask_user_question" => Self::WaitingForInput,
            _ => Self::Executing,
        }
    }
}

/// Activity record for heartbeat display
#[derive(Debug, Clone)]
pub struct Activity {
    pub description: String,
    pub timestamp: Instant,
}

/// Subagent status for display
#[derive(Debug, Clone)]
pub struct SubagentStatus {
    pub task_description: String,
    pub current_step: u32,
    pub max_steps: Option<u32>,
    pub last_tool: Option<String>,
}

/// Progress tracker for execution feedback
pub struct ProgressTracker {
    /// Start time of the task
    start_time: Instant,
    /// Current phase
    phase: RwLock<ExecutionPhase>,
    /// Current step number
    current_step: AtomicU64,
    /// Maximum steps (if known)
    max_steps: RwLock<Option<u32>>,
    /// Last activity
    last_activity: RwLock<Option<Activity>>,
    /// Active subagent status
    subagent: RwLock<Option<SubagentStatus>>,
    /// Whether heartbeat display is active
    heartbeat_active: AtomicBool,
    /// Heartbeat task handle
    heartbeat_handle: RwLock<Option<tokio::task::JoinHandle<()>>>,
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressTracker {
    /// Create a new progress tracker
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            phase: RwLock::new(ExecutionPhase::Exploring),
            current_step: AtomicU64::new(0),
            max_steps: RwLock::new(None),
            last_activity: RwLock::new(None),
            subagent: RwLock::new(None),
            heartbeat_active: AtomicBool::new(false),
            heartbeat_handle: RwLock::new(None),
        }
    }

    /// Reset tracker for a new task
    pub async fn reset(&self) {
        *self.phase.write().await = ExecutionPhase::Exploring;
        self.current_step.store(0, Ordering::SeqCst);
        *self.max_steps.write().await = None;
        *self.last_activity.write().await = None;
        *self.subagent.write().await = None;
    }

    /// Update the current phase
    pub async fn set_phase(&self, phase: ExecutionPhase) {
        *self.phase.write().await = phase;
    }

    /// Get the current phase
    pub async fn get_phase(&self) -> ExecutionPhase {
        *self.phase.read().await
    }

    /// Update step number
    pub fn set_step(&self, step: u32) {
        self.current_step.store(step as u64, Ordering::SeqCst);
    }

    /// Get current step
    pub fn get_step(&self) -> u32 {
        self.current_step.load(Ordering::SeqCst) as u32
    }

    /// Set maximum steps
    pub async fn set_max_steps(&self, max: Option<u32>) {
        *self.max_steps.write().await = max;
    }

    /// Record an activity
    pub async fn record_activity(&self, description: &str) {
        *self.last_activity.write().await = Some(Activity {
            description: description.to_string(),
            timestamp: Instant::now(),
        });
    }

    /// Update subagent status
    pub async fn set_subagent(&self, status: Option<SubagentStatus>) {
        *self.subagent.write().await = status;
    }

    /// Get elapsed time
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Display phase header
    pub async fn display_phase_header(&self) {
        let phase = self.phase.read().await;
        let step = self.current_step.load(Ordering::SeqCst) as u32;
        let max = self.max_steps.read().await;

        let step_info = match *max {
            Some(max) => format!("Step {}/{}", step, max),
            None => format!("Step {}", step),
        };

        println!();
        println!(
            "  {} {}  {}",
            phase.icon().bright_cyan(),
            phase.display_name().bright_white().bold(),
            step_info.dimmed()
        );
    }

    /// Display subagent status if active
    pub async fn display_subagent_status(&self) {
        let subagent = self.subagent.read().await;
        if let Some(ref status) = *subagent {
            let step_info = match status.max_steps {
                Some(max) => format!("Step {}/{}", status.current_step, max),
                None => format!("Step {}", status.current_step),
            };

            println!();
            println!(
                "  {} {}",
                "󰜗".bright_yellow(),
                "Subagent running".bright_yellow().bold()
            );

            // Truncate task description if too long (UTF-8 safe)
            let desc = crate::utils::truncate_with_ellipsis(&status.task_description, 50);
            println!("    {} {}", "".dimmed(), desc);

            if let Some(ref tool) = status.last_tool {
                println!(
                    "    └──  {}  {}",
                    step_info.dimmed(),
                    tool.bright_magenta()
                );
            } else {
                println!("    └── {}", step_info.dimmed());
            }
        }
    }

    /// Display progress bar for long-running tasks
    pub async fn display_progress_bar(&self) {
        let elapsed = self.elapsed();

        // Only show progress bar after 30 seconds
        if elapsed.as_secs() < 30 {
            return;
        }

        let step = self.current_step.load(Ordering::SeqCst) as u32;
        let max = self.max_steps.read().await;
        let phase = self.phase.read().await;

        // Calculate progress percentage
        let (percentage, bar) = match *max {
            Some(max) if max > 0 => {
                let pct = (step as f64 / max as f64 * 100.0).min(100.0) as usize;
                let filled = pct * 20 / 100;
                let empty = 20 - filled;
                (
                    Some(pct),
                    format!("{}{}", "█".repeat(filled), "░".repeat(empty)),
                )
            }
            _ => (None, "░".repeat(20)),
        };

        // Format elapsed time
        let elapsed_str = format_duration(elapsed);

        println!();
        print!("  ⏳ {} ({})", "Long-running task".dimmed(), elapsed_str.dimmed());

        if let Some(pct) = percentage {
            println!("  [{}] {}%", bar, pct);
        } else {
            println!();
        }

        println!("     {} {}", phase.icon(), phase.display_name().dimmed());
    }

    /// Display heartbeat indicator
    pub async fn display_heartbeat(&self) {
        let activity = self.last_activity.read().await;

        if let Some(ref act) = *activity {
            let ago = act.timestamp.elapsed();
            let ago_str = if ago.as_secs() < 1 {
                "just now".to_string()
            } else {
                format!("{}s ago", ago.as_secs())
            };

            // Truncate description (UTF-8 safe)
            let desc = crate::utils::truncate_with_ellipsis(&act.description, 40);

            print!("\r\x1B[K");
            print!(
                "  {} Working... (last: {} {})",
                "●".bright_green(),
                desc.dimmed(),
                ago_str.dimmed()
            );
            std::io::Write::flush(&mut std::io::stdout()).ok();
        }
    }

    /// Start heartbeat display loop
    pub async fn start_heartbeat(self: &Arc<Self>) {
        if self.heartbeat_active.swap(true, Ordering::SeqCst) {
            return; // Already running
        }

        let tracker = Arc::clone(self);
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            loop {
                interval.tick().await;
                if !tracker.heartbeat_active.load(Ordering::SeqCst) {
                    break;
                }
                tracker.display_heartbeat().await;
            }
        });

        *self.heartbeat_handle.write().await = Some(handle);
    }

    /// Stop heartbeat display
    pub async fn stop_heartbeat(&self) {
        self.heartbeat_active.store(false, Ordering::SeqCst);

        // Clear the heartbeat line
        print!("\r\x1B[K");
        std::io::Write::flush(&mut std::io::stdout()).ok();

        // Abort the task if running
        if let Some(handle) = self.heartbeat_handle.write().await.take() {
            handle.abort();
        }
    }

    /// Display full status (phase + subagent + progress if needed)
    pub async fn display_full_status(&self) {
        self.display_phase_header().await;
        self.display_subagent_status().await;
        self.display_progress_bar().await;
    }
}

/// Format duration for display
fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}

/// Global progress tracker instance
static PROGRESS_TRACKER: std::sync::OnceLock<Arc<ProgressTracker>> = std::sync::OnceLock::new();

/// Get the global progress tracker
pub fn global_progress_tracker() -> Arc<ProgressTracker> {
    PROGRESS_TRACKER
        .get_or_init(|| Arc::new(ProgressTracker::new()))
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_from_tool() {
        assert_eq!(ExecutionPhase::from_tool_name("glob"), ExecutionPhase::Exploring);
        assert_eq!(ExecutionPhase::from_tool_name("read"), ExecutionPhase::Analyzing);
        assert_eq!(ExecutionPhase::from_tool_name("write"), ExecutionPhase::Synthesizing);
        assert_eq!(ExecutionPhase::from_tool_name("bash"), ExecutionPhase::Executing);
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_secs(30)), "30s");
        assert_eq!(format_duration(Duration::from_secs(90)), "1m 30s");
        assert_eq!(format_duration(Duration::from_secs(3661)), "1h 1m");
    }
}
