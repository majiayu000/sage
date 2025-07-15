//! Agent state management

use serde::{Deserialize, Serialize};

/// Current state of an agent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentState {
    /// Agent is initializing
    Initializing,
    /// Agent is thinking/processing
    Thinking,
    /// Agent is executing a tool
    ToolExecution,
    /// Agent is waiting for tool results
    WaitingForTools,
    /// Agent has completed the task successfully
    Completed,
    /// Agent encountered an error
    Error,
    /// Agent was cancelled by user
    Cancelled,
    /// Agent execution timed out
    Timeout,
}

impl std::fmt::Display for AgentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentState::Initializing => write!(f, "initializing"),
            AgentState::Thinking => write!(f, "thinking"),
            AgentState::ToolExecution => write!(f, "tool_execution"),
            AgentState::WaitingForTools => write!(f, "waiting_for_tools"),
            AgentState::Completed => write!(f, "completed"),
            AgentState::Error => write!(f, "error"),
            AgentState::Cancelled => write!(f, "cancelled"),
            AgentState::Timeout => write!(f, "timeout"),
        }
    }
}

impl AgentState {
    /// Check if the state represents a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            AgentState::Completed | AgentState::Error | AgentState::Cancelled | AgentState::Timeout
        )
    }

    /// Check if the state represents an active state
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            AgentState::Thinking | AgentState::ToolExecution | AgentState::WaitingForTools
        )
    }

    /// Check if the state represents a successful completion
    pub fn is_successful(&self) -> bool {
        matches!(self, AgentState::Completed)
    }

    /// Check if the state represents an error condition
    pub fn is_error(&self) -> bool {
        matches!(
            self,
            AgentState::Error | AgentState::Cancelled | AgentState::Timeout
        )
    }

    /// Get a human-readable description of the state
    pub fn description(&self) -> &'static str {
        match self {
            AgentState::Initializing => "Setting up and preparing for task execution",
            AgentState::Thinking => "Processing information and planning next actions",
            AgentState::ToolExecution => "Executing tools and commands",
            AgentState::WaitingForTools => "Waiting for tool execution to complete",
            AgentState::Completed => "Task completed successfully",
            AgentState::Error => "An error occurred during execution",
            AgentState::Cancelled => "Execution was cancelled by user",
            AgentState::Timeout => "Execution timed out",
        }
    }

    /// Get the next possible states from this state
    pub fn possible_transitions(&self) -> Vec<AgentState> {
        match self {
            AgentState::Initializing => vec![AgentState::Thinking, AgentState::Error],
            AgentState::Thinking => vec![
                AgentState::ToolExecution,
                AgentState::Completed,
                AgentState::Error,
                AgentState::Cancelled,
            ],
            AgentState::ToolExecution => vec![
                AgentState::WaitingForTools,
                AgentState::Thinking,
                AgentState::Error,
                AgentState::Cancelled,
            ],
            AgentState::WaitingForTools => vec![
                AgentState::Thinking,
                AgentState::Error,
                AgentState::Cancelled,
                AgentState::Timeout,
            ],
            // Terminal states have no transitions
            AgentState::Completed
            | AgentState::Error
            | AgentState::Cancelled
            | AgentState::Timeout => vec![],
        }
    }

    /// Check if transition to another state is valid
    pub fn can_transition_to(&self, target: &AgentState) -> bool {
        self.possible_transitions().contains(target)
    }
}
