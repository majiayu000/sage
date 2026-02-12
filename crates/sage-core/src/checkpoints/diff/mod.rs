//! Diff utilities for checkpoint system
//!
//! This module provides utilities for detecting changes between checkpoints
//! and generating file snapshots.

mod capture;
mod changes;
mod compare;
mod scanner;

#[cfg(test)]
mod tests;

// Re-export public types
pub use capture::ChangeDetector;
pub use changes::FileChange;
pub use compare::{changes_to_snapshots, compare_snapshots};
