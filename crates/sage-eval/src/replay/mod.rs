//! Trajectory replay system for regression testing
//!
//! This module provides functionality for recording "golden" sessions
//! and replaying them to detect regressions.

mod recorder;
mod regression;
mod replayer;

pub use recorder::GoldenRecorder;
pub use regression::RegressionDetector;
pub use replayer::SessionReplayer;
