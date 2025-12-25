//! Core types for conversation caching

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Cache TTL for conversation prefixes (5 minutes, matching Anthropic's cache TTL)
pub const DEFAULT_CACHE_TTL_SECS: i64 = 300;

/// Extended cache TTL (1 hour, available at additional cost)
pub const EXTENDED_CACHE_TTL_SECS: i64 = 3600;

/// Configuration for conversation caching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationCacheConfig {
    /// Whether conversation caching is enabled
    pub enabled: bool,
    /// Use extended TTL (1 hour instead of 5 minutes)
    pub use_extended_ttl: bool,
    /// Maximum number of cached conversations to track
    pub max_cached_conversations: usize,
    /// Maximum cache checkpoints per conversation
    pub max_checkpoints_per_conversation: usize,
    /// Minimum tokens required for caching (provider-specific)
    pub min_tokens_for_cache: usize,
}

impl Default for ConversationCacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            use_extended_ttl: false,
            max_cached_conversations: 100,
            max_checkpoints_per_conversation: 10,
            min_tokens_for_cache: 1024, // Claude 3.5 Sonnet minimum
        }
    }
}

impl ConversationCacheConfig {
    /// Create config for Anthropic Claude models
    pub fn for_anthropic(model: &str) -> Self {
        let min_tokens = if model.contains("haiku") {
            2048 // Claude Haiku requires 2048 minimum
        } else {
            1024 // Claude Sonnet/Opus require 1024 minimum
        };

        Self {
            enabled: true,
            use_extended_ttl: false,
            max_cached_conversations: 100,
            max_checkpoints_per_conversation: 10,
            min_tokens_for_cache: min_tokens,
        }
    }

    /// Get the cache TTL in seconds
    pub fn cache_ttl_secs(&self) -> i64 {
        if self.use_extended_ttl {
            EXTENDED_CACHE_TTL_SECS
        } else {
            DEFAULT_CACHE_TTL_SECS
        }
    }
}

/// A cache checkpoint representing a cached message prefix
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheCheckpoint {
    /// Hash of the message prefix (for quick comparison)
    pub prefix_hash: String,
    /// Number of messages in this checkpoint
    pub message_count: usize,
    /// Estimated token count at this checkpoint
    pub token_count: usize,
    /// When this checkpoint was created
    pub created_at: DateTime<Utc>,
    /// When this checkpoint was last accessed
    pub last_accessed: DateTime<Utc>,
    /// Number of times this checkpoint was used
    pub hit_count: u64,
}

impl CacheCheckpoint {
    /// Create a new cache checkpoint
    pub fn new(prefix_hash: String, message_count: usize, token_count: usize) -> Self {
        let now = Utc::now();
        Self {
            prefix_hash,
            message_count,
            token_count,
            created_at: now,
            last_accessed: now,
            hit_count: 0,
        }
    }

    /// Check if this checkpoint has expired
    pub fn is_expired(&self, ttl_secs: i64) -> bool {
        let elapsed = Utc::now()
            .signed_duration_since(self.last_accessed)
            .num_seconds();
        elapsed > ttl_secs
    }

    /// Mark this checkpoint as accessed (refreshes TTL)
    pub fn touch(&mut self) {
        self.last_accessed = Utc::now();
        self.hit_count += 1;
    }
}

/// Statistics for conversation caching
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConversationCacheStats {
    /// Total cache hits
    pub total_hits: u64,
    /// Total cache misses
    pub total_misses: u64,
    /// Total checkpoints created
    pub checkpoints_created: u64,
    /// Total checkpoints expired
    pub checkpoints_expired: u64,
    /// Estimated tokens saved
    pub tokens_saved: u64,
    /// Estimated cost saved (USD)
    pub cost_saved_usd: f64,
}

impl ConversationCacheStats {
    /// Get cache hit rate
    pub fn hit_rate(&self) -> f64 {
        let total = self.total_hits + self.total_misses;
        if total == 0 {
            0.0
        } else {
            self.total_hits as f64 / total as f64
        }
    }
}

/// Result of a cache lookup
#[derive(Debug, Clone)]
pub struct CacheLookupResult {
    /// Number of messages that are cached
    pub cached_message_count: usize,
    /// Number of tokens that are cached
    pub cached_token_count: usize,
    /// Hash of the cached checkpoint
    pub checkpoint_hash: String,
    /// Number of times this checkpoint has been hit
    pub hit_count: u64,
}
