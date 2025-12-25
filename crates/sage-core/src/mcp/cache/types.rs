//! Cache types and configuration

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

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
