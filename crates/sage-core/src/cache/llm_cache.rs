//! LLM response caching implementation

use super::{CacheKey, CacheManager, types::hash_utils};
use crate::error::SageResult;
use crate::llm::client::LlmClient;
use crate::llm::messages::{LlmMessage, LlmResponse};
use crate::tools::ToolSchema;
use std::time::Duration;

/// LLM response cache
pub struct LlmCache {
    /// Cache manager
    cache_manager: CacheManager,
    /// Default TTL for LLM responses
    default_ttl: Option<Duration>,
}

/// Deprecated: Use `LlmCache` instead
#[deprecated(since = "0.2.0", note = "Use `LlmCache` instead")]
pub type LlmCache = LlmCache;

impl LlmCache {
    /// Create a new LLM cache
    pub fn new(cache_manager: CacheManager, default_ttl: Option<Duration>) -> Self {
        Self {
            cache_manager,
            default_ttl,
        }
    }

    /// Get cached LLM response
    pub async fn get_response(
        &self,
        provider: &str,
        model: &str,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<Option<LlmResponse>> {
        let key = self.create_cache_key(provider, model, messages, tools);
        self.cache_manager.get(&key).await
    }

    /// Cache LLM response
    pub async fn cache_response(
        &self,
        provider: &str,
        model: &str,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
        response: &LlmResponse,
        ttl: Option<Duration>,
    ) -> SageResult<()> {
        let key = self.create_cache_key(provider, model, messages, tools);
        let ttl = ttl.or(self.default_ttl);
        self.cache_manager.set(key, response.clone(), ttl).await
    }

    /// Check if response is cached
    pub async fn is_cached(
        &self,
        provider: &str,
        model: &str,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<bool> {
        let key = self.create_cache_key(provider, model, messages, tools);
        Ok(self.cache_manager.get::<LlmResponse>(&key).await?.is_some())
    }

    /// Invalidate cached response
    pub async fn invalidate_response(
        &self,
        provider: &str,
        model: &str,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<()> {
        let key = self.create_cache_key(provider, model, messages, tools);
        self.cache_manager.remove(&key).await
    }

    /// Clear all cached responses
    pub async fn clear_all(&self) -> SageResult<()> {
        self.cache_manager.clear().await
    }

    /// Get cache statistics
    pub async fn statistics(&self) -> SageResult<super::types::CacheStatistics> {
        self.cache_manager.statistics().await
    }

    /// Cleanup expired entries
    pub async fn cleanup_expired(&self) -> SageResult<()> {
        self.cache_manager.cleanup_expired().await
    }

    /// Create cache key for LLM request
    fn create_cache_key(
        &self,
        provider: &str,
        model: &str,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> CacheKey {
        let messages_hash = hash_utils::hash_messages(messages);
        let tools_hash = tools.map(|t| hash_utils::hash_tools(t));

        CacheKey::llm_response(provider, model, messages_hash, tools_hash)
    }
}

/// LLM cache builder for easy configuration
pub struct LlmCacheBuilder {
    cache_manager: Option<CacheManager>,
    default_ttl: Option<Duration>,
}

/// Deprecated: Use `LlmCacheBuilder` instead
#[deprecated(since = "0.2.0", note = "Use `LlmCacheBuilder` instead")]
pub type LlmCacheBuilder = LlmCacheBuilder;

impl LlmCacheBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            cache_manager: None,
            default_ttl: None,
        }
    }

    /// Set cache manager
    pub fn with_cache_manager(mut self, cache_manager: CacheManager) -> Self {
        self.cache_manager = Some(cache_manager);
        self
    }

    /// Set default TTL
    pub fn with_default_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = Some(ttl);
        self
    }

    /// Build the LLM cache
    pub fn build(self) -> SageResult<LlmCache> {
        let cache_manager = self.cache_manager.unwrap_or_default();
        Ok(LlmCache::new(cache_manager, self.default_ttl))
    }
}

impl Default for LlmCacheBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache-aware LLM client wrapper
pub struct CachedLlmClient<T> {
    /// Inner LLM client
    inner: T,
    /// LLM cache
    cache: LlmCache,
    /// Whether to use cache for reads
    enable_read_cache: bool,
    /// Whether to use cache for writes
    enable_write_cache: bool,
}

/// Deprecated: Use `CachedLlmClient` instead
#[deprecated(since = "0.2.0", note = "Use `CachedLlmClient` instead")]
pub type CachedLlmClient<T> = CachedLlmClient<T>;

impl<T> CachedLlmClient<T> {
    /// Create a new cached LLM client
    pub fn new(inner: T, cache: LlmCache) -> Self {
        Self {
            inner,
            cache,
            enable_read_cache: true,
            enable_write_cache: true,
        }
    }

    /// Enable or disable read cache
    pub fn with_read_cache(mut self, enabled: bool) -> Self {
        self.enable_read_cache = enabled;
        self
    }

    /// Enable or disable write cache
    pub fn with_write_cache(mut self, enabled: bool) -> Self {
        self.enable_write_cache = enabled;
        self
    }

    /// Get the inner client
    pub fn inner(&self) -> &T {
        &self.inner
    }

    /// Get the cache
    pub fn cache(&self) -> &LlmCache {
        &self.cache
    }
}

impl CachedLlmClient<LlmClient> {
    /// Chat with caching support
    pub async fn chat_with_cache(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LlmResponse> {
        let provider = self.inner.provider().to_string();
        let model = self.inner.model().to_string();

        // Try to get from cache first
        if self.enable_read_cache {
            if let Some(cached_response) = self
                .cache
                .get_response(&provider, &model, messages, tools)
                .await?
            {
                return Ok(cached_response);
            }
        }

        // Call the actual LLM
        let response = self.inner.chat(messages, tools).await?;

        // Cache the response
        if self.enable_write_cache {
            self.cache
                .cache_response(&provider, &model, messages, tools, &response, None)
                .await?;
        }

        Ok(response)
    }
}

/// Cache warming utilities
pub mod warming {
    use super::*;

    /// Warm cache with common requests
    pub async fn warm_cache_with_common_requests(
        cache: &LlmCache,
        requests: &[(String, String, Vec<LlmMessage>, Option<Vec<ToolSchema>>)],
    ) -> SageResult<()> {
        for (provider, model, messages, tools) in requests {
            // Check if already cached
            if !cache
                .is_cached(provider, model, messages, tools.as_deref())
                .await?
            {
                // This would typically involve making actual LLM calls
                // For now, we just mark the cache as ready for these requests
                tracing::info!(
                    "Cache warming: {} {} with {} messages",
                    provider,
                    model,
                    messages.len()
                );
            }
        }
        Ok(())
    }

    /// Preload frequently used responses
    pub async fn preload_responses(
        cache: &LlmCache,
        responses: &[(
            String,
            String,
            Vec<LlmMessage>,
            Option<Vec<ToolSchema>>,
            LlmResponse,
        )],
    ) -> SageResult<()> {
        for (provider, model, messages, tools, response) in responses {
            cache
                .cache_response(provider, model, messages, tools.as_deref(), response, None)
                .await?;
        }
        Ok(())
    }
}
