//! MCP resource cache implementation

use super::types::{CacheConfig, CacheEntry, CacheSizeBreakdown, CacheStats};
use crate::mcp::types::{McpPrompt, McpResource, McpResourceContent, McpTool};
use dashmap::DashMap;
use std::sync::atomic::Ordering;
use tracing::debug;

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

    // Internal access for eviction module
    pub(super) fn tools_map(&self) -> &DashMap<String, CacheEntry<Vec<McpTool>>> {
        &self.tools
    }

    pub(super) fn resources_map(&self) -> &DashMap<String, CacheEntry<Vec<McpResource>>> {
        &self.resources
    }

    pub(super) fn prompts_map(&self) -> &DashMap<String, CacheEntry<Vec<McpPrompt>>> {
        &self.prompts
    }

    pub(super) fn resource_content_map(&self) -> &DashMap<String, CacheEntry<McpResourceContent>> {
        &self.resource_content
    }

    pub(super) fn stats_mut(&self) -> &CacheStats {
        &self.stats
    }
}

impl Default for McpCache {
    fn default() -> Self {
        Self::new()
    }
}
