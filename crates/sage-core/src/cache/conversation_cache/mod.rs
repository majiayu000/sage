//! Conversation/Context Caching for LLM interactions
//!
//! This module implements incremental conversation caching similar to Claude Code.
//! It tracks the longest cached prefix of messages and enables efficient cache reuse
//! for multi-turn conversations.
//!
//! ## How it works
//!
//! 1. Each conversation turn is assigned a cache checkpoint
//! 2. The system tracks which prefixes have been cached
//! 3. On subsequent requests, it finds the longest previously cached sequence
//! 4. Only new content after the cache checkpoint needs to be processed
//!
//! ## Benefits
//!
//! - Reduces latency for follow-up messages
//! - Saves on input token costs (cache reads are 90% cheaper)
//! - Progressive efficiency as conversations continue

mod eviction;
mod operations;
mod storage;
#[cfg(test)]
mod tests;
mod types;

// Re-export public types
pub use operations::ConversationCache;
pub use storage::CachedConversation;
pub use types::{
    CacheCheckpoint, CacheLookupResult, ConversationCacheConfig, ConversationCacheStats,
    DEFAULT_CACHE_TTL_SECS, EXTENDED_CACHE_TTL_SECS,
};
