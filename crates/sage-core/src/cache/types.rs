//! Cache types and data structures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;

/// Cache key for identifying cached entries
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CacheKey {
    /// Key namespace (e.g., "llm_response", "tool_result")
    pub namespace: String,
    /// Unique identifier within the namespace
    pub identifier: String,
    /// Hash of the key for fast comparison
    pub hash: u64,
}

impl CacheKey {
    /// Create a new cache key
    pub fn new(namespace: impl Into<String>, identifier: impl Into<String>) -> Self {
        let namespace = namespace.into();
        let identifier = identifier.into();
        
        // Generate hash for fast comparison
        let mut hasher = DefaultHasher::new();
        namespace.hash(&mut hasher);
        identifier.hash(&mut hasher);
        let hash = hasher.finish();

        Self {
            namespace,
            identifier,
            hash,
        }
    }

    /// Create a cache key for LLM responses
    pub fn llm_response(
        provider: &str,
        model: &str,
        messages_hash: u64,
        tools_hash: Option<u64>,
    ) -> Self {
        let identifier = format!(
            "{}:{}:{}:{}",
            provider,
            model,
            messages_hash,
            tools_hash.unwrap_or(0)
        );
        Self::new("llm_response", identifier)
    }

    /// Create a cache key for tool results
    pub fn tool_result(tool_name: &str, parameters_hash: u64) -> Self {
        let identifier = format!("{}:{}", tool_name, parameters_hash);
        Self::new("tool_result", identifier)
    }

    /// Create a cache key for codebase retrieval
    pub fn codebase_retrieval(query_hash: u64, context_hash: u64) -> Self {
        let identifier = format!("{}:{}", query_hash, context_hash);
        Self::new("codebase_retrieval", identifier)
    }
}

/// Cache entry containing data and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Cached data
    pub data: serde_json::Value,
    /// When the entry was created
    pub created_at: DateTime<Utc>,
    /// When the entry expires (None means no expiration)
    pub expires_at: Option<DateTime<Utc>>,
    /// Size of the entry in bytes
    pub size_bytes: usize,
    /// Number of times this entry has been accessed
    pub access_count: u64,
    /// Last access time
    pub last_accessed: DateTime<Utc>,
}

impl CacheEntry {
    /// Create a new cache entry
    pub fn new(data: serde_json::Value, ttl: Option<Duration>) -> Self {
        let now = Utc::now();
        let expires_at = ttl.map(|duration| now + chrono::Duration::from_std(duration).unwrap());
        let size_bytes = data.to_string().len();

        Self {
            data,
            created_at: now,
            expires_at,
            size_bytes,
            access_count: 0,
            last_accessed: now,
        }
    }

    /// Check if the entry has expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() > expires_at
        } else {
            false
        }
    }

    /// Mark the entry as accessed
    pub fn mark_accessed(&mut self) {
        self.access_count += 1;
        self.last_accessed = Utc::now();
    }

    /// Get the age of the entry
    pub fn age(&self) -> chrono::Duration {
        Utc::now() - self.created_at
    }

    /// Get time until expiration
    pub fn time_to_expiry(&self) -> Option<chrono::Duration> {
        self.expires_at.map(|expires_at| expires_at - Utc::now())
    }
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Enable memory cache
    pub enable_memory_cache: bool,
    /// Memory cache capacity (number of entries)
    pub memory_capacity: usize,
    /// Enable disk cache
    pub enable_disk_cache: bool,
    /// Disk cache directory
    pub disk_cache_dir: String,
    /// Disk cache capacity in bytes
    pub disk_capacity: u64,
    /// Default TTL for cache entries
    pub default_ttl: Option<Duration>,
    /// TTL for LLM responses
    pub llm_response_ttl: Option<Duration>,
    /// TTL for tool results
    pub tool_result_ttl: Option<Duration>,
    /// TTL for codebase retrieval results
    pub codebase_retrieval_ttl: Option<Duration>,
    /// Cleanup interval for expired entries
    pub cleanup_interval: Duration,
    /// Maximum entry size in bytes
    pub max_entry_size: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enable_memory_cache: true,
            memory_capacity: 1000,
            enable_disk_cache: true,
            disk_cache_dir: "cache".to_string(),
            disk_capacity: 100 * 1024 * 1024, // 100MB
            default_ttl: Some(Duration::from_secs(3600)), // 1 hour
            llm_response_ttl: Some(Duration::from_secs(7200)), // 2 hours
            tool_result_ttl: Some(Duration::from_secs(1800)), // 30 minutes
            codebase_retrieval_ttl: Some(Duration::from_secs(3600)), // 1 hour
            cleanup_interval: Duration::from_secs(300), // 5 minutes
            max_entry_size: 1024 * 1024, // 1MB
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatistics {
    /// Memory cache statistics
    pub memory_stats: StorageStatistics,
    /// Disk cache statistics (if enabled)
    pub disk_stats: Option<StorageStatistics>,
    /// Total cache hits across all layers
    pub total_hits: u64,
    /// Total cache misses across all layers
    pub total_misses: u64,
}

impl CacheStatistics {
    /// Calculate hit rate
    pub fn hit_rate(&self) -> f64 {
        let total_requests = self.total_hits + self.total_misses;
        if total_requests == 0 {
            0.0
        } else {
            self.total_hits as f64 / total_requests as f64
        }
    }

    /// Get total entries across all cache layers
    pub fn total_entries(&self) -> usize {
        self.memory_stats.entry_count + 
        self.disk_stats.as_ref().map(|s| s.entry_count).unwrap_or(0)
    }

    /// Get total size across all cache layers
    pub fn total_size_bytes(&self) -> u64 {
        self.memory_stats.size_bytes + 
        self.disk_stats.as_ref().map(|s| s.size_bytes).unwrap_or(0)
    }
}

/// Storage layer statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStatistics {
    /// Number of entries in storage
    pub entry_count: usize,
    /// Total size in bytes
    pub size_bytes: u64,
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
    /// Number of evictions
    pub evictions: u64,
}

impl Default for StorageStatistics {
    fn default() -> Self {
        Self {
            entry_count: 0,
            size_bytes: 0,
            hits: 0,
            misses: 0,
            evictions: 0,
        }
    }
}

/// Hash helper functions
pub mod hash_utils {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    /// Generate hash for any hashable type
    pub fn hash_value<T: Hash + ?Sized>(value: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    /// Generate hash for a slice of messages
    pub fn hash_messages(messages: &[crate::llm::LLMMessage]) -> u64 {
        let mut hasher = DefaultHasher::new();
        for message in messages {
            // Hash the role and content
            message.role.hash(&mut hasher);
            message.content.hash(&mut hasher);
        }
        hasher.finish()
    }

    /// Generate hash for tool schemas
    pub fn hash_tools(tools: &[crate::tools::ToolSchema]) -> u64 {
        let mut hasher = DefaultHasher::new();
        for tool in tools {
            tool.name.hash(&mut hasher);
            tool.description.hash(&mut hasher);
            // Hash the schema as a string since it's JSON
            if let Ok(schema_str) = serde_json::to_string(&tool.parameters) {
                schema_str.hash(&mut hasher);
            }
        }
        hasher.finish()
    }

    /// Generate hash for tool parameters
    pub fn hash_parameters(params: &serde_json::Value) -> u64 {
        hash_value(&params.to_string())
    }
}
