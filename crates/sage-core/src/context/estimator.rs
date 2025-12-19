//! Token estimation for LLM messages
//!
//! This module provides token counting functionality for different LLM providers.
//! Since exact tokenization varies by provider, we use approximations based on
//! character counts with provider-specific adjustments.

use crate::llm::LLMMessage;
use crate::tools::types::ToolSchema;
use crate::tools::ToolCall;

/// Token estimator for LLM messages
#[derive(Debug, Clone)]
pub struct TokenEstimator {
    /// Characters per token (average)
    chars_per_token: f32,
    /// Overhead tokens per message (role, formatting)
    message_overhead: usize,
}

impl Default for TokenEstimator {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenEstimator {
    /// Create a new token estimator with default settings
    pub fn new() -> Self {
        Self {
            chars_per_token: 4.0, // Common approximation for English text
            message_overhead: 4,  // Role token + formatting
        }
    }

    /// Create an estimator optimized for a specific provider
    pub fn for_provider(provider: &str) -> Self {
        match provider.to_lowercase().as_str() {
            "openai" => Self {
                chars_per_token: 4.0,
                message_overhead: 4,
            },
            "anthropic" => Self {
                chars_per_token: 3.5, // Claude tends to have slightly smaller tokens
                message_overhead: 3,
            },
            "google" => Self {
                chars_per_token: 4.0,
                message_overhead: 4,
            },
            _ => Self::default(),
        }
    }

    /// Estimate tokens for a single message
    pub fn estimate_message(&self, message: &LLMMessage) -> usize {
        let content_chars = message.content.len();
        let content_tokens = (content_chars as f32 / self.chars_per_token).ceil() as usize;

        // Add tool call tokens if present
        let tool_tokens = if let Some(ref tool_calls) = message.tool_calls {
            tool_calls.iter().map(|tc| self.estimate_tool_call(tc)).sum()
        } else {
            0
        };

        content_tokens + tool_tokens + self.message_overhead
    }

    /// Estimate tokens for a tool call
    fn estimate_tool_call(&self, tool_call: &ToolCall) -> usize {
        let name_tokens = (tool_call.name.len() as f32 / self.chars_per_token).ceil() as usize;
        // Serialize arguments to estimate token count
        let args_str = serde_json::to_string(&tool_call.arguments).unwrap_or_default();
        let args_tokens = (args_str.len() as f32 / self.chars_per_token).ceil() as usize;
        name_tokens + args_tokens + 10 // Overhead for tool call structure
    }

    /// Estimate tokens for a conversation (list of messages)
    pub fn estimate_conversation(&self, messages: &[LLMMessage]) -> usize {
        messages.iter().map(|m| self.estimate_message(m)).sum()
    }

    /// Estimate tokens for tool schemas (sent with each request)
    pub fn estimate_tools(&self, tools: &[ToolSchema]) -> usize {
        tools.iter().map(|t| self.estimate_tool_schema(t)).sum()
    }

    /// Estimate tokens for a single tool schema
    fn estimate_tool_schema(&self, schema: &ToolSchema) -> usize {
        let name_tokens = (schema.name.len() as f32 / self.chars_per_token).ceil() as usize;
        let desc_tokens =
            (schema.description.len() as f32 / self.chars_per_token).ceil() as usize;

        // Estimate parameter schema tokens
        let params_str = serde_json::to_string(&schema.parameters).unwrap_or_default();
        let params_tokens = (params_str.len() as f32 / self.chars_per_token).ceil() as usize;

        name_tokens + desc_tokens + params_tokens + 20 // Schema overhead
    }

    /// Estimate total tokens for a request (messages + tools)
    pub fn estimate_request(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[ToolSchema]>,
    ) -> usize {
        let message_tokens = self.estimate_conversation(messages);
        let tool_tokens = tools.map(|t| self.estimate_tools(t)).unwrap_or(0);
        message_tokens + tool_tokens + 10 // Request overhead
    }

    /// Estimate tokens for a string
    pub fn estimate_string(&self, text: &str) -> usize {
        (text.len() as f32 / self.chars_per_token).ceil() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::MessageRole;
    use std::collections::HashMap;

    fn create_message(role: MessageRole, content: &str) -> LLMMessage {
        LLMMessage {
            role,
            content: content.to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn test_estimate_message_basic() {
        let estimator = TokenEstimator::new();

        // ~40 chars = ~10 tokens + 4 overhead = ~14 tokens
        let msg = create_message(MessageRole::User, "Hello, how are you doing today?");
        let tokens = estimator.estimate_message(&msg);
        assert!(tokens >= 10 && tokens <= 20);
    }

    #[test]
    fn test_estimate_conversation() {
        let estimator = TokenEstimator::new();

        let messages = vec![
            create_message(MessageRole::System, "You are a helpful assistant."),
            create_message(MessageRole::User, "Hello!"),
            create_message(MessageRole::Assistant, "Hi there! How can I help you today?"),
        ];

        let total = estimator.estimate_conversation(&messages);
        assert!(total > 20); // Should be reasonable for these messages
    }

    #[test]
    fn test_provider_specific_estimator() {
        let openai = TokenEstimator::for_provider("openai");
        let anthropic = TokenEstimator::for_provider("anthropic");

        // Anthropic should estimate slightly more tokens for same content
        // (since chars_per_token is lower)
        let text = "This is a test message with some content.";
        let openai_tokens = openai.estimate_string(text);
        let anthropic_tokens = anthropic.estimate_string(text);

        assert!(anthropic_tokens >= openai_tokens);
    }

    #[test]
    fn test_estimate_empty_message() {
        let estimator = TokenEstimator::new();
        let msg = create_message(MessageRole::User, "");
        let tokens = estimator.estimate_message(&msg);
        assert_eq!(tokens, 4); // Just overhead
    }

    #[test]
    fn test_estimate_tool_schema() {
        let estimator = TokenEstimator::new();

        let schema = ToolSchema::new(
            "test_tool",
            "A test tool that does something useful",
            vec![],
        );

        let tokens = estimator.estimate_tool_schema(&schema);
        assert!(tokens > 0);
    }

    #[test]
    fn test_estimate_request() {
        let estimator = TokenEstimator::new();

        let messages = vec![
            create_message(MessageRole::User, "Please help me."),
        ];

        let tools = vec![
            ToolSchema::new("tool1", "First tool", vec![]),
            ToolSchema::new("tool2", "Second tool", vec![]),
        ];

        let total = estimator.estimate_request(&messages, Some(&tools));
        let messages_only = estimator.estimate_request(&messages, None);

        assert!(total > messages_only);
    }

    #[test]
    fn test_estimate_string() {
        let estimator = TokenEstimator::new();

        // 100 chars / 4 chars per token = 25 tokens
        let text = "a".repeat(100);
        let tokens = estimator.estimate_string(&text);
        assert_eq!(tokens, 25);
    }
}
