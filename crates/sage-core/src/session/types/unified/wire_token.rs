//! Wire-format token usage for JSONL persistence.

use serde::{Deserialize, Serialize};

/// Token usage statistics (wire format with camelCase serde for JSONL persistence)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WireTokenUsage {
    /// Input tokens used
    #[serde(rename = "inputTokens")]
    pub input_tokens: u64,

    /// Output tokens used
    #[serde(rename = "outputTokens")]
    pub output_tokens: u64,

    /// Cache read tokens
    #[serde(rename = "cacheReadTokens")]
    #[serde(default)]
    pub cache_read_tokens: u64,

    /// Cache write tokens
    #[serde(rename = "cacheWriteTokens")]
    #[serde(default)]
    pub cache_write_tokens: u64,

    /// Cost estimate (USD)
    #[serde(rename = "costEstimate")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_estimate: Option<f64>,
}

impl WireTokenUsage {
    /// Add usage from another WireTokenUsage
    pub fn add(&mut self, other: &WireTokenUsage) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.cache_read_tokens += other.cache_read_tokens;
        self.cache_write_tokens += other.cache_write_tokens;
        if let Some(cost) = other.cost_estimate {
            *self.cost_estimate.get_or_insert(0.0) += cost;
        }
    }

    /// Get total tokens
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}

impl From<&crate::types::TokenUsage> for WireTokenUsage {
    fn from(usage: &crate::types::TokenUsage) -> Self {
        Self {
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            cache_read_tokens: usage.cache_read_tokens.unwrap_or(0),
            cache_write_tokens: usage.cache_write_tokens.unwrap_or(0),
            cost_estimate: usage.cost_estimate,
        }
    }
}

impl From<&WireTokenUsage> for crate::types::TokenUsage {
    fn from(wire: &WireTokenUsage) -> Self {
        Self {
            input_tokens: wire.input_tokens,
            output_tokens: wire.output_tokens,
            cache_read_tokens: if wire.cache_read_tokens > 0 {
                Some(wire.cache_read_tokens)
            } else {
                None
            },
            cache_write_tokens: if wire.cache_write_tokens > 0 {
                Some(wire.cache_write_tokens)
            } else {
                None
            },
            cost_estimate: wire.cost_estimate,
        }
    }
}
