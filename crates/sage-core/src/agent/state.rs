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
    ///
    /// Uses direct pattern matching instead of allocating a Vec for efficiency.
    pub fn can_transition_to(&self, target: &AgentState) -> bool {
        match (self, target) {
            // From Initializing
            (AgentState::Initializing, AgentState::Thinking | AgentState::Error) => true,
            // From Thinking
            (
                AgentState::Thinking,
                AgentState::ToolExecution
                | AgentState::Completed
                | AgentState::Error
                | AgentState::Cancelled,
            ) => true,
            // From ToolExecution
            (
                AgentState::ToolExecution,
                AgentState::WaitingForTools
                | AgentState::Thinking
                | AgentState::Error
                | AgentState::Cancelled,
            ) => true,
            // From WaitingForTools
            (
                AgentState::WaitingForTools,
                AgentState::Thinking
                | AgentState::Error
                | AgentState::Cancelled
                | AgentState::Timeout,
            ) => true,
            // Terminal states have no valid transitions
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_is_terminal() {
        assert!(AgentState::Completed.is_terminal());
        assert!(AgentState::Error.is_terminal());
        assert!(AgentState::Cancelled.is_terminal());
        assert!(AgentState::Timeout.is_terminal());

        assert!(!AgentState::Initializing.is_terminal());
        assert!(!AgentState::Thinking.is_terminal());
        assert!(!AgentState::ToolExecution.is_terminal());
        assert!(!AgentState::WaitingForTools.is_terminal());
    }

    #[test]
    fn test_state_is_active() {
        assert!(AgentState::Thinking.is_active());
        assert!(AgentState::ToolExecution.is_active());
        assert!(AgentState::WaitingForTools.is_active());

        assert!(!AgentState::Initializing.is_active());
        assert!(!AgentState::Completed.is_active());
        assert!(!AgentState::Error.is_active());
        assert!(!AgentState::Cancelled.is_active());
        assert!(!AgentState::Timeout.is_active());
    }

    #[test]
    fn test_state_is_successful() {
        assert!(AgentState::Completed.is_successful());

        assert!(!AgentState::Thinking.is_successful());
        assert!(!AgentState::Error.is_successful());
        assert!(!AgentState::Cancelled.is_successful());
    }

    #[test]
    fn test_state_is_error() {
        assert!(AgentState::Error.is_error());
        assert!(AgentState::Cancelled.is_error());
        assert!(AgentState::Timeout.is_error());

        assert!(!AgentState::Completed.is_error());
        assert!(!AgentState::Thinking.is_error());
    }

    #[test]
    fn test_state_transitions() {
        // Valid transitions from Initializing
        assert!(AgentState::Initializing.can_transition_to(&AgentState::Thinking));
        assert!(AgentState::Initializing.can_transition_to(&AgentState::Error));
        assert!(!AgentState::Initializing.can_transition_to(&AgentState::Completed));

        // Valid transitions from Thinking
        assert!(AgentState::Thinking.can_transition_to(&AgentState::ToolExecution));
        assert!(AgentState::Thinking.can_transition_to(&AgentState::Completed));
        assert!(AgentState::Thinking.can_transition_to(&AgentState::Error));
        assert!(AgentState::Thinking.can_transition_to(&AgentState::Cancelled));
        assert!(!AgentState::Thinking.can_transition_to(&AgentState::Initializing));

        // Valid transitions from ToolExecution
        assert!(AgentState::ToolExecution.can_transition_to(&AgentState::WaitingForTools));
        assert!(AgentState::ToolExecution.can_transition_to(&AgentState::Thinking));
        assert!(AgentState::ToolExecution.can_transition_to(&AgentState::Error));

        // Terminal states have no valid transitions
        assert!(!AgentState::Completed.can_transition_to(&AgentState::Thinking));
        assert!(!AgentState::Error.can_transition_to(&AgentState::Thinking));
        assert!(AgentState::Completed.possible_transitions().is_empty());
    }

    #[test]
    fn test_state_display() {
        assert_eq!(AgentState::Thinking.to_string(), "thinking");
        assert_eq!(AgentState::Completed.to_string(), "completed");
        assert_eq!(AgentState::Error.to_string(), "error");
        assert_eq!(AgentState::ToolExecution.to_string(), "tool_execution");
    }

    #[test]
    fn test_state_description() {
        assert!(!AgentState::Thinking.description().is_empty());
        assert!(!AgentState::Completed.description().is_empty());
        assert!(AgentState::Thinking.description().contains("Processing"));
        assert!(AgentState::Completed.description().contains("successfully"));
    }

    #[test]
    fn test_state_possible_transitions() {
        let transitions = AgentState::Thinking.possible_transitions();
        assert!(transitions.contains(&AgentState::ToolExecution));
        assert!(transitions.contains(&AgentState::Completed));
        assert!(transitions.contains(&AgentState::Error));
        assert!(!transitions.contains(&AgentState::Initializing));
    }
}
