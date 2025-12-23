//! Agent status and result types

use super::{AgentProgress, ExecutionMetadata};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Result from sub-agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentResult {
    /// Unique agent identifier
    pub agent_id: String,
    /// Result content/output
    pub content: String,
    /// Execution metadata
    pub metadata: ExecutionMetadata,
}

impl fmt::Display for SubAgentResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SubAgentResult(id: {}, content_len: {}, {})",
            self.agent_id,
            self.content.len(),
            self.metadata
        )
    }
}

/// Agent status during execution lifecycle
#[derive(Debug, Clone)]
pub enum AgentStatus {
    /// Agent is queued but not yet running
    Pending,
    /// Agent is currently executing with progress information
    Running(AgentProgress),
    /// Agent completed successfully
    Completed(SubAgentResult),
    /// Agent failed with error message
    Failed(String),
    /// Agent was cancelled/killed
    Killed,
}

impl fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentStatus::Pending => write!(f, "Pending"),
            AgentStatus::Running(progress) => write!(f, "Running({})", progress),
            AgentStatus::Completed(result) => write!(f, "Completed({})", result.agent_id),
            AgentStatus::Failed(err) => write!(f, "Failed: {}", err),
            AgentStatus::Killed => write!(f, "Killed"),
        }
    }
}

impl AgentStatus {
    /// Check if agent is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            AgentStatus::Completed(_) | AgentStatus::Failed(_) | AgentStatus::Killed
        )
    }

    /// Check if agent is currently running
    pub fn is_running(&self) -> bool {
        matches!(self, AgentStatus::Running(_))
    }

    /// Get progress if agent is running
    pub fn progress(&self) -> Option<&AgentProgress> {
        match self {
            AgentStatus::Running(progress) => Some(progress),
            _ => None,
        }
    }

    /// Get mutable progress if agent is running
    pub fn progress_mut(&mut self) -> Option<&mut AgentProgress> {
        match self {
            AgentStatus::Running(progress) => Some(progress),
            _ => None,
        }
    }

    /// Get result if agent completed successfully
    pub fn result(&self) -> Option<&SubAgentResult> {
        match self {
            AgentStatus::Completed(result) => Some(result),
            _ => None,
        }
    }
}
