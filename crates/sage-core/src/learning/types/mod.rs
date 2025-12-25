//! Learning mode types and data structures

pub mod base;
pub mod entries;
pub mod metadata;

// Re-export all public types from submodules
pub use base::{Confidence, PatternId, PatternSource, PatternType};
pub use entries::{LearningEvent, LearningEventType, Pattern};
pub use metadata::{LearningConfig, LearningStats};
