//! Agent Events - Events emitted by Agent for UI consumption
//!
//! These events represent all UI-relevant actions from Agent execution.
//! The EventAdapter converts these to AppState updates.

use std::time::Duration;

/// Events emitted by Agent for UI consumption
#[derive(Clone, Debug)]
pub enum AgentEvent {
    /// Session started
    SessionStarted {
        session_id: String,
        model: String,
        provider: String,
    },

    /// Session ended
    SessionEnded { session_id: String },

    /// Step started
    StepStarted { step_number: u32 },

    /// Thinking started
    ThinkingStarted,

    /// Thinking stopped
    ThinkingStopped,

    /// Content stream started
    ContentStreamStarted,

    /// Content chunk received
    ContentChunk { chunk: String },

    /// Content stream ended
    ContentStreamEnded,

    /// Tool execution started
    ToolExecutionStarted {
        tool_name: String,
        description: String,
    },

    /// Tool execution completed
    ToolExecutionCompleted {
        tool_name: String,
        success: bool,
        output: Option<String>,
        error: Option<String>,
        duration: Duration,
    },

    /// Error occurred
    ErrorOccurred { error_type: String, message: String },

    /// User input requested
    UserInputRequested { prompt: String },

    /// User input received
    UserInputReceived { input: String },

    /// Git branch changed
    GitBranchChanged { branch: String },

    /// Working directory changed
    WorkingDirectoryChanged { path: String },
}

impl AgentEvent {
    /// Create a session started event
    pub fn session_started(session_id: impl Into<String>, model: impl Into<String>, provider: impl Into<String>) -> Self {
        Self::SessionStarted {
            session_id: session_id.into(),
            model: model.into(),
            provider: provider.into(),
        }
    }

    /// Create a tool execution started event
    pub fn tool_started(tool_name: impl Into<String>, description: impl Into<String>) -> Self {
        Self::ToolExecutionStarted {
            tool_name: tool_name.into(),
            description: description.into(),
        }
    }

    /// Create a tool execution completed event
    pub fn tool_completed(
        tool_name: impl Into<String>,
        success: bool,
        output: Option<String>,
        error: Option<String>,
        duration: Duration,
    ) -> Self {
        Self::ToolExecutionCompleted {
            tool_name: tool_name.into(),
            success,
            output,
            error,
            duration,
        }
    }

    /// Create an error event
    pub fn error(error_type: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ErrorOccurred {
            error_type: error_type.into(),
            message: message.into(),
        }
    }

    /// Create a content chunk event
    pub fn chunk(chunk: impl Into<String>) -> Self {
        Self::ContentChunk {
            chunk: chunk.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let event = AgentEvent::session_started("sess-123", "claude-sonnet-4-20250514", "anthropic");
        if let AgentEvent::SessionStarted { session_id, model, provider } = event {
            assert_eq!(session_id, "sess-123");
            assert_eq!(model, "claude-sonnet-4-20250514");
            assert_eq!(provider, "anthropic");
        } else {
            panic!("Expected SessionStarted event");
        }
    }

    #[test]
    fn test_tool_event() {
        let event = AgentEvent::tool_started("bash", "ls -la");
        if let AgentEvent::ToolExecutionStarted { tool_name, description } = event {
            assert_eq!(tool_name, "bash");
            assert_eq!(description, "ls -la");
        } else {
            panic!("Expected ToolExecutionStarted event");
        }
    }
}
