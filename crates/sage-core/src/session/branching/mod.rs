//! Conversation branching system
//!
//! Allows saving conversation state at key points and exploring
//! different approaches while being able to restore to previous states.

mod manager;
mod tree;
mod types;

#[cfg(test)]
mod tests;

// Re-export types
pub use manager::BranchManager;
pub use types::{BranchId, BranchNode, BranchSnapshot, SerializedMessage, SerializedToolCall};

use std::sync::Arc;

/// Thread-safe shared branch manager
pub type SharedBranchManager = Arc<BranchManager>;

/// Create a shared branch manager
pub fn create_branch_manager() -> SharedBranchManager {
    Arc::new(BranchManager::new())
}
