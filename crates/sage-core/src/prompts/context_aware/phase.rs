//! Conversation phase enum and properties

use std::fmt;

/// Conversation phases that influence prompt behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConversationPhase {
    /// Fresh conversation, understanding the request
    Initial,
    /// Gathering context, reading files, searching codebase
    Exploring,
    /// Designing implementation approach
    Planning,
    /// Writing code, making changes
    Implementing,
    /// Fixing errors, investigating issues
    Debugging,
    /// Running tests, verifying behavior
    Testing,
    /// Code review, final checks
    Reviewing,
    /// Wrapping up, summarizing work done
    Completing,
}

impl fmt::Display for ConversationPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConversationPhase::Initial => write!(f, "Initial"),
            ConversationPhase::Exploring => write!(f, "Exploring"),
            ConversationPhase::Planning => write!(f, "Planning"),
            ConversationPhase::Implementing => write!(f, "Implementing"),
            ConversationPhase::Debugging => write!(f, "Debugging"),
            ConversationPhase::Testing => write!(f, "Testing"),
            ConversationPhase::Reviewing => write!(f, "Reviewing"),
            ConversationPhase::Completing => write!(f, "Completing"),
        }
    }
}

impl ConversationPhase {
    /// Get all phases in typical workflow order
    pub fn workflow_order() -> &'static [ConversationPhase] {
        &[
            ConversationPhase::Initial,
            ConversationPhase::Exploring,
            ConversationPhase::Planning,
            ConversationPhase::Implementing,
            ConversationPhase::Testing,
            ConversationPhase::Debugging,
            ConversationPhase::Reviewing,
            ConversationPhase::Completing,
        ]
    }

    /// Check if this phase is read-only (no file modifications expected)
    pub fn is_read_only(&self) -> bool {
        matches!(
            self,
            ConversationPhase::Initial
                | ConversationPhase::Exploring
                | ConversationPhase::Planning
                | ConversationPhase::Reviewing
        )
    }

    /// Check if this phase involves active coding
    pub fn is_coding_phase(&self) -> bool {
        matches!(
            self,
            ConversationPhase::Implementing | ConversationPhase::Debugging
        )
    }
}
