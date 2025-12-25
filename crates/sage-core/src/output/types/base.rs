//! Base output types and format definitions

use serde::{Deserialize, Serialize};

/// Output format mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// Human-readable text (default)
    #[default]
    Text,
    /// Structured JSON output
    Json,
    /// JSONL streaming output (one JSON object per line)
    StreamJson,
}

impl OutputFormat {
    /// Parse from string
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "text" => Some(Self::Text),
            "json" => Some(Self::Json),
            "stream-json" | "streamjson" | "jsonl" => Some(Self::StreamJson),
            _ => None,
        }
    }

    /// Check if this format is JSON-based
    pub fn is_json(&self) -> bool {
        matches!(self, Self::Json | Self::StreamJson)
    }

    /// Check if this format is streaming
    pub fn is_streaming(&self) -> bool {
        matches!(self, Self::StreamJson)
    }
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text => write!(f, "text"),
            Self::Json => write!(f, "json"),
            Self::StreamJson => write!(f, "stream-json"),
        }
    }
}

/// Cost information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostInfo {
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub total_tokens: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_cost_usd: Option<f64>,
    /// Number of tokens written to cache (Anthropic prompt caching)
    /// Cache writes cost 25% more than base input tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<usize>,
    /// Number of tokens read from cache (Anthropic prompt caching)
    /// Cache reads cost only 10% of base input tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<usize>,
}

impl CostInfo {
    /// Create new cost info
    pub fn new(input_tokens: usize, output_tokens: usize) -> Self {
        Self {
            input_tokens,
            output_tokens,
            total_tokens: input_tokens + output_tokens,
            estimated_cost_usd: None,
            cache_creation_input_tokens: None,
            cache_read_input_tokens: None,
        }
    }

    /// Set estimated cost
    pub fn with_cost(mut self, cost: f64) -> Self {
        self.estimated_cost_usd = Some(cost);
        self
    }

    /// Set cache creation input tokens
    pub fn with_cache_creation(mut self, tokens: usize) -> Self {
        self.cache_creation_input_tokens = Some(tokens);
        self
    }

    /// Set cache read input tokens
    pub fn with_cache_read(mut self, tokens: usize) -> Self {
        self.cache_read_input_tokens = Some(tokens);
        self
    }

    /// Check if cache metrics are available
    pub fn has_cache_metrics(&self) -> bool {
        self.cache_creation_input_tokens.is_some() || self.cache_read_input_tokens.is_some()
    }

    /// Get cache summary string (e.g., "cache: 1000 created, 5000 read")
    pub fn cache_summary(&self) -> Option<String> {
        if !self.has_cache_metrics() {
            return None;
        }

        let mut parts = Vec::new();
        if let Some(created) = self.cache_creation_input_tokens {
            if created > 0 {
                parts.push(format!("{} created", created));
            }
        }
        if let Some(read) = self.cache_read_input_tokens {
            if read > 0 {
                parts.push(format!("{} read", read));
            }
        }

        if parts.is_empty() {
            None
        } else {
            Some(format!("cache: {}", parts.join(", ")))
        }
    }
}

/// Summary of a tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallSummary {
    pub call_id: String,
    pub tool_name: String,
    pub success: bool,
    pub duration_ms: u64,
}

impl ToolCallSummary {
    /// Create a new summary
    pub fn new(call_id: impl Into<String>, tool_name: impl Into<String>, success: bool) -> Self {
        Self {
            call_id: call_id.into(),
            tool_name: tool_name.into(),
            success,
            duration_ms: 0,
        }
    }

    /// Set duration
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }
}
