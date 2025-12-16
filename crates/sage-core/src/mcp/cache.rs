//! MCP Resource Cache
//!
//! Provides caching for MCP resources, tools, and prompts to reduce
//! redundant requests to MCP servers.

use super::types::{McpPrompt, McpResource, McpResourceContent, McpTool};
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tracing::debug;

/// Cache entry with expiration tracking
#[derive(Debug, Clone)]
pub struct CacheEntry<T> {
    /// The cached value
    pub value: T,
    /// When this entry was created
    pub created_at: Instant,
    /// When this entry expires
    pub expires_at: Option<Instant>,
    /// Number of times this entry has been accessed
    pub hits: u64,
}

impl<T> CacheEntry<T> {
    /// Create a new cache entry
    pub fn new(value: T, ttl: Option<Duration>) -> Self {
        let now = Instant::now();
        Self {
            value,
            created_at: now,
            expires_at: ttl.map(|d| now + d),
            hits: 0,
        }
    }

    /// Check if this entry has expired
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|exp| Instant::now() > exp)
            .unwrap_or(false)
    }

    /// Increment hit counter and return the value
    pub fn get(&mut self) -> &T {
        self.hits += 1;
        &self.value
    }
}

/// Configuration for the resource cache
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Default TTL for cache entries
    pub default_ttl: Option<Duration>,
    /// Maximum number of entries to cache
    pub max_entries: usize,
    /// TTL for tool cache entries
    pub tool_ttl: Option<Duration>,
    /// TTL for resource cache entries
    pub resource_ttl: Option<Duration>,
    /// TTL for prompt cache entries
    pub prompt_ttl: Option<Duration>,
    /// Whether to enable automatic cleanup
    pub auto_cleanup: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            default_ttl: Some(Duration::from_secs(300)), // 5 minutes
            max_entries: 1000,
            tool_ttl: Some(Duration::from_secs(600)), // 10 minutes
            resource_ttl: Some(Duration::from_secs(60)), // 1 minute
            prompt_ttl: Some(Duration::from_secs(300)), // 5 minutes
            auto_cleanup: true,
        }
    }
}

impl CacheConfig {
    /// Create config with no expiration
    pub fn no_expiry() -> Self {
        Self {
            default_ttl: None,
            max_entries: 10000,
            tool_ttl: None,
            resource_ttl: None,
            prompt_ttl: None,
            auto_cleanup: false,
        }
    }

    /// Create config with custom TTL for all entries
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            default_ttl: Some(ttl),
            max_entries: 1000,
            tool_ttl: Some(ttl),
            resource_ttl: Some(ttl),
            prompt_ttl: Some(ttl),
            auto_cleanup: true,
        }
    }
}

/// Cache for MCP resources
pub struct McpCache {
    /// Cached tools by server name
    tools: DashMap<String, CacheEntry<Vec<McpTool>>>,
    /// Cached resources by server name
    resources: DashMap<String, CacheEntry<Vec<McpResource>>>,
    /// Cached prompts by server name
    prompts: DashMap<String, CacheEntry<Vec<McpPrompt>>>,
    /// Cached resource content by URI
    resource_content: DashMap<String, CacheEntry<McpResourceContent>>,
    /// Cache configuration
    config: CacheConfig,
    /// Cache statistics
    stats: CacheStats,
}

/// Cache statistics
#[derive(Debug, Default)]
pub struct CacheStats {
    /// Total cache hits
    pub hits: AtomicU64,
    /// Total cache misses
    pub misses: AtomicU64,
    /// Total cache evictions
    pub evictions: AtomicU64,
}

impl CacheStats {
    /// Get total hits
    pub fn hits(&self) -> u64 {
        self.hits.load(Ordering::Relaxed)
    }

    /// Get total misses
    pub fn misses(&self) -> u64 {
        self.misses.load(Ordering::Relaxed)
    }

    /// Get total evictions
    pub fn evictions(&self) -> u64 {
        self.evictions.load(Ordering::Relaxed)
    }

    /// Get hit rate as a percentage
    pub fn hit_rate(&self) -> f64 {
        let hits = self.hits() as f64;
        let total = hits + self.misses() as f64;
        if total == 0.0 {
            0.0
        } else {
            (hits / total) * 100.0
        }
    }
}

impl McpCache {
    /// Create a new cache with default configuration
    pub fn new() -> Self {
        Self::with_config(CacheConfig::default())
    }

    /// Create a new cache with custom configuration
    pub fn with_config(config: CacheConfig) -> Self {
        Self {
            tools: DashMap::new(),
            resources: DashMap::new(),
            prompts: DashMap::new(),
            resource_content: DashMap::new(),
            config,
            stats: CacheStats::default(),
        }
    }

    // ==========================================================================
    // Tool Cache
    // ==========================================================================

    /// Cache tools for a server
    pub fn cache_tools(&self, server_name: &str, tools: Vec<McpTool>) {
        let entry = CacheEntry::new(tools, self.config.tool_ttl);
        self.tools.insert(server_name.to_string(), entry);
        debug!("Cached tools for server: {}", server_name);
    }

    /// Get cached tools for a server
    pub fn get_tools(&self, server_name: &str) -> Option<Vec<McpTool>> {
        if let Some(mut entry) = self.tools.get_mut(server_name) {
            if entry.is_expired() {
                drop(entry);
                self.tools.remove(server_name);
                self.stats.misses.fetch_add(1, Ordering::Relaxed);
                return None;
            }
            self.stats.hits.fetch_add(1, Ordering::Relaxed);
            Some(entry.get().clone())
        } else {
            self.stats.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    /// Invalidate tools cache for a server
    pub fn invalidate_tools(&self, server_name: &str) {
        if self.tools.remove(server_name).is_some() {
            self.stats.evictions.fetch_add(1, Ordering::Relaxed);
            debug!("Invalidated tools cache for server: {}", server_name);
        }
    }

    // ==========================================================================
    // Resource Cache
    // ==========================================================================

    /// Cache resources for a server
    pub fn cache_resources(&self, server_name: &str, resources: Vec<McpResource>) {
        let entry = CacheEntry::new(resources, self.config.resource_ttl);
        self.resources.insert(server_name.to_string(), entry);
        debug!("Cached resources for server: {}", server_name);
    }

    /// Get cached resources for a server
    pub fn get_resources(&self, server_name: &str) -> Option<Vec<McpResource>> {
        if let Some(mut entry) = self.resources.get_mut(server_name) {
            if entry.is_expired() {
                drop(entry);
                self.resources.remove(server_name);
                self.stats.misses.fetch_add(1, Ordering::Relaxed);
                return None;
            }
            self.stats.hits.fetch_add(1, Ordering::Relaxed);
            Some(entry.get().clone())
        } else {
            self.stats.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    /// Invalidate resources cache for a server
    pub fn invalidate_resources(&self, server_name: &str) {
        if self.resources.remove(server_name).is_some() {
            self.stats.evictions.fetch_add(1, Ordering::Relaxed);
            debug!("Invalidated resources cache for server: {}", server_name);
        }
    }

    // ==========================================================================
    // Resource Content Cache
    // ==========================================================================

    /// Cache resource content
    pub fn cache_resource_content(&self, uri: &str, content: McpResourceContent) {
        let entry = CacheEntry::new(content, self.config.resource_ttl);
        self.resource_content.insert(uri.to_string(), entry);
        debug!("Cached resource content for URI: {}", uri);
    }

    /// Get cached resource content
    pub fn get_resource_content(&self, uri: &str) -> Option<McpResourceContent> {
        if let Some(mut entry) = self.resource_content.get_mut(uri) {
            if entry.is_expired() {
                drop(entry);
                self.resource_content.remove(uri);
                self.stats.misses.fetch_add(1, Ordering::Relaxed);
                return None;
            }
            self.stats.hits.fetch_add(1, Ordering::Relaxed);
            Some(entry.get().clone())
        } else {
            self.stats.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    /// Invalidate resource content cache
    pub fn invalidate_resource_content(&self, uri: &str) {
        if self.resource_content.remove(uri).is_some() {
            self.stats.evictions.fetch_add(1, Ordering::Relaxed);
            debug!("Invalidated resource content cache for URI: {}", uri);
        }
    }

    // ==========================================================================
    // Prompt Cache
    // ==========================================================================

    /// Cache prompts for a server
    pub fn cache_prompts(&self, server_name: &str, prompts: Vec<McpPrompt>) {
        let entry = CacheEntry::new(prompts, self.config.prompt_ttl);
        self.prompts.insert(server_name.to_string(), entry);
        debug!("Cached prompts for server: {}", server_name);
    }

    /// Get cached prompts for a server
    pub fn get_prompts(&self, server_name: &str) -> Option<Vec<McpPrompt>> {
        if let Some(mut entry) = self.prompts.get_mut(server_name) {
            if entry.is_expired() {
                drop(entry);
                self.prompts.remove(server_name);
                self.stats.misses.fetch_add(1, Ordering::Relaxed);
                return None;
            }
            self.stats.hits.fetch_add(1, Ordering::Relaxed);
            Some(entry.get().clone())
        } else {
            self.stats.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    /// Invalidate prompts cache for a server
    pub fn invalidate_prompts(&self, server_name: &str) {
        if self.prompts.remove(server_name).is_some() {
            self.stats.evictions.fetch_add(1, Ordering::Relaxed);
            debug!("Invalidated prompts cache for server: {}", server_name);
        }
    }

    // ==========================================================================
    // Cache Management
    // ==========================================================================

    /// Invalidate all cached data for a server
    pub fn invalidate_server(&self, server_name: &str) {
        self.invalidate_tools(server_name);
        self.invalidate_resources(server_name);
        self.invalidate_prompts(server_name);
        debug!("Invalidated all caches for server: {}", server_name);
    }

    /// Clear all caches
    pub fn clear(&self) {
        let total_evictions = self.tools.len()
            + self.resources.len()
            + self.prompts.len()
            + self.resource_content.len();

        self.tools.clear();
        self.resources.clear();
        self.prompts.clear();
        self.resource_content.clear();

        self.stats
            .evictions
            .fetch_add(total_evictions as u64, Ordering::Relaxed);
        debug!("Cleared all caches ({} entries)", total_evictions);
    }

    /// Remove expired entries from all caches
    pub fn cleanup_expired(&self) {
        let mut evicted = 0;

        // Cleanup tools
        self.tools.retain(|_, entry| {
            if entry.is_expired() {
                evicted += 1;
                false
            } else {
                true
            }
        });

        // Cleanup resources
        self.resources.retain(|_, entry| {
            if entry.is_expired() {
                evicted += 1;
                false
            } else {
                true
            }
        });

        // Cleanup prompts
        self.prompts.retain(|_, entry| {
            if entry.is_expired() {
                evicted += 1;
                false
            } else {
                true
            }
        });

        // Cleanup resource content
        self.resource_content.retain(|_, entry| {
            if entry.is_expired() {
                evicted += 1;
                false
            } else {
                true
            }
        });

        if evicted > 0 {
            self.stats.evictions.fetch_add(evicted, Ordering::Relaxed);
            debug!("Cleaned up {} expired cache entries", evicted);
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }

    /// Get total number of cached entries
    pub fn total_entries(&self) -> usize {
        self.tools.len() + self.resources.len() + self.prompts.len() + self.resource_content.len()
    }

    /// Get cache size breakdown
    pub fn size_breakdown(&self) -> CacheSizeBreakdown {
        CacheSizeBreakdown {
            tools: self.tools.len(),
            resources: self.resources.len(),
            prompts: self.prompts.len(),
            resource_content: self.resource_content.len(),
        }
    }
}

impl Default for McpCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Breakdown of cache sizes by category
#[derive(Debug, Clone)]
pub struct CacheSizeBreakdown {
    /// Number of cached tool lists
    pub tools: usize,
    /// Number of cached resource lists
    pub resources: usize,
    /// Number of cached prompt lists
    pub prompts: usize,
    /// Number of cached resource contents
    pub resource_content: usize,
}

impl CacheSizeBreakdown {
    /// Get total entries
    pub fn total(&self) -> usize {
        self.tools + self.resources + self.prompts + self.resource_content
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_entry_expiration() {
        let entry = CacheEntry::new("test", Some(Duration::from_millis(1)));
        assert!(!entry.is_expired());

        std::thread::sleep(Duration::from_millis(2));
        assert!(entry.is_expired());
    }

    #[test]
    fn test_cache_entry_no_expiration() {
        let entry: CacheEntry<&str> = CacheEntry::new("test", None);
        assert!(!entry.is_expired());
    }

    #[test]
    fn test_cache_tools() {
        let cache = McpCache::new();
        let tools = vec![McpTool::new("test_tool")];

        cache.cache_tools("server1", tools.clone());
        let cached = cache.get_tools("server1");

        assert!(cached.is_some());
        assert_eq!(cached.unwrap().len(), 1);
    }

    #[test]
    fn test_cache_miss() {
        let cache = McpCache::new();
        let result = cache.get_tools("nonexistent");
        assert!(result.is_none());
        assert_eq!(cache.stats.misses(), 1);
    }

    #[test]
    fn test_cache_hit_tracking() {
        let cache = McpCache::new();
        let tools = vec![McpTool::new("test_tool")];

        cache.cache_tools("server1", tools);

        let _ = cache.get_tools("server1");
        let _ = cache.get_tools("server1");

        assert_eq!(cache.stats.hits(), 2);
    }

    #[test]
    fn test_cache_invalidation() {
        let cache = McpCache::new();
        let tools = vec![McpTool::new("test_tool")];

        cache.cache_tools("server1", tools);
        assert!(cache.get_tools("server1").is_some());

        cache.invalidate_tools("server1");
        // Note: get_tools after invalidate counts as a miss
        let result = cache.get_tools("server1");
        assert!(result.is_none());
    }

    #[test]
    fn test_cache_clear() {
        let cache = McpCache::new();
        let tools = vec![McpTool::new("test_tool")];

        cache.cache_tools("server1", tools);
        cache.cache_prompts("server1", vec![]);

        cache.clear();

        assert_eq!(cache.total_entries(), 0);
    }

    #[test]
    fn test_cache_expiration() {
        let config = CacheConfig::with_ttl(Duration::from_millis(1));
        let cache = McpCache::with_config(config);
        let tools = vec![McpTool::new("test_tool")];

        cache.cache_tools("server1", tools);

        std::thread::sleep(Duration::from_millis(2));

        let result = cache.get_tools("server1");
        assert!(result.is_none());
    }

    #[test]
    fn test_cache_stats() {
        let cache = McpCache::new();
        let tools = vec![McpTool::new("test_tool")];

        cache.cache_tools("server1", tools);

        let _ = cache.get_tools("server1"); // hit
        let _ = cache.get_tools("nonexistent"); // miss

        assert_eq!(cache.stats.hits(), 1);
        assert_eq!(cache.stats.misses(), 1);
        assert_eq!(cache.stats.hit_rate(), 50.0);
    }

    #[test]
    fn test_size_breakdown() {
        let cache = McpCache::new();

        cache.cache_tools("server1", vec![]);
        cache.cache_resources("server1", vec![]);
        cache.cache_prompts("server1", vec![]);

        let breakdown = cache.size_breakdown();
        assert_eq!(breakdown.tools, 1);
        assert_eq!(breakdown.resources, 1);
        assert_eq!(breakdown.prompts, 1);
        assert_eq!(breakdown.total(), 3);
    }
}
