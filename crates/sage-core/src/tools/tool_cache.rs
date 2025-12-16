//! Tool result caching
//!
//! This module provides caching for expensive tool operations
//! like file reads, glob searches, and web fetches.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Cache key for tool results
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ToolCacheKey {
    /// Tool name
    pub tool_name: String,
    /// Normalized arguments hash
    pub args_hash: String,
}

impl ToolCacheKey {
    /// Create a new cache key
    pub fn new(tool_name: impl Into<String>, args: &serde_json::Value) -> Self {
        let args_hash = Self::hash_args(args);
        Self {
            tool_name: tool_name.into(),
            args_hash,
        }
    }

    /// Hash arguments to a string
    fn hash_args(args: &serde_json::Value) -> String {
        use std::collections::hash_map::DefaultHasher;

        let canonical = Self::canonicalize_json(args);
        let json_str = serde_json::to_string(&canonical).unwrap_or_default();

        let mut hasher = DefaultHasher::new();
        json_str.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Canonicalize JSON for consistent hashing
    fn canonicalize_json(value: &serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(map) => {
                // Sort keys for consistent ordering
                let mut sorted: Vec<_> = map.iter().collect();
                sorted.sort_by_key(|(k, _)| *k);

                let canonical: serde_json::Map<String, serde_json::Value> = sorted
                    .into_iter()
                    .map(|(k, v)| (k.clone(), Self::canonicalize_json(v)))
                    .collect();

                serde_json::Value::Object(canonical)
            }
            serde_json::Value::Array(arr) => {
                serde_json::Value::Array(arr.iter().map(Self::canonicalize_json).collect())
            }
            other => other.clone(),
        }
    }
}

/// Cached tool result
#[derive(Debug, Clone)]
pub struct CachedResult {
    /// The cached result value
    pub result: String,
    /// Whether the operation was successful
    pub success: bool,
    /// When this was cached
    pub cached_at: Instant,
    /// Time-to-live
    pub ttl: Duration,
    /// Number of times this cache entry was hit
    pub hit_count: u64,
}

impl CachedResult {
    /// Create a new cached result
    pub fn new(result: String, success: bool, ttl: Duration) -> Self {
        Self {
            result,
            success,
            cached_at: Instant::now(),
            ttl,
            hit_count: 0,
        }
    }

    /// Check if the cached result is still valid
    pub fn is_valid(&self) -> bool {
        self.cached_at.elapsed() < self.ttl
    }

    /// Get age of the cache entry
    pub fn age(&self) -> Duration {
        self.cached_at.elapsed()
    }

    /// Time remaining until expiry
    pub fn time_remaining(&self) -> Option<Duration> {
        self.ttl.checked_sub(self.cached_at.elapsed())
    }
}

/// Configuration for tool caching
#[derive(Debug, Clone)]
pub struct ToolCacheConfig {
    /// Maximum cache entries
    pub max_entries: usize,
    /// Default TTL for cache entries
    pub default_ttl: Duration,
    /// TTL overrides per tool
    pub tool_ttls: HashMap<String, Duration>,
    /// Tools to never cache
    pub excluded_tools: Vec<String>,
    /// Maximum result size to cache (bytes)
    pub max_result_size: usize,
}

impl Default for ToolCacheConfig {
    fn default() -> Self {
        let mut tool_ttls = HashMap::new();
        // File reads: short TTL (files may change)
        tool_ttls.insert("Read".to_string(), Duration::from_secs(30));
        // Glob results: medium TTL
        tool_ttls.insert("Glob".to_string(), Duration::from_secs(60));
        // Grep results: medium TTL
        tool_ttls.insert("Grep".to_string(), Duration::from_secs(60));
        // Web fetch: longer TTL
        tool_ttls.insert("WebFetch".to_string(), Duration::from_secs(300));
        // Web search: longer TTL
        tool_ttls.insert("WebSearch".to_string(), Duration::from_secs(600));

        Self {
            max_entries: 1000,
            default_ttl: Duration::from_secs(120),
            tool_ttls,
            excluded_tools: vec![
                "Bash".to_string(),    // Commands have side effects
                "Write".to_string(),   // Writes have side effects
                "Edit".to_string(),    // Edits have side effects
            ],
            max_result_size: 1024 * 1024, // 1MB
        }
    }
}

impl ToolCacheConfig {
    /// Create a config with no caching
    pub fn disabled() -> Self {
        Self {
            max_entries: 0,
            default_ttl: Duration::ZERO,
            tool_ttls: HashMap::new(),
            excluded_tools: Vec::new(),
            max_result_size: 0,
        }
    }

    /// Create an aggressive caching config
    pub fn aggressive() -> Self {
        let mut config = Self::default();
        config.max_entries = 5000;
        config.default_ttl = Duration::from_secs(600);
        config
    }

    /// Set default TTL
    pub fn with_default_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = ttl;
        self
    }

    /// Set TTL for a specific tool
    pub fn with_tool_ttl(mut self, tool: impl Into<String>, ttl: Duration) -> Self {
        self.tool_ttls.insert(tool.into(), ttl);
        self
    }

    /// Exclude a tool from caching
    pub fn exclude_tool(mut self, tool: impl Into<String>) -> Self {
        self.excluded_tools.push(tool.into());
        self
    }

    /// Get TTL for a tool
    pub fn ttl_for_tool(&self, tool: &str) -> Duration {
        self.tool_ttls.get(tool).cloned().unwrap_or(self.default_ttl)
    }

    /// Check if a tool should be cached
    pub fn should_cache(&self, tool: &str) -> bool {
        !self.excluded_tools.iter().any(|t| t.eq_ignore_ascii_case(tool))
    }
}

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

/// Cache statistics
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Cache hits
    pub hits: u64,
    /// Cache misses
    pub misses: u64,
    /// Insertions
    pub inserts: u64,
    /// Expirations
    pub expirations: u64,
    /// Manual clears
    pub clears: u64,
}

impl CacheStats {
    /// Calculate hit rate
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// Format stats as summary string
    pub fn summary(&self) -> String {
        format!(
            "hits: {}, misses: {}, hit rate: {:.1}%, inserts: {}",
            self.hits,
            self.misses,
            self.hit_rate() * 100.0,
            self.inserts
        )
    }
}

/// Thread-safe shared tool cache
pub type SharedToolCache = Arc<ToolCache>;

/// Create a shared tool cache
pub fn create_shared_cache(config: ToolCacheConfig) -> SharedToolCache {
    Arc::new(ToolCache::new(config))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_cache_key_creation() {
        let args = json!({"path": "/test/file.txt"});
        let key = ToolCacheKey::new("Read", &args);

        assert_eq!(key.tool_name, "Read");
        assert!(!key.args_hash.is_empty());
    }

    #[test]
    fn test_cache_key_consistency() {
        let args1 = json!({"a": 1, "b": 2});
        let args2 = json!({"b": 2, "a": 1}); // Same content, different order

        let key1 = ToolCacheKey::new("Test", &args1);
        let key2 = ToolCacheKey::new("Test", &args2);

        // Keys should be the same (canonicalized)
        assert_eq!(key1.args_hash, key2.args_hash);
    }

    #[test]
    fn test_cached_result_validity() {
        let result = CachedResult::new("test".to_string(), true, Duration::from_millis(100));
        assert!(result.is_valid());

        std::thread::sleep(Duration::from_millis(150));
        assert!(!result.is_valid());
    }

    #[test]
    fn test_cached_result_time_remaining() {
        let result = CachedResult::new("test".to_string(), true, Duration::from_secs(10));
        let remaining = result.time_remaining();
        assert!(remaining.is_some());
        assert!(remaining.unwrap() > Duration::from_secs(9));
    }

    #[tokio::test]
    async fn test_cache_set_get() {
        let cache = ToolCache::with_defaults();
        let key = ToolCacheKey::new("Read", &json!({"path": "/test"}));

        cache.set(key.clone(), "content".to_string(), true).await;

        let result = cache.get(&key).await;
        assert!(result.is_some());
        assert_eq!(result.unwrap().result, "content");
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let cache = ToolCache::with_defaults();
        let key = ToolCacheKey::new("Read", &json!({"path": "/nonexistent"}));

        let result = cache.get(&key).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let config = ToolCacheConfig::default().with_default_ttl(Duration::from_millis(50));
        let cache = ToolCache::new(config);
        let key = ToolCacheKey::new("Test", &json!({}));

        cache.set(key.clone(), "test".to_string(), true).await;
        assert!(cache.get(&key).await.is_some());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(cache.get(&key).await.is_none());
    }

    #[tokio::test]
    async fn test_cache_excluded_tool() {
        let cache = ToolCache::with_defaults();
        let key = ToolCacheKey::new("Bash", &json!({"command": "ls"}));

        cache.set(key.clone(), "output".to_string(), true).await;

        // Bash is excluded, so get should return None
        let result = cache.get(&key).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_cache_invalidate_tool() {
        let cache = ToolCache::with_defaults();

        cache.set(ToolCacheKey::new("Read", &json!({"path": "a"})), "a".to_string(), true).await;
        cache.set(ToolCacheKey::new("Read", &json!({"path": "b"})), "b".to_string(), true).await;
        cache.set(ToolCacheKey::new("Glob", &json!({"pattern": "*"})), "glob".to_string(), true).await;

        assert_eq!(cache.len().await, 3);

        cache.invalidate_tool("Read").await;

        assert_eq!(cache.len().await, 1);
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let cache = ToolCache::with_defaults();
        let key = ToolCacheKey::new("Read", &json!({"path": "/test"}));

        cache.set(key.clone(), "content".to_string(), true).await;
        assert!(!cache.is_empty().await);

        cache.clear().await;
        assert!(cache.is_empty().await);
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let cache = ToolCache::with_defaults();
        let key = ToolCacheKey::new("Read", &json!({"path": "/test"}));

        // Miss
        cache.get(&key).await;

        // Insert
        cache.set(key.clone(), "content".to_string(), true).await;

        // Hit
        cache.get(&key).await;
        cache.get(&key).await;

        let stats = cache.stats().await;
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.inserts, 1);
        assert!((stats.hit_rate() - 0.666).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_cache_cleanup_expired() {
        let config = ToolCacheConfig::default().with_default_ttl(Duration::from_millis(50));
        let cache = ToolCache::new(config);

        cache.set(ToolCacheKey::new("Test", &json!({"a": 1})), "1".to_string(), true).await;
        cache.set(ToolCacheKey::new("Test", &json!({"a": 2})), "2".to_string(), true).await;

        assert_eq!(cache.len().await, 2);

        tokio::time::sleep(Duration::from_millis(100)).await;

        let removed = cache.cleanup_expired().await;
        assert_eq!(removed, 2);
        assert!(cache.is_empty().await);
    }

    #[tokio::test]
    async fn test_cache_max_entries() {
        let config = ToolCacheConfig {
            max_entries: 2,
            ..Default::default()
        };
        let cache = ToolCache::new(config);

        cache.set(ToolCacheKey::new("Read", &json!({"a": 1})), "1".to_string(), true).await;
        cache.set(ToolCacheKey::new("Read", &json!({"a": 2})), "2".to_string(), true).await;
        cache.set(ToolCacheKey::new("Read", &json!({"a": 3})), "3".to_string(), true).await;

        // Should only keep max_entries
        assert_eq!(cache.len().await, 2);
    }

    #[tokio::test]
    async fn test_cache_max_result_size() {
        let config = ToolCacheConfig {
            max_result_size: 10,
            ..Default::default()
        };
        let cache = ToolCache::new(config);

        // Small result should be cached
        cache.set(ToolCacheKey::new("Read", &json!({"a": 1})), "small".to_string(), true).await;
        assert_eq!(cache.len().await, 1);

        // Large result should not be cached
        cache.set(ToolCacheKey::new("Read", &json!({"a": 2})), "this is a very large result".to_string(), true).await;
        assert_eq!(cache.len().await, 1);
    }

    #[test]
    fn test_config_tool_ttl() {
        let config = ToolCacheConfig::default();

        // Read has custom TTL
        assert_eq!(config.ttl_for_tool("Read"), Duration::from_secs(30));

        // Unknown tool uses default
        assert_eq!(config.ttl_for_tool("Unknown"), config.default_ttl);
    }

    #[test]
    fn test_config_should_cache() {
        let config = ToolCacheConfig::default();

        assert!(config.should_cache("Read"));
        assert!(config.should_cache("Glob"));
        assert!(!config.should_cache("Bash"));
        assert!(!config.should_cache("Write"));
    }

    #[test]
    fn test_config_builder() {
        let config = ToolCacheConfig::default()
            .with_default_ttl(Duration::from_secs(60))
            .with_tool_ttl("Custom", Duration::from_secs(30))
            .exclude_tool("ExcludedTool");

        assert_eq!(config.default_ttl, Duration::from_secs(60));
        assert_eq!(config.ttl_for_tool("Custom"), Duration::from_secs(30));
        assert!(!config.should_cache("ExcludedTool"));
    }

    #[test]
    fn test_cache_stats_summary() {
        let stats = CacheStats {
            hits: 80,
            misses: 20,
            inserts: 100,
            expirations: 10,
            clears: 1,
        };

        let summary = stats.summary();
        assert!(summary.contains("80"));
        assert!(summary.contains("80.0%"));
    }

    #[tokio::test]
    async fn test_hit_count_tracking() {
        let cache = ToolCache::with_defaults();
        let key = ToolCacheKey::new("Read", &json!({"path": "/test"}));

        cache.set(key.clone(), "content".to_string(), true).await;

        cache.get(&key).await;
        cache.get(&key).await;
        cache.get(&key).await;

        let result = cache.get(&key).await.unwrap();
        assert_eq!(result.hit_count, 4); // Including this get
    }
}
