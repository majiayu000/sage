//! Types for mode manager

use std::path::PathBuf;

use crate::modes::types::{AgentMode, PlanModeConfig};

/// Context returned when entering plan mode
#[derive(Debug, Clone)]
pub struct PlanModeContext {
    /// Path to the plan file
    pub plan_file: PathBuf,
    /// Plan mode configuration
    pub config: PlanModeConfig,
    /// Previous mode before entering plan mode
    pub previous_mode: AgentMode,
}

/// Result of exiting a mode
#[derive(Debug, Clone)]
pub struct ModeExitResult {
    /// Whether the mode was actually exited
    pub exited: bool,
    /// Plan file path (if was in plan mode)
    pub plan_file: Option<PathBuf>,
    /// Number of tool calls blocked during mode
    pub blocked_tool_calls: usize,
    /// Duration in the mode (seconds)
    pub duration_secs: u64,
    /// Status message
    pub message: String,
}
