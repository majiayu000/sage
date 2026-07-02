//! Agent memory and persistence system
//!
//! Provides long-term memory storage, retrieval, and management for agents.
//! Supports facts, preferences, code context, and conversational history.

pub mod manager;
pub mod runtime;
pub mod storage;
pub mod types;

pub use manager::{
    MemoryConfig, MemoryManager, MemoryStats, SharedMemoryManager, create_memory_manager,
};
pub use runtime::{
    AgentMemoryRuntime, AgentOutcomeKind, AgentOutcomeRecord, RecallQuery, RecalledContext,
    init_agent_memory_runtime, init_global_learning_engine, init_global_memory_manager,
    recall_agent_context, record_agent_outcome,
};
pub use storage::{FileMemoryStorage, MemoryStorage, MemoryStorageError};
pub use types::{
    Memory, MemoryCategory, MemoryId, MemoryMetadata, MemoryQuery, MemoryScore, MemorySource,
    MemoryType, RelevanceScore,
};
