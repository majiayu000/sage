//! Agent memory and persistence system
//!
//! Provides long-term memory storage, retrieval, and management for agents.
//! Supports facts, preferences, code context, and conversational history.

pub mod manager;
pub mod storage;
pub mod types;

pub use manager::{
    MemoryConfig, MemoryManager, MemoryStats, SharedMemoryManager, create_memory_manager,
};
pub use storage::{FileMemoryStorage, MemoryStorage, MemoryStorageError};
pub use types::{
    Memory, MemoryCategory, MemoryId, MemoryMetadata, MemoryQuery, MemoryScore, MemorySource,
    MemoryType, RelevanceScore,
};
