//! Trajectory recording system
//!
//! Provides two storage backends:
//! - `ProjectStorage`: Claude Code-style JSONL storage in ~/.sage/projects/{cwd}/
//! - `FileStorage`: Simple JSON file storage (legacy)

pub mod memory_optimized;
pub mod project_storage;
pub mod recorder;
pub mod storage;

#[cfg(test)]
mod memory_optimized_tests;

pub use project_storage::{ProjectStorage, TokenUsageEntry, TrajectoryEntry};
pub use recorder::TrajectoryRecorder;
pub use storage::{FileStorage, TrajectoryStorage};
