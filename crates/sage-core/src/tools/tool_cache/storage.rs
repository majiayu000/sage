//! Cache storage and operations

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::config::ToolCacheConfig;
use super::stats::CacheStats;
use super::types::{CachedResult, ToolCacheKey};

/// Tool result cache
#[derive(Debug)]
pub struct ToolCache {
    /// Configuration
    config: ToolCacheConfig,
    /// Cache entries
    entries: Arc<RwLock<HashMap<ToolCacheKey, CachedResult>>>,
    /// Statistics
    stats: Arc<RwLock<CacheStats>>,
}

impl ToolCache {
    /// Create a new tool cache
    pub fn new(config: ToolCacheConfig) -> Self {
        Self {
            config,
            entries: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }

    /// Create with default config
    pub fn with_defaults() -> Self {
        Self::new(ToolCacheConfig::default())
    }

    /// Get a cached result
    pub async fn get(&self, key: &ToolCacheKey) -> Option<CachedResult> {
        if !self.config.should_cache(&key.tool_name) {
            return None;
        }

        let mut entries = self.entries.write().await;

        if let Some(entry) = entries.get_mut(key) {
            if entry.is_valid() {
                entry.hit_count += 1;
                self.stats.write().await.hits += 1;
                return Some(entry.clone());
            } else {
                // Remove expired entry
                entries.remove(key);
                self.stats.write().await.expirations += 1;
            }
        }

        self.stats.write().await.misses += 1;
        None
    }

    /// Set a cached result
    pub async fn set(&self, key: ToolCacheKey, result: String, success: bool) {
        if !self.config.should_cache(&key.tool_name) {
            return;
        }

        // Check result size
        if result.len() > self.config.max_result_size {
            return;
        }

        let ttl = self.config.ttl_for_tool(&key.tool_name);
        let cached = CachedResult::new(result, success, ttl);

        let mut entries = self.entries.write().await;

        // Evict if at capacity
        if entries.len() >= self.config.max_entries {
            self.evict_oldest(&mut entries);
        }

        entries.insert(key, cached);
        self.stats.write().await.inserts += 1;
    }

    /// Evict the oldest entry
    fn evict_oldest(&self, entries: &mut HashMap<ToolCacheKey, CachedResult>) {
        // Find oldest entry
        let oldest_key = entries
            .iter()
            .min_by_key(|(_, v)| v.cached_at)
            .map(|(k, _)| k.clone());

        if let Some(key) = oldest_key {
            entries.remove(&key);
            // Can't update stats here as we already have a mutable borrow
        }
    }

    /// Invalidate cache for a tool
    pub async fn invalidate_tool(&self, tool_name: &str) {
        let mut entries = self.entries.write().await;
        entries.retain(|k, _| !k.tool_name.eq_ignore_ascii_case(tool_name));
    }

    /// Invalidate cache entries matching a pattern
    pub async fn invalidate_matching(&self, pattern: impl Fn(&ToolCacheKey) -> bool) {
        let mut entries = self.entries.write().await;
        entries.retain(|k, _| !pattern(k));
    }

    /// Clear all cache entries
    pub async fn clear(&self) {
        self.entries.write().await.clear();
        self.stats.write().await.clears += 1;
    }

    /// Remove expired entries
    pub async fn cleanup_expired(&self) -> usize {
        let mut entries = self.entries.write().await;
        let before = entries.len();
        entries.retain(|_, v| v.is_valid());
        let removed = before - entries.len();

        if removed > 0 {
            self.stats.write().await.expirations += removed as u64;
        }

        removed
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        self.stats.read().await.clone()
    }

    /// Get current entry count
    pub async fn len(&self) -> usize {
        self.entries.read().await.len()
    }

    /// Check if cache is empty
    pub async fn is_empty(&self) -> bool {
        self.entries.read().await.is_empty()
    }

    /// Get configuration
    pub fn config(&self) -> &ToolCacheConfig {
        &self.config
    }
}

impl Default for ToolCache {
    fn default() -> Self {
        Self::with_defaults()
    }
}
