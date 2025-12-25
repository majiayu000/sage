//! Error types for learning operations

/// Error types for learning operations
#[derive(Debug, thiserror::Error)]
pub enum LearningError {
    #[error("Learning mode is disabled")]
    Disabled,
    #[error("Pattern not found: {0}")]
    PatternNotFound(String),
    #[error("Storage error: {0}")]
    StorageError(String),
    #[error("Pattern limit reached")]
    PatternLimitReached,
}
