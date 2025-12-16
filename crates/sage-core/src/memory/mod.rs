//! Agent memory and persistence system
//!
//! Provides long-term memory storage, retrieval, and management for agents.
//! Supports facts, preferences, code context, and conversational history.

pub mod storage;
pub mod types;
pub mod manager;

pub use storage::{
    FileMemoryStorage, MemoryStorage, MemoryStorageError,
};
pub use types::{
    Memory, MemoryCategory, MemoryId, MemoryMetadata, MemoryQuery, MemoryScore,
    MemorySource, MemoryType, RelevanceScore,
};
pub use manager::{
    MemoryConfig, MemoryManager, MemoryStats, SharedMemoryManager, create_memory_manager,
};
