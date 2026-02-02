//! LLM request/response snapshot types for debugging and replay

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// LLM request snapshot for debugging and replay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequestSnapshot {
    /// Provider name (anthropic, openai, google, etc.)
    pub provider: String,

    /// Model name
    pub model: String,

    /// Number of messages in the request
    #[serde(rename = "messageCount")]
    pub message_count: usize,

    /// Number of tools available
    #[serde(rename = "toolCount")]
    pub tool_count: usize,

    /// System prompt length in characters
    #[serde(rename = "systemPromptLength")]
    pub system_prompt_length: usize,

    /// Temperature setting
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Maximum tokens to generate
    #[serde(rename = "maxTokens")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// Request timestamp
    pub timestamp: DateTime<Utc>,

    /// Provider-assigned request ID
    #[serde(rename = "requestId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl LlmRequestSnapshot {
    /// Create a new LLM request snapshot
    pub fn new(provider: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            model: model.into(),
            message_count: 0,
            tool_count: 0,
            system_prompt_length: 0,
            temperature: None,
            max_tokens: None,
            timestamp: Utc::now(),
            request_id: None,
        }
    }

    /// Set message count
    pub fn with_message_count(mut self, count: usize) -> Self {
        self.message_count = count;
        self
    }

    /// Set tool count
    pub fn with_tool_count(mut self, count: usize) -> Self {
        self.tool_count = count;
        self
    }

    /// Set system prompt length
    pub fn with_system_prompt_length(mut self, length: usize) -> Self {
        self.system_prompt_length = length;
        self
    }

    /// Set temperature
    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    /// Set max tokens
    pub fn with_max_tokens(mut self, tokens: u32) -> Self {
        self.max_tokens = Some(tokens);
        self
    }

    /// Set request ID
    pub fn with_request_id(mut self, id: impl Into<String>) -> Self {
        self.request_id = Some(id.into());
        self
    }
}

/// LLM response snapshot for performance tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponseSnapshot {
    /// Provider-assigned response ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Model that generated the response
    pub model: String,

    /// Stop reason (end_turn, tool_use, max_tokens, etc.)
    #[serde(rename = "stopReason")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,

    /// Total latency in milliseconds
    #[serde(rename = "latencyMs")]
    pub latency_ms: u64,

    /// Time to first token in milliseconds
    #[serde(rename = "firstTokenLatencyMs")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_token_latency_ms: Option<u64>,

    /// Response headers (rate limit info, etc.)
    #[serde(rename = "responseHeaders")]
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub response_headers: HashMap<String, String>,
}

impl LlmResponseSnapshot {
    /// Create a new LLM response snapshot
    pub fn new(model: impl Into<String>, latency_ms: u64) -> Self {
        Self {
            id: None,
            model: model.into(),
            stop_reason: None,
            latency_ms,
            first_token_latency_ms: None,
            response_headers: HashMap::new(),
        }
    }

    /// Set response ID
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set stop reason
    pub fn with_stop_reason(mut self, reason: impl Into<String>) -> Self {
        self.stop_reason = Some(reason.into());
        self
    }

    /// Set first token latency
    pub fn with_first_token_latency(mut self, latency_ms: u64) -> Self {
        self.first_token_latency_ms = Some(latency_ms);
        self
    }

    /// Add response header
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.response_headers.insert(key.into(), value.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_request_snapshot() {
        let snapshot = LlmRequestSnapshot::new("anthropic", "claude-3-opus")
            .with_message_count(5)
            .with_tool_count(10)
            .with_system_prompt_length(1000)
            .with_temperature(0.7)
            .with_max_tokens(4096);

        assert_eq!(snapshot.provider, "anthropic");
        assert_eq!(snapshot.model, "claude-3-opus");
        assert_eq!(snapshot.message_count, 5);
        assert_eq!(snapshot.tool_count, 10);
        assert_eq!(snapshot.temperature, Some(0.7));
    }

    #[test]
    fn test_llm_response_snapshot() {
        let snapshot = LlmResponseSnapshot::new("claude-3-opus", 1500)
            .with_id("resp_123")
            .with_stop_reason("end_turn")
            .with_first_token_latency(200)
            .with_header("x-ratelimit-remaining", "100");

        assert_eq!(snapshot.model, "claude-3-opus");
        assert_eq!(snapshot.latency_ms, 1500);
        assert_eq!(snapshot.first_token_latency_ms, Some(200));
        assert_eq!(
            snapshot.response_headers.get("x-ratelimit-remaining"),
            Some(&"100".to_string())
        );
    }

    #[test]
    fn test_serialization() {
        let request = LlmRequestSnapshot::new("openai", "gpt-4");
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("messageCount"));

        let response = LlmResponseSnapshot::new("gpt-4", 500);
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("latencyMs"));
    }
}
