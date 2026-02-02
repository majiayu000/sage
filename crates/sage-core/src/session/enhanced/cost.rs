//! Cost tracking data types for session expense monitoring

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Cost snapshot for session expense tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostSnapshot {
    /// Session total costs
    #[serde(rename = "sessionTotal")]
    pub session_total: TokenCost,

    /// Costs broken down by model
    #[serde(rename = "byModel")]
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub by_model: HashMap<String, TokenCost>,

    /// Usage statistics by tool
    #[serde(rename = "byTool")]
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub by_tool: HashMap<String, ToolUsageStats>,
}

impl CostSnapshot {
    /// Create a new empty cost snapshot
    pub fn new() -> Self {
        Self {
            session_total: TokenCost::default(),
            by_model: HashMap::new(),
            by_tool: HashMap::new(),
        }
    }

    /// Add token usage for a model
    pub fn add_model_usage(&mut self, model: &str, cost: TokenCost) {
        self.session_total.input_tokens += cost.input_tokens;
        self.session_total.output_tokens += cost.output_tokens;
        self.session_total.cache_read_tokens += cost.cache_read_tokens;
        self.session_total.cache_write_tokens += cost.cache_write_tokens;

        let entry = self.by_model.entry(model.to_string()).or_default();
        entry.input_tokens += cost.input_tokens;
        entry.output_tokens += cost.output_tokens;
        entry.cache_read_tokens += cost.cache_read_tokens;
        entry.cache_write_tokens += cost.cache_write_tokens;
    }

    /// Record tool usage
    pub fn record_tool_usage(&mut self, tool: &str, duration_ms: u64, success: bool) {
        let entry = self.by_tool.entry(tool.to_string()).or_default();
        entry.calls += 1;
        entry.total_duration_ms += duration_ms;
        if success {
            entry.success_count += 1;
        } else {
            entry.error_count += 1;
        }
    }
}

impl Default for CostSnapshot {
    fn default() -> Self {
        Self::new()
    }
}

/// Token cost breakdown
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenCost {
    /// Input tokens consumed
    #[serde(rename = "inputTokens")]
    pub input_tokens: u64,

    /// Output tokens generated
    #[serde(rename = "outputTokens")]
    pub output_tokens: u64,

    /// Tokens read from cache
    #[serde(rename = "cacheReadTokens")]
    #[serde(default)]
    pub cache_read_tokens: u64,

    /// Tokens written to cache
    #[serde(rename = "cacheWriteTokens")]
    #[serde(default)]
    pub cache_write_tokens: u64,

    /// Estimated cost in USD
    #[serde(rename = "estimatedCostUsd")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_cost_usd: Option<f64>,
}

impl TokenCost {
    /// Create a new token cost entry
    pub fn new(input_tokens: u64, output_tokens: u64) -> Self {
        Self {
            input_tokens,
            output_tokens,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            estimated_cost_usd: None,
        }
    }

    /// Set cache tokens
    pub fn with_cache(mut self, read: u64, write: u64) -> Self {
        self.cache_read_tokens = read;
        self.cache_write_tokens = write;
        self
    }

    /// Set estimated cost
    pub fn with_estimated_cost(mut self, cost: f64) -> Self {
        self.estimated_cost_usd = Some(cost);
        self
    }

    /// Get total tokens (input + output)
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}

/// Tool usage statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolUsageStats {
    /// Number of calls
    pub calls: u32,

    /// Total execution time in milliseconds
    #[serde(rename = "totalDurationMs")]
    pub total_duration_ms: u64,

    /// Number of successful calls
    #[serde(rename = "successCount")]
    pub success_count: u32,

    /// Number of failed calls
    #[serde(rename = "errorCount")]
    pub error_count: u32,
}

impl ToolUsageStats {
    /// Get average duration per call
    pub fn avg_duration_ms(&self) -> u64 {
        if self.calls > 0 {
            self.total_duration_ms / self.calls as u64
        } else {
            0
        }
    }

    /// Get success rate (0.0 - 1.0)
    pub fn success_rate(&self) -> f64 {
        if self.calls > 0 {
            self.success_count as f64 / self.calls as f64
        } else {
            0.0
        }
    }
}

/// Compact metadata for compression boundaries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactMetadata {
    /// Trigger type (auto, manual)
    pub trigger: String,

    /// Token count before compression
    #[serde(rename = "preTokens")]
    pub pre_tokens: u64,

    /// Token count after compression
    #[serde(rename = "postTokens")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_tokens: Option<u64>,
}

impl CompactMetadata {
    /// Create new compact metadata
    pub fn new(trigger: impl Into<String>, pre_tokens: u64) -> Self {
        Self {
            trigger: trigger.into(),
            pre_tokens,
            post_tokens: None,
        }
    }

    /// Set post-compression token count
    pub fn with_post_tokens(mut self, tokens: u64) -> Self {
        self.post_tokens = Some(tokens);
        self
    }

    /// Calculate compression ratio
    pub fn compression_ratio(&self) -> Option<f64> {
        self.post_tokens
            .map(|post| post as f64 / self.pre_tokens as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_snapshot() {
        let mut snapshot = CostSnapshot::new();
        snapshot.add_model_usage("claude-3-opus", TokenCost::new(1000, 500));
        snapshot.add_model_usage("claude-3-haiku", TokenCost::new(500, 200));
        snapshot.record_tool_usage("bash", 100, true);
        snapshot.record_tool_usage("bash", 50, false);

        assert_eq!(snapshot.session_total.input_tokens, 1500);
        assert_eq!(snapshot.session_total.output_tokens, 700);
        assert_eq!(snapshot.by_model.len(), 2);
        assert_eq!(snapshot.by_tool.get("bash").unwrap().calls, 2);
    }

    #[test]
    fn test_token_cost() {
        let cost = TokenCost::new(1000, 500)
            .with_cache(100, 50)
            .with_estimated_cost(0.05);

        assert_eq!(cost.total_tokens(), 1500);
        assert_eq!(cost.estimated_cost_usd, Some(0.05));
    }

    #[test]
    fn test_tool_usage_stats() {
        let mut stats = ToolUsageStats::default();
        stats.calls = 10;
        stats.total_duration_ms = 1000;
        stats.success_count = 8;
        stats.error_count = 2;

        assert_eq!(stats.avg_duration_ms(), 100);
        assert!((stats.success_rate() - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_compact_metadata() {
        let meta = CompactMetadata::new("auto", 10000).with_post_tokens(2000);

        assert_eq!(meta.trigger, "auto");
        assert!((meta.compression_ratio().unwrap() - 0.2).abs() < 0.001);
    }

    #[test]
    fn test_serialization() {
        let snapshot = CostSnapshot::new();
        let json = serde_json::to_string(&snapshot).unwrap();
        assert!(json.contains("sessionTotal"));

        let deserialized: CostSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.session_total.input_tokens, 0);
    }
}
