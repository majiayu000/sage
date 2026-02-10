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

pub use crate::llm::messages::MessageRole;

// =============================================================================
// Token Usage
// =============================================================================

/// Token usage statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Input tokens used
    pub input_tokens: u64,
    /// Output tokens used
    pub output_tokens: u64,
    /// Cache read tokens
    pub cache_read_tokens: u64,
    /// Cache write tokens
    pub cache_write_tokens: u64,
    /// Total cost estimate (in USD)
    pub cost_estimate: f64,
}

impl TokenUsage {
    /// Create new token usage
    pub fn new() -> Self {
        Self::default()
    }

    /// Add usage from another TokenUsage
    pub fn add(&mut self, other: &TokenUsage) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.cache_read_tokens += other.cache_read_tokens;
        self.cache_write_tokens += other.cache_write_tokens;
        self.cost_estimate += other.cost_estimate;
    }

    /// Get total tokens
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}

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
            cache_read_tokens: 10,
            cache_write_tokens: 5,
            cost_estimate: 0.01,
        };

        let usage2 = TokenUsage {
            input_tokens: 200,
            output_tokens: 100,
            cache_read_tokens: 20,
            cache_write_tokens: 10,
            cost_estimate: 0.02,
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
