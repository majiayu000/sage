//! Types for sub-agent executor configuration and progress tracking

use super::super::types::{AgentType, SubAgentResult};

/// Configuration for sub-agent execution
#[derive(Debug, Clone)]
pub struct SubAgentConfig {
    /// Agent type to use
    pub agent_type: AgentType,
    /// Task description
    pub task: String,
    /// Additional context
    pub context: Option<String>,
    /// Override maximum steps
    pub max_steps: Option<usize>,
    /// Override temperature
    pub temperature: Option<f64>,
}

impl SubAgentConfig {
    /// Create a new sub-agent configuration
    pub fn new(agent_type: AgentType, task: impl Into<String>) -> Self {
        Self {
            agent_type,
            task: task.into(),
            context: None,
            max_steps: None,
            temperature: None,
        }
    }

    /// Set context
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Set max steps
    pub fn with_max_steps(mut self, max_steps: usize) -> Self {
        self.max_steps = Some(max_steps);
        self
    }

    /// Set temperature
    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }
}

/// Progress update from executor
#[derive(Debug, Clone)]
pub struct AgentProgress {
    /// Current step
    pub step: usize,
    /// Max steps
    pub max_steps: usize,
    /// Current action
    pub action: String,
    /// Progress percentage
    pub percentage: u8,
}

impl AgentProgress {
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
    Progress(AgentProgress),
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
