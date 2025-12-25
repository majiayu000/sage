//! Tool result caching
//!
//! This module provides caching for expensive tool operations
//! like file reads, glob searches, and web fetches.

mod config;
mod stats;
mod storage;
mod types;

#[cfg(test)]
mod tests;

pub use config::ToolCacheConfig;
pub use stats::CacheStats;
pub use storage::ToolCache;
pub use types::{CachedResult, SharedToolCache, ToolCacheKey};

/// Create a shared tool cache
pub fn create_shared_cache(config: ToolCacheConfig) -> SharedToolCache {
    std::sync::Arc::new(ToolCache::new(config))
}
