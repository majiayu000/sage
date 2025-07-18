//! Trajectory recording system

pub mod recorder;
pub mod storage;
pub mod memory_optimized;

#[cfg(test)]
mod memory_optimized_tests;

pub use recorder::TrajectoryRecorder;
pub use storage::{TrajectoryStorage, FileStorage};
