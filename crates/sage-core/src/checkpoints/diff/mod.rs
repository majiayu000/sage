//! Diff utilities for checkpoint system
//!
//! This module provides utilities for detecting changes between checkpoints
//! and generating file snapshots.

mod capture;
mod changes;
mod compare;
mod scanner;
mod text_diff;

#[cfg(test)]
mod tests;

// Re-export public types
pub use capture::ChangeDetector;
pub use changes::FileChange;
pub use compare::{changes_to_snapshots, compare_snapshots};
pub use text_diff::{DiffHunk, DiffLine, TextDiff};
