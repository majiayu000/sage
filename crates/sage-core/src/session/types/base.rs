//! Base types for session management
//!
//! This module contains the foundational types used across the session system.

use serde::{Deserialize, Serialize};
use std::fmt;

// =============================================================================
// Type Aliases
// =============================================================================

/// Unique session identifier
pub type SessionId = String;

// =============================================================================
// Session State
// =============================================================================

/// Session state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    /// Session is actively being used
    Active,
    /// Session is paused/suspended
    Paused,
    /// Session completed successfully
    Completed,
    /// Session failed with an error
    Failed,
    /// Session was cancelled by user
    Cancelled,
}

impl fmt::Display for SessionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionState::Active => write!(f, "active"),
            SessionState::Paused => write!(f, "paused"),
            SessionState::Completed => write!(f, "completed"),
            SessionState::Failed => write!(f, "failed"),
            SessionState::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl Default for SessionState {
    fn default() -> Self {
        SessionState::Active
    }
}

// =============================================================================
// Message Role (re-exported from llm::messages)
// =============================================================================

pub use crate::types::MessageRole;

// =============================================================================
// Token Usage (re-exported from crate::types)
// =============================================================================

pub use crate::types::TokenUsage;

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_usage() {
        let mut usage1 = TokenUsage {
            input_tokens: 100,
            output_tokens: 50,
            cache_read_tokens: Some(10),
            cache_write_tokens: Some(5),
            cost_estimate: Some(0.01),
        };

        let usage2 = TokenUsage {
            input_tokens: 200,
            output_tokens: 100,
            cache_read_tokens: Some(20),
            cache_write_tokens: Some(10),
            cost_estimate: Some(0.02),
        };

        usage1.add(&usage2);
        assert_eq!(usage1.input_tokens, 300);
        assert_eq!(usage1.output_tokens, 150);
        assert_eq!(usage1.total_tokens(), 450);
    }

    #[test]
    fn test_session_state_display() {
        assert_eq!(format!("{}", SessionState::Active), "active");
        assert_eq!(format!("{}", SessionState::Paused), "paused");
        assert_eq!(format!("{}", SessionState::Completed), "completed");
        assert_eq!(format!("{}", SessionState::Failed), "failed");
        assert_eq!(format!("{}", SessionState::Cancelled), "cancelled");
    }
}
