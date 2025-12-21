//! Caching system for Sage Agent
//!
//! This module provides intelligent caching for LLM responses and tool results
//! to improve performance and reduce API costs.
//!
//! ## Caching Strategies
//!
//! - **LLM Response Cache**: Caches complete LLM responses keyed by messages + model
//! - **Conversation Cache**: Incremental caching of conversation prefixes for multi-turn efficiency
//! - **Tool Result Cache**: Caches expensive tool execution results

pub mod conversation_cache;
pub mod llm_cache;
pub mod storage;
pub mod types;

#[cfg(test)]
mod tests;

pub use conversation_cache::{
    CacheCheckpoint, CacheLookupResult, CachedConversation, ConversationCache,
    ConversationCacheConfig, ConversationCacheStats,
};
pub use llm_cache::LLMCache;
pub use storage::{CacheStorage, DiskStorage, MemoryStorage};
pub use types::{CacheConfig, CacheEntry, CacheKey, CacheStatistics};

use crate::error::SageResult;
use async_trait::async_trait;
use std::time::Duration;

/// Cache manager that coordinates different cache layers
#[derive(Debug)]
pub struct CacheManager {
    /// Memory cache for fast access
    memory_cache: MemoryStorage,
    /// Disk cache for persistence
    disk_cache: Option<DiskStorage>,
    /// Cache configuration
    config: CacheConfig,
}

impl CacheManager {
    /// Create a new cache manager
    pub fn new(config: CacheConfig) -> SageResult<Self> {
        let memory_cache = MemoryStorage::new(config.memory_capacity);
        let disk_cache = if config.enable_disk_cache {
            Some(DiskStorage::new(
                &config.disk_cache_dir,
                config.disk_capacity,
            )?)
        } else {
            None
        };

        Ok(Self {
            memory_cache,
            disk_cache,
            config,
        })
    }

    /// Get a value from cache (checks memory first, then disk)
    pub async fn get<T>(&self, key: &CacheKey) -> SageResult<Option<T>>
    where
        T: serde::de::DeserializeOwned + Clone,
    {
        // Try memory cache first
        if let Some(entry) = self.memory_cache.get(key).await? {
            if !entry.is_expired() {
                if let Ok(value) = serde_json::from_value(entry.data.clone()) {
                    return Ok(Some(value));
                }
            }
        }

        // Try disk cache if memory cache miss
        if let Some(disk_cache) = &self.disk_cache {
            if let Some(entry) = disk_cache.get(key).await? {
                if !entry.is_expired() {
                    if let Ok(value) = serde_json::from_value(entry.data.clone()) {
                        // Promote to memory cache
                        let _ = self.memory_cache.set(key.clone(), entry).await;
                        return Ok(Some(value));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Set a value in cache (stores in both memory and disk if enabled)
    pub async fn set<T>(&self, key: CacheKey, value: T, ttl: Option<Duration>) -> SageResult<()>
    where
        T: serde::Serialize,
    {
        let data = serde_json::to_value(value)?;
        let entry = CacheEntry::new(data, ttl);

        // Store in memory cache
        self.memory_cache.set(key.clone(), entry.clone()).await?;

        // Store in disk cache if enabled
        if let Some(disk_cache) = &self.disk_cache {
            disk_cache.set(key, entry).await?;
        }

        Ok(())
    }

    /// Remove a value from cache
    pub async fn remove(&self, key: &CacheKey) -> SageResult<()> {
        self.memory_cache.remove(key).await?;
        if let Some(disk_cache) = &self.disk_cache {
            disk_cache.remove(key).await?;
        }
        Ok(())
    }

    /// Clear all cache entries
    pub async fn clear(&self) -> SageResult<()> {
        self.memory_cache.clear().await?;
        if let Some(disk_cache) = &self.disk_cache {
            disk_cache.clear().await?;
        }
        Ok(())
    }

    /// Get cache statistics
    pub async fn statistics(&self) -> SageResult<CacheStatistics> {
        let memory_stats = self.memory_cache.statistics().await?;
        let disk_stats = if let Some(disk_cache) = &self.disk_cache {
            Some(disk_cache.statistics().await?)
        } else {
            None
        };

        let total_hits = memory_stats.hits + disk_stats.as_ref().map(|s| s.hits).unwrap_or(0);
        let total_misses = memory_stats.misses + disk_stats.as_ref().map(|s| s.misses).unwrap_or(0);

        Ok(CacheStatistics {
            memory_stats,
            disk_stats,
            total_hits,
            total_misses,
        })
    }

    /// Cleanup expired entries
    pub async fn cleanup_expired(&self) -> SageResult<()> {
        self.memory_cache.cleanup_expired().await?;
        if let Some(disk_cache) = &self.disk_cache {
            disk_cache.cleanup_expired().await?;
        }
        Ok(())
    }

    /// Get the cache configuration
    pub fn config(&self) -> &CacheConfig {
        &self.config
    }
}

/// Trait for cache storage backends
#[async_trait]
pub trait Cache {
    /// Get a value from cache
    async fn get<T>(&self, key: &CacheKey) -> SageResult<Option<T>>
    where
        T: serde::de::DeserializeOwned + Clone;

    /// Set a value in cache
    async fn set<T>(&self, key: CacheKey, value: T, ttl: Option<Duration>) -> SageResult<()>
    where
        T: serde::Serialize;

    /// Remove a value from cache
    async fn remove(&self, key: &CacheKey) -> SageResult<()>;

    /// Clear all cache entries
    async fn clear(&self) -> SageResult<()>;

    /// Get cache statistics
    async fn statistics(&self) -> SageResult<CacheStatistics>;
}

impl Default for CacheManager {
    fn default() -> Self {
        let config = CacheConfig::default();
        Self::new(config).expect("Failed to create default cache manager")
    }
}
