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

use crate::error::SageResult;
use crate::llm::LlmMessage;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Cache TTL for conversation prefixes (5 minutes, matching Anthropic's cache TTL)
const DEFAULT_CACHE_TTL_SECS: i64 = 300;

/// Extended cache TTL (1 hour, available at additional cost)
const EXTENDED_CACHE_TTL_SECS: i64 = 3600;

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

/// Cached conversation state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedConversation {
    /// Conversation ID
    pub conversation_id: String,
    /// Cache checkpoints (ordered by message count)
    pub checkpoints: Vec<CacheCheckpoint>,
    /// When this conversation cache was created
    pub created_at: DateTime<Utc>,
    /// Total cache hits for this conversation
    pub total_hits: u64,
    /// Total cache misses for this conversation
    pub total_misses: u64,
}

impl CachedConversation {
    /// Create a new cached conversation
    pub fn new(conversation_id: String) -> Self {
        Self {
            conversation_id,
            checkpoints: Vec::new(),
            created_at: Utc::now(),
            total_hits: 0,
            total_misses: 0,
        }
    }

    /// Add a new checkpoint
    pub fn add_checkpoint(&mut self, checkpoint: CacheCheckpoint, max_checkpoints: usize) {
        // Remove expired checkpoints first
        self.checkpoints
            .retain(|cp| !cp.is_expired(DEFAULT_CACHE_TTL_SECS));

        // Add new checkpoint
        self.checkpoints.push(checkpoint);

        // Sort by message count (descending) for efficient lookup
        self.checkpoints
            .sort_by(|a, b| b.message_count.cmp(&a.message_count));

        // Trim to max checkpoints
        if self.checkpoints.len() > max_checkpoints {
            self.checkpoints.truncate(max_checkpoints);
        }
    }

    /// Find the longest cached prefix for the given messages
    pub fn find_longest_cached_prefix(
        &mut self,
        messages: &[LlmMessage],
        ttl_secs: i64,
    ) -> Option<&CacheCheckpoint> {
        // Compute prefix hashes for all message counts
        let prefix_hashes: HashMap<usize, String> = (1..=messages.len())
            .map(|n| (n, compute_prefix_hash(&messages[..n])))
            .collect();

        // Find the longest matching checkpoint
        for checkpoint in &mut self.checkpoints {
            if checkpoint.is_expired(ttl_secs) {
                continue;
            }

            if let Some(hash) = prefix_hashes.get(&checkpoint.message_count) {
                if hash == &checkpoint.prefix_hash {
                    checkpoint.touch();
                    self.total_hits += 1;
                    return Some(checkpoint);
                }
            }
        }

        self.total_misses += 1;
        None
    }

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

/// Conversation cache manager
///
/// Tracks cached conversation prefixes across multiple conversations
/// and provides efficient cache lookup for incremental caching.
#[derive(Debug)]
pub struct ConversationCache {
    /// Configuration
    config: ConversationCacheConfig,
    /// Cached conversations by ID
    conversations: Arc<RwLock<HashMap<String, CachedConversation>>>,
    /// Global statistics
    stats: Arc<RwLock<ConversationCacheStats>>,
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

impl ConversationCache {
    /// Create a new conversation cache
    pub fn new(config: ConversationCacheConfig) -> Self {
        Self {
            config,
            conversations: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(ConversationCacheStats::default())),
        }
    }

    /// Create with default configuration
    pub fn default_for_anthropic(model: &str) -> Self {
        Self::new(ConversationCacheConfig::for_anthropic(model))
    }

    /// Check if caching is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get the configuration
    pub fn config(&self) -> &ConversationCacheConfig {
        &self.config
    }

    /// Find the longest cached prefix for a conversation
    ///
    /// Returns the cache checkpoint if found, along with the number of
    /// messages that are cached.
    pub async fn find_cached_prefix(
        &self,
        conversation_id: &str,
        messages: &[LlmMessage],
    ) -> SageResult<Option<CacheLookupResult>> {
        if !self.config.enabled {
            return Ok(None);
        }

        let mut conversations = self.conversations.write().await;
        let ttl_secs = self.config.cache_ttl_secs();

        if let Some(conversation) = conversations.get_mut(conversation_id) {
            if let Some(checkpoint) = conversation.find_longest_cached_prefix(messages, ttl_secs) {
                let mut stats = self.stats.write().await;
                stats.total_hits += 1;
                stats.tokens_saved += checkpoint.token_count as u64;
                // Estimate cost saved (assuming $3/1M input tokens, 90% savings)
                stats.cost_saved_usd += (checkpoint.token_count as f64 / 1_000_000.0) * 3.0 * 0.9;

                return Ok(Some(CacheLookupResult {
                    cached_message_count: checkpoint.message_count,
                    cached_token_count: checkpoint.token_count,
                    checkpoint_hash: checkpoint.prefix_hash.clone(),
                    hit_count: checkpoint.hit_count,
                }));
            }
        }

        let mut stats = self.stats.write().await;
        stats.total_misses += 1;
        Ok(None)
    }

    /// Record a new cache checkpoint
    ///
    /// Call this after a successful LLM response to cache the message prefix.
    pub async fn record_checkpoint(
        &self,
        conversation_id: &str,
        messages: &[LlmMessage],
        token_count: usize,
    ) -> SageResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Check minimum token requirement
        if token_count < self.config.min_tokens_for_cache {
            tracing::debug!(
                "Skipping cache checkpoint: {} tokens < {} minimum",
                token_count,
                self.config.min_tokens_for_cache
            );
            return Ok(());
        }

        let prefix_hash = compute_prefix_hash(messages);
        let checkpoint = CacheCheckpoint::new(prefix_hash, messages.len(), token_count);

        let mut conversations = self.conversations.write().await;

        // Cleanup old conversations if at capacity
        if conversations.len() >= self.config.max_cached_conversations {
            self.cleanup_oldest_conversations(&mut conversations).await;
        }

        // Get or create conversation entry
        let conversation = conversations
            .entry(conversation_id.to_string())
            .or_insert_with(|| CachedConversation::new(conversation_id.to_string()));

        conversation.add_checkpoint(checkpoint, self.config.max_checkpoints_per_conversation);

        let mut stats = self.stats.write().await;
        stats.checkpoints_created += 1;

        tracing::debug!(
            "Recorded cache checkpoint for conversation {}: {} messages, {} tokens",
            conversation_id,
            messages.len(),
            token_count
        );

        Ok(())
    }

    /// Clear cache for a specific conversation
    pub async fn clear_conversation(&self, conversation_id: &str) -> SageResult<()> {
        let mut conversations = self.conversations.write().await;
        conversations.remove(conversation_id);
        Ok(())
    }

    /// Clear all cached conversations
    pub async fn clear_all(&self) -> SageResult<()> {
        let mut conversations = self.conversations.write().await;
        conversations.clear();

        let mut stats = self.stats.write().await;
        *stats = ConversationCacheStats::default();

        Ok(())
    }

    /// Get cache statistics
    pub async fn statistics(&self) -> ConversationCacheStats {
        self.stats.read().await.clone()
    }

    /// Get conversation count
    pub async fn conversation_count(&self) -> usize {
        self.conversations.read().await.len()
    }

    /// Cleanup expired checkpoints across all conversations
    pub async fn cleanup_expired(&self) -> SageResult<u64> {
        let mut conversations = self.conversations.write().await;
        let ttl_secs = self.config.cache_ttl_secs();
        let mut expired_count = 0u64;

        for conversation in conversations.values_mut() {
            let before_count = conversation.checkpoints.len();
            conversation
                .checkpoints
                .retain(|cp| !cp.is_expired(ttl_secs));
            expired_count += (before_count - conversation.checkpoints.len()) as u64;
        }

        // Remove empty conversations
        conversations.retain(|_, conv| !conv.checkpoints.is_empty());

        if expired_count > 0 {
            let mut stats = self.stats.write().await;
            stats.checkpoints_expired += expired_count;
        }

        Ok(expired_count)
    }

    /// Cleanup oldest conversations when at capacity
    async fn cleanup_oldest_conversations(
        &self,
        conversations: &mut HashMap<String, CachedConversation>,
    ) {
        // Remove conversations with lowest hit rates
        let mut conv_stats: Vec<_> = conversations
            .iter()
            .map(|(id, conv)| (id.clone(), conv.hit_rate(), conv.created_at))
            .collect();

        // Sort by hit rate (ascending), then by creation time (oldest first)
        conv_stats.sort_by(|a, b| {
            a.1.partial_cmp(&b.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.2.cmp(&b.2))
        });

        // Remove the bottom 10%
        let to_remove = (conversations.len() / 10).max(1);
        for (id, _, _) in conv_stats.into_iter().take(to_remove) {
            conversations.remove(&id);
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

/// Compute a hash for a message prefix
fn compute_prefix_hash(messages: &[LlmMessage]) -> String {
    let mut hasher = Sha256::new();

    for msg in messages {
        hasher.update(msg.role.to_string().as_bytes());
        hasher.update(msg.content.as_bytes());
        if let Some(name) = &msg.name {
            hasher.update(name.as_bytes());
        }
        if let Some(tool_call_id) = &msg.tool_call_id {
            hasher.update(tool_call_id.as_bytes());
        }
    }

    format!("{:x}", hasher.finalize())
}

impl Default for ConversationCache {
    fn default() -> Self {
        Self::new(ConversationCacheConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::MessageRole;
    use std::collections::HashMap;

    fn create_message(role: MessageRole, content: &str) -> LlmMessage {
        LlmMessage {
            role,
            content: content.to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
            metadata: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_conversation_cache_basic() {
        let cache = ConversationCache::default();
        let conv_id = "test-conv-1";

        let messages = vec![
            create_message(MessageRole::System, "You are a helpful assistant."),
            create_message(MessageRole::User, "Hello!"),
            create_message(MessageRole::Assistant, "Hi there! How can I help you?"),
        ];

        // Initially, no cache
        let result = cache.find_cached_prefix(conv_id, &messages).await.unwrap();
        assert!(result.is_none());

        // Record checkpoint
        cache
            .record_checkpoint(conv_id, &messages, 2000)
            .await
            .unwrap();

        // Now should find cache
        let result = cache.find_cached_prefix(conv_id, &messages).await.unwrap();
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.cached_message_count, 3);
        assert_eq!(result.cached_token_count, 2000);
    }

    #[tokio::test]
    async fn test_incremental_caching() {
        let cache = ConversationCache::default();
        let conv_id = "test-conv-2";

        let messages_v1 = vec![
            create_message(MessageRole::System, "You are a helpful assistant."),
            create_message(MessageRole::User, "Hello!"),
        ];

        // Record first checkpoint
        cache
            .record_checkpoint(conv_id, &messages_v1, 1500)
            .await
            .unwrap();

        // Add more messages
        let mut messages_v2 = messages_v1.clone();
        messages_v2.push(create_message(
            MessageRole::Assistant,
            "Hi there! How can I help?",
        ));
        messages_v2.push(create_message(MessageRole::User, "What's the weather?"));

        // Should find the original cached prefix
        let result = cache
            .find_cached_prefix(conv_id, &messages_v2)
            .await
            .unwrap();
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.cached_message_count, 2); // Original 2 messages cached
    }

    #[tokio::test]
    async fn test_min_tokens_requirement() {
        let mut config = ConversationCacheConfig::default();
        config.min_tokens_for_cache = 1000;
        let cache = ConversationCache::new(config);
        let conv_id = "test-conv-3";

        let messages = vec![create_message(MessageRole::User, "Hi")];

        // Should not cache (below minimum)
        cache
            .record_checkpoint(conv_id, &messages, 500)
            .await
            .unwrap();

        let result = cache.find_cached_prefix(conv_id, &messages).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_statistics() {
        let cache = ConversationCache::default();
        let conv_id = "test-conv-4";

        let messages = vec![
            create_message(MessageRole::System, "System prompt"),
            create_message(MessageRole::User, "User message"),
        ];

        // Initial stats
        let stats = cache.statistics().await;
        assert_eq!(stats.total_hits, 0);
        assert_eq!(stats.total_misses, 0);

        // Miss
        cache.find_cached_prefix(conv_id, &messages).await.unwrap();
        let stats = cache.statistics().await;
        assert_eq!(stats.total_misses, 1);

        // Record and hit
        cache
            .record_checkpoint(conv_id, &messages, 2000)
            .await
            .unwrap();
        cache.find_cached_prefix(conv_id, &messages).await.unwrap();
        let stats = cache.statistics().await;
        assert_eq!(stats.total_hits, 1);
    }
}
