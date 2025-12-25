//! Cache configuration

use std::collections::HashMap;
use std::time::Duration;

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
                "Bash".to_string(),  // Commands have side effects
                "Write".to_string(), // Writes have side effects
                "Edit".to_string(),  // Edits have side effects
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
        self.tool_ttls
            .get(tool)
            .cloned()
            .unwrap_or(self.default_ttl)
    }

    /// Check if a tool should be cached
    pub fn should_cache(&self, tool: &str) -> bool {
        !self
            .excluded_tools
            .iter()
            .any(|t| t.eq_ignore_ascii_case(tool))
    }
}
