//! Execution outcome types for clear success/failure semantics
//!
//! This module provides explicit outcome types that preserve the full execution
//! trace regardless of success or failure, solving the problem where errors
//! were being silently swallowed.

use crate::agent::AgentExecution;
use crate::error::SageError;
use serde::{Deserialize, Serialize};

/// Explicit outcome of an agent execution.
///
/// Unlike returning `Result<AgentExecution, SageError>`, this type
/// preserves the full execution trace even on failure, allowing
/// callers to inspect what happened before the failure.
///
/// # Example
/// ```ignore
/// let outcome = agent.execute_task(task).await?;
/// match outcome {
///     ExecutionOutcome::Success(exec) => {
///         println!("Task completed: {:?}", exec.final_result);
///     }
///     ExecutionOutcome::Failed { execution, error } => {
///         println!("Task failed: {}", error.message);
///         println!("Steps completed: {}", execution.steps.len());
///         if let Some(suggestion) = &error.suggestion {
///             println!("Suggestion: {}", suggestion);
///         }
///     }
///     _ => {}
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionOutcome {
    /// Task completed successfully
    Success(AgentExecution),

    /// Task failed due to an error
    Failed {
        /// The execution state at the point of failure
        execution: AgentExecution,
        /// The error that caused the failure
        error: ExecutionError,
    },

    /// Task was interrupted by user (Ctrl+C)
    Interrupted {
        /// The execution state when interrupted
        execution: AgentExecution,
    },

    /// Task reached maximum steps without completion
    MaxStepsReached {
        /// The execution state at max steps
        execution: AgentExecution,
    },
}

/// Structured error information for failed executions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionError {
    /// Error category for programmatic handling
    pub kind: ExecutionErrorKind,
    /// Human-readable error message
    pub message: String,
    /// Optional provider info (for LLM errors)
    pub provider: Option<String>,
    /// Optional suggestion for fixing the error
    pub suggestion: Option<String>,
}

/// Categories of execution errors for programmatic handling
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionErrorKind {
    /// Authentication/API key errors (401, invalid key, etc.)
    Authentication,
    /// Rate limiting (429)
    RateLimit,
    /// Invalid request to LLM (400, bad parameters)
    InvalidRequest,
    /// LLM service unavailable (502, 503)
    ServiceUnavailable,
    /// Tool execution failure
    ToolExecution {
        /// Name of the tool that failed
        tool_name: String,
    },
    /// Configuration error
    Configuration,
    /// Network/HTTP error
    Network,
    /// Timeout
    Timeout,
    /// Other errors
    Other,
}

impl ExecutionOutcome {
    /// Check if the outcome is successful
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success(_))
    }

    /// Check if the outcome is a failure
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }

    /// Check if the outcome was interrupted
    pub fn is_interrupted(&self) -> bool {
        matches!(self, Self::Interrupted { .. })
    }

    /// Get the execution regardless of outcome
    pub fn execution(&self) -> &AgentExecution {
        match self {
            Self::Success(exec) => exec,
            Self::Failed { execution, .. } => execution,
            Self::Interrupted { execution } => execution,
            Self::MaxStepsReached { execution } => execution,
        }
    }

    /// Consume and get the execution
    pub fn into_execution(self) -> AgentExecution {
        match self {
            Self::Success(exec) => exec,
            Self::Failed { execution, .. } => execution,
            Self::Interrupted { execution } => execution,
            Self::MaxStepsReached { execution } => execution,
        }
    }

    /// Get error if present
    pub fn error(&self) -> Option<&ExecutionError> {
        match self {
            Self::Failed { error, .. } => Some(error),
            _ => None,
        }
    }

    /// Get a user-friendly status message
    pub fn status_message(&self) -> &'static str {
        match self {
            Self::Success(_) => "Task completed successfully",
            Self::Failed { .. } => "Task failed",
            Self::Interrupted { .. } => "Task interrupted by user",
            Self::MaxStepsReached { .. } => "Task reached maximum steps",
        }
    }

    /// Get a status icon for CLI display
    pub fn status_icon(&self) -> &'static str {
        match self {
            Self::Success(_) => "✓",
            Self::Failed { .. } => "✗",
            Self::Interrupted { .. } => "🛑",
            Self::MaxStepsReached { .. } => "⚠",
        }
    }

    /// Convert to legacy Result format (for backward compatibility)
    pub fn into_result(self) -> Result<AgentExecution, SageError> {
        match self {
            Self::Success(exec) => Ok(exec),
            Self::Failed { error, .. } => Err(SageError::agent(error.message)),
            Self::Interrupted { .. } => Err(SageError::Cancelled),
            Self::MaxStepsReached { execution } => {
                // Return Ok with the execution, as max steps is not necessarily an error
                Ok(execution)
            }
        }
    }
}

impl ExecutionError {
    /// Create a new execution error
    pub fn new(kind: ExecutionErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            provider: None,
            suggestion: None,
        }
    }

    /// Set the provider
    pub fn with_provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = Some(provider.into());
        self
    }

    /// Set a suggestion
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Parse from SageError with additional context
    pub fn from_sage_error(error: &SageError, provider: Option<String>) -> Self {
        let (kind, suggestion) = Self::classify_error(error);
        Self {
            kind,
            message: error.to_string(),
            provider,
            suggestion,
        }
    }

    /// Classify error and provide suggestions
    fn classify_error(error: &SageError) -> (ExecutionErrorKind, Option<String>) {
        match error {
            SageError::Llm(msg) => {
                let msg_lower = msg.to_lowercase();

                if msg_lower.contains("authentication")
                    || msg_lower.contains("api key")
                    || msg_lower.contains("api_key")
                    || msg_lower.contains("unauthorized")
                    || msg_lower.contains("x-api-key")
                    || msg_lower.contains("401")
                {
                    (
                        ExecutionErrorKind::Authentication,
                        Some(
                            "Check your API key in sage_config.json or environment variables"
                                .into(),
                        ),
                    )
                } else if msg_lower.contains("rate limit")
                    || msg_lower.contains("rate_limit")
                    || msg_lower.contains("429")
                {
                    (
                        ExecutionErrorKind::RateLimit,
                        Some("Wait a moment and try again, or upgrade your API plan".into()),
                    )
                } else if msg_lower.contains("503")
                    || msg_lower.contains("502")
                    || msg_lower.contains("service unavailable")
                    || msg_lower.contains("overloaded")
                {
                    (
                        ExecutionErrorKind::ServiceUnavailable,
                        Some("The LLM service is temporarily unavailable. Try again later".into()),
                    )
                } else if msg_lower.contains("400") || msg_lower.contains("invalid") {
                    (
                        ExecutionErrorKind::InvalidRequest,
                        Some("Check your request parameters and model configuration".into()),
                    )
                } else {
                    (ExecutionErrorKind::Other, None)
                }
            }
            SageError::Tool { tool_name, .. } => (
                ExecutionErrorKind::ToolExecution {
                    tool_name: tool_name.clone(),
                },
                Some(format!("Check the {} tool configuration and inputs", tool_name)),
            ),
            SageError::Config(_) => (
                ExecutionErrorKind::Configuration,
                Some("Check your sage_config.json configuration".into()),
            ),
            SageError::Timeout { seconds } => (
                ExecutionErrorKind::Timeout,
                Some(format!(
                    "Task timed out after {} seconds. Try a simpler task or increase timeout",
                    seconds
                )),
            ),
            SageError::Http(_) => (
                ExecutionErrorKind::Network,
                Some("Check your network connection and try again".into()),
            ),
            SageError::Cancelled => (
                ExecutionErrorKind::Other,
                Some("Task was cancelled".into()),
            ),
            _ => (ExecutionErrorKind::Other, None),
        }
    }

    /// Check if this error is potentially retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self.kind,
            ExecutionErrorKind::RateLimit
                | ExecutionErrorKind::ServiceUnavailable
                | ExecutionErrorKind::Network
                | ExecutionErrorKind::Timeout
        )
    }
}

impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)?;
        if let Some(provider) = &self.provider {
            write!(f, " (provider: {})", provider)?;
        }
        Ok(())
    }
}

impl std::fmt::Display for ExecutionErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Authentication => write!(f, "Authentication Error"),
            Self::RateLimit => write!(f, "Rate Limit"),
            Self::InvalidRequest => write!(f, "Invalid Request"),
            Self::ServiceUnavailable => write!(f, "Service Unavailable"),
            Self::ToolExecution { tool_name } => write!(f, "Tool Error ({})", tool_name),
            Self::Configuration => write!(f, "Configuration Error"),
            Self::Network => write!(f, "Network Error"),
            Self::Timeout => write!(f, "Timeout"),
            Self::Other => write!(f, "Error"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_error_classification() {
        // Test authentication error detection
        let auth_error = SageError::llm("Anthropic API error: authentication_error");
        let exec_error = ExecutionError::from_sage_error(&auth_error, Some("anthropic".into()));
        assert_eq!(exec_error.kind, ExecutionErrorKind::Authentication);
        assert!(exec_error.suggestion.is_some());

        // Test rate limit detection
        let rate_error = SageError::llm("Rate limit exceeded (429)");
        let exec_error = ExecutionError::from_sage_error(&rate_error, None);
        assert_eq!(exec_error.kind, ExecutionErrorKind::RateLimit);

        // Test tool error
        let tool_error = SageError::tool("bash", "Command failed");
        let exec_error = ExecutionError::from_sage_error(&tool_error, None);
        assert!(matches!(
            exec_error.kind,
            ExecutionErrorKind::ToolExecution { .. }
        ));
    }

    #[test]
    fn test_execution_error_retryable() {
        let rate_error = ExecutionError::new(ExecutionErrorKind::RateLimit, "rate limited");
        assert!(rate_error.is_retryable());

        let auth_error = ExecutionError::new(ExecutionErrorKind::Authentication, "bad key");
        assert!(!auth_error.is_retryable());
    }
}
