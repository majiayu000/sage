//! Types for sub-agent executor configuration and progress tracking

use super::super::types::SubAgentResult;

/// Progress update from executor
#[derive(Debug, Clone)]
pub struct ExecutorProgress {
    /// Current step
    pub step: usize,
    /// Max steps
    pub max_steps: usize,
    /// Current action
    pub action: String,
    /// Progress percentage
    pub percentage: u8,
}

impl ExecutorProgress {
    /// Create progress update
    pub fn new(step: usize, max_steps: usize, action: impl Into<String>) -> Self {
        let percentage = if max_steps > 0 {
            ((step as f64 / max_steps as f64) * 100.0).min(100.0) as u8
        } else {
            0
        };

        Self {
            step,
            max_steps,
            action: action.into(),
            percentage,
        }
    }
}

/// Message from executor to monitor progress
#[derive(Debug, Clone)]
pub enum ExecutorMessage {
    /// Progress update
    Progress(ExecutorProgress),
    /// Tool call started
    ToolCall { name: String, id: String },
    /// Tool result received
    ToolResult { id: String, success: bool },
    /// Execution completed
    Completed(SubAgentResult),
    /// Execution failed
    Failed(String),
}

/// Result of a single step
pub(super) enum StepResult {
    /// Continue to next step
    Continue,
    /// Task completed with final message
    Completed(String),
    /// Need more steps but hit limit
    NeedsMoreSteps,
}
