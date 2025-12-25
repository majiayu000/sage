//! Cache types and key structures

use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::storage::ToolCache;

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

/// Thread-safe shared tool cache
pub type SharedToolCache = Arc<ToolCache>;
