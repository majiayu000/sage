//! Trajectory recording and replay system
//!
//! Provides two storage backends:
//! - `ProjectStorage`: Claude Code-style JSONL storage in ~/.sage/projects/{cwd}/
//! - `FileStorage`: Simple JSON file storage (legacy)
//!
//! And replay functionality:
//! - `TrajectoryReplayer`: Load and analyze recorded trajectories

pub mod memory_optimized;
pub mod project_storage;
pub mod recorder;
pub mod replayer;
pub mod storage;

#[cfg(test)]
mod memory_optimized_tests;

pub use project_storage::{ProjectStorage, TokenUsageEntry, TrajectoryEntry};
pub use recorder::TrajectoryRecorder;
pub use replayer::{
    ReplayMode, ReplaySummary, StepAnalysis, StepReplayResult, TokenUsageStats, ToolCallInfo,
    ToolResultInfo, TrajectoryInfo, TrajectoryReplayer,
};
pub use storage::{FileStorage, RotationConfig, StorageStatistics, TrajectoryStorage};
