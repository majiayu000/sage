//! Memory storage errors

use super::super::types::MemoryId;
use thiserror::Error;

/// Memory storage error
#[derive(Debug, Error)]
pub enum MemoryStorageError {
    #[error("Memory not found: {0}")]
    NotFound(MemoryId),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Storage corrupted: {0}")]
    Corrupted(String),

    #[error("Storage full")]
    StorageFull,
}
