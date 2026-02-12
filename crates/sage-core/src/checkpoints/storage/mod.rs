//! Checkpoint storage implementations
//!
//! This module provides storage backends for persisting checkpoints.

use crate::error::SageResult;
use async_trait::async_trait;

use super::types::{Checkpoint, CheckpointId};

mod compression;
mod file_storage;

#[cfg(test)]
mod memory_storage;

#[cfg(test)]
mod tests;

pub use file_storage::FileCheckpointStorage;

#[cfg(test)]
pub use memory_storage::MemoryCheckpointStorage;

/// Trait for checkpoint storage backends
#[async_trait]
pub trait CheckpointStorage: Send + Sync {
    /// Save a checkpoint
    async fn save(&self, checkpoint: &Checkpoint) -> SageResult<()>;

    /// Load a checkpoint by ID
    async fn load(&self, id: &CheckpointId) -> SageResult<Option<Checkpoint>>;

    /// List all checkpoints
    async fn list(&self) -> SageResult<Vec<CheckpointSummary>>;

    /// Delete a checkpoint
    async fn delete(&self, id: &CheckpointId) -> SageResult<()>;

    /// Check if a checkpoint exists
    async fn exists(&self, id: &CheckpointId) -> SageResult<bool>;

    /// Get the latest checkpoint
    async fn latest(&self) -> SageResult<Option<Checkpoint>>;

    /// Store file content (for large files)
    async fn store_content(&self, content: &str) -> SageResult<String>;

    /// Load file content by reference
    async fn load_content(&self, content_ref: &str) -> SageResult<Option<String>>;
}

/// Summary of a checkpoint for listing
#[derive(Debug, Clone)]
pub struct CheckpointSummary {
    pub id: CheckpointId,
    pub name: Option<String>,
    pub description: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub checkpoint_type: super::types::CheckpointType,
    pub file_count: usize,
    pub has_conversation: bool,
}

impl From<&Checkpoint> for CheckpointSummary {
    fn from(checkpoint: &Checkpoint) -> Self {
        Self {
            id: checkpoint.id.clone(),
            name: checkpoint.name.clone(),
            description: checkpoint.description.clone(),
            created_at: checkpoint.created_at,
            checkpoint_type: checkpoint.checkpoint_type,
            file_count: checkpoint.files.len(),
            has_conversation: checkpoint.conversation.is_some(),
        }
    }
}
