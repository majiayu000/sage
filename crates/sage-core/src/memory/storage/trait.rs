//! Memory storage trait definition

use super::super::types::{Memory, MemoryId, MemoryQuery, MemoryScore};
use super::error::MemoryStorageError;
use async_trait::async_trait;

/// Memory storage trait
#[async_trait]
pub trait MemoryStorage: Send + Sync {
    /// Store a memory
    async fn store(&self, memory: Memory) -> Result<MemoryId, MemoryStorageError>;

    /// Get a memory by ID
    async fn get(&self, id: &MemoryId) -> Result<Option<Memory>, MemoryStorageError>;

    /// Update a memory
    async fn update(&self, memory: Memory) -> Result<(), MemoryStorageError>;

    /// Delete a memory
    async fn delete(&self, id: &MemoryId) -> Result<(), MemoryStorageError>;

    /// Search memories
    async fn search(&self, query: &MemoryQuery) -> Result<Vec<MemoryScore>, MemoryStorageError>;

    /// List all memories (paginated)
    async fn list(&self, offset: usize, limit: usize) -> Result<Vec<Memory>, MemoryStorageError>;

    /// Count total memories
    async fn count(&self) -> Result<usize, MemoryStorageError>;

    /// Clear all memories
    async fn clear(&self) -> Result<(), MemoryStorageError>;
}
