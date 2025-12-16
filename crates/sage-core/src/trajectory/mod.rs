//! Trajectory recording system

pub mod memory_optimized;
pub mod recorder;
pub mod storage;

#[cfg(test)]
mod memory_optimized_tests;

pub use recorder::TrajectoryRecorder;
pub use storage::{FileStorage, TrajectoryStorage};
