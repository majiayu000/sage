//! Trajectory recording system

pub mod recorder;
pub mod storage;

pub use recorder::TrajectoryRecorder;
pub use storage::{TrajectoryStorage, FileStorage};
