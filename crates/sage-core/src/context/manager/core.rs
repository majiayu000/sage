//! Core ContextManager struct and constructors

use crate::llm::{LlmClient, LlmMessage};
use crate::tools::types::ToolSchema;
use std::sync::Arc;

use super::super::config::ContextConfig;
use super::super::estimator::TokenEstimator;
use super::super::pruner::{MessagePruner, PruneResult};
use super::super::summarizer::ConversationSummarizer;
use super::types::ContextUsageStats;

/// Context window manager for LLM conversations
///
/// Handles automatic context management including:
/// - Token estimation before sending messages
/// - Automatic pruning when approaching limits
/// - Conversation summarization for context compression
#[derive(Clone)]
pub struct ContextManager {
    /// Configuration for context management
    pub(super) config: ContextConfig,
    /// Token estimator for counting tokens
    pub(super) estimator: TokenEstimator,
    /// Message pruner for reducing context size
    pub(super) pruner: MessagePruner,
    /// Summarizer for compressing conversation history
    pub(super) summarizer: ConversationSummarizer,
}

impl ContextManager {
    /// Create a new context manager with default configuration
    pub fn new(config: ContextConfig) -> Self {
        let estimator = TokenEstimator::new();
        let pruner = MessagePruner::new(config.clone());
        let summarizer = ConversationSummarizer::new();

        Self {
            config,
            estimator,
            pruner,
            summarizer,
        }
    }

    /// Create a context manager with an LLM client for summarization
    pub fn with_llm_client(config: ContextConfig, llm_client: Arc<LlmClient>) -> Self {
        let estimator = TokenEstimator::new();
        let pruner = MessagePruner::new(config.clone());
        let summarizer = ConversationSummarizer::with_client(llm_client);

        Self {
            config,
            estimator,
            pruner,
            summarizer,
        }
    }

    /// Create a context manager optimized for a specific provider
    pub fn for_provider(provider: &str, model: &str) -> Self {
        let config = ContextConfig::for_provider(provider, model);
        let estimator = TokenEstimator::for_provider(provider);
        let pruner = MessagePruner::new(config.clone());
        let summarizer = ConversationSummarizer::new();

        Self {
            config,
            estimator,
            pruner,
            summarizer,
        }
    }

    /// Create with custom components
    pub fn with_components(
        config: ContextConfig,
        estimator: TokenEstimator,
        summarizer: ConversationSummarizer,
    ) -> Self {
        let pruner = MessagePruner::new(config.clone());

        Self {
            config,
            estimator,
            pruner,
            summarizer,
        }
    }

    /// Get the current configuration
    pub fn config(&self) -> &ContextConfig {
        &self.config
    }

    /// Get the token estimator
    pub fn estimator(&self) -> &TokenEstimator {
        &self.estimator
    }

    /// Estimate tokens for a conversation
    pub fn estimate_tokens(&self, messages: &[LlmMessage]) -> usize {
        self.estimator.estimate_conversation(messages)
    }

    /// Estimate total tokens including tools
    pub fn estimate_request_tokens(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> usize {
        self.estimator.estimate_request(messages, tools)
    }

    /// Check if context is approaching the limit
    pub fn is_approaching_limit(&self, messages: &[LlmMessage]) -> bool {
        let current_tokens = self.estimator.estimate_conversation(messages);
        current_tokens >= self.config.threshold_tokens()
    }

    /// Check if context exceeds the maximum
    pub fn exceeds_limit(&self, messages: &[LlmMessage]) -> bool {
        let current_tokens = self.estimator.estimate_conversation(messages);
        current_tokens >= self.config.max_context_tokens
    }

    /// Get context usage statistics
    pub fn get_usage_stats(&self, messages: &[LlmMessage]) -> ContextUsageStats {
        let current_tokens = self.estimator.estimate_conversation(messages);
        let max_tokens = self.config.max_context_tokens;
        let threshold_tokens = self.config.threshold_tokens();

        ContextUsageStats {
            current_tokens,
            max_tokens,
            threshold_tokens,
            usage_percentage: (current_tokens as f32 / max_tokens as f32) * 100.0,
            messages_count: messages.len(),
            is_approaching_limit: current_tokens >= threshold_tokens,
            is_over_limit: current_tokens >= max_tokens,
        }
    }

    /// Prune messages without summarization
    pub fn prune(&self, messages: Vec<LlmMessage>, target_tokens: usize) -> PruneResult {
        self.pruner.prune(messages, target_tokens)
    }
}
