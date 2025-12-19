//! Message pruning strategies for context management
//!
//! This module provides different strategies for pruning conversation history
//! while preserving important context.

use crate::llm::{LLMMessage, MessageRole};

use super::config::{ContextConfig, OverflowStrategy};
use super::estimator::TokenEstimator;

/// Message pruner for managing conversation history
#[derive(Debug, Clone)]
pub struct MessagePruner {
    config: ContextConfig,
    estimator: TokenEstimator,
}

impl MessagePruner {
    /// Create a new message pruner
    pub fn new(config: ContextConfig) -> Self {
        Self {
            config,
            estimator: TokenEstimator::new(),
        }
    }

    /// Create with custom estimator
    pub fn with_estimator(config: ContextConfig, estimator: TokenEstimator) -> Self {
        Self { config, estimator }
    }

    /// Prune messages to fit within target token count
    ///
    /// Returns the pruned messages and optionally the messages that were removed
    /// (for potential summarization)
    pub fn prune(&self, messages: Vec<LLMMessage>, target_tokens: usize) -> PruneResult {
        match self.config.overflow_strategy {
            OverflowStrategy::Truncate => self.prune_truncate(messages, target_tokens),
            OverflowStrategy::SlidingWindow => self.prune_sliding_window(messages, target_tokens),
            OverflowStrategy::Summarize | OverflowStrategy::Hybrid => {
                self.prune_for_summarization(messages, target_tokens)
            }
        }
    }

    /// Simple truncation - keep most recent messages
    fn prune_truncate(&self, messages: Vec<LLMMessage>, target_tokens: usize) -> PruneResult {
        let mut result = Vec::new();
        let mut removed = Vec::new();
        let mut current_tokens = 0;

        // Always keep system messages first
        let (system_msgs, other_msgs): (Vec<_>, Vec<_>) =
            messages.into_iter().partition(|m| m.role == MessageRole::System);

        for msg in &system_msgs {
            current_tokens += self.estimator.estimate_message(msg);
            result.push(msg.clone());
        }

        // Keep recent messages from the end
        let min_keep = self.config.min_messages_to_keep;
        let to_consider: Vec<_> = other_msgs.into_iter().rev().collect();

        for (i, msg) in to_consider.into_iter().enumerate() {
            let msg_tokens = self.estimator.estimate_message(&msg);

            // Always keep minimum messages
            if i < min_keep || current_tokens + msg_tokens <= target_tokens {
                current_tokens += msg_tokens;
                result.push(msg);
            } else {
                removed.push(msg);
            }
        }

        // Reverse to restore chronological order (we added in reverse)
        let _system_count = result.iter().filter(|m| m.role == MessageRole::System).count();
        let (system_part, rest): (Vec<_>, Vec<_>) =
            result.into_iter().partition(|m| m.role == MessageRole::System);

        let mut final_result = system_part;
        let mut rest: Vec<_> = rest.into_iter().rev().collect();
        final_result.append(&mut rest);

        removed.reverse();

        PruneResult {
            kept: final_result,
            removed,
            kept_tokens: current_tokens,
        }
    }

    /// Sliding window - keep first N and last M messages
    fn prune_sliding_window(
        &self,
        messages: Vec<LLMMessage>,
        _target_tokens: usize,
    ) -> PruneResult {
        let (system_msgs, other_msgs): (Vec<_>, Vec<_>) =
            messages.into_iter().partition(|m| m.role == MessageRole::System);

        let first_n = self.config.sliding_window_first;
        let last_m = self.config.sliding_window_last;

        let mut kept = Vec::new();
        let mut removed = Vec::new();
        let mut current_tokens = 0;

        // Add system messages
        for msg in &system_msgs {
            current_tokens += self.estimator.estimate_message(msg);
            kept.push(msg.clone());
        }

        let other_len = other_msgs.len();
        if other_len <= first_n + last_m {
            // No pruning needed
            for msg in other_msgs {
                current_tokens += self.estimator.estimate_message(&msg);
                kept.push(msg);
            }
        } else {
            // Keep first N
            for msg in other_msgs.iter().take(first_n) {
                current_tokens += self.estimator.estimate_message(msg);
                kept.push(msg.clone());
            }

            // Remove middle messages
            for msg in other_msgs.iter().skip(first_n).take(other_len - first_n - last_m) {
                removed.push(msg.clone());
            }

            // Keep last M
            for msg in other_msgs.iter().skip(other_len - last_m) {
                current_tokens += self.estimator.estimate_message(msg);
                kept.push(msg.clone());
            }
        }

        PruneResult {
            kept,
            removed,
            kept_tokens: current_tokens,
        }
    }

    /// Prune for summarization - separate old messages for summarization
    fn prune_for_summarization(
        &self,
        messages: Vec<LLMMessage>,
        _target_tokens: usize,
    ) -> PruneResult {
        let (system_msgs, other_msgs): (Vec<_>, Vec<_>) =
            messages.into_iter().partition(|m| m.role == MessageRole::System);

        let min_keep = self.config.min_messages_to_keep;

        let mut kept = system_msgs;
        let mut removed = Vec::new();
        let mut current_tokens = 0;

        for msg in &kept {
            current_tokens += self.estimator.estimate_message(msg);
        }

        let other_len = other_msgs.len();
        if other_len <= min_keep {
            // Keep all
            for msg in other_msgs {
                current_tokens += self.estimator.estimate_message(&msg);
                kept.push(msg);
            }
        } else {
            // Remove older messages for summarization
            let to_remove = other_len - min_keep;

            for msg in other_msgs.iter().take(to_remove) {
                // Optionally preserve tool results
                if self.config.preserve_tool_results && self.is_tool_message(msg) {
                    current_tokens += self.estimator.estimate_message(msg);
                    kept.push(msg.clone());
                } else {
                    removed.push(msg.clone());
                }
            }

            // Keep recent messages
            for msg in other_msgs.into_iter().skip(to_remove) {
                current_tokens += self.estimator.estimate_message(&msg);
                kept.push(msg);
            }
        }

        PruneResult {
            kept,
            removed,
            kept_tokens: current_tokens,
        }
    }

    /// Check if a message is related to tool calls
    fn is_tool_message(&self, message: &LLMMessage) -> bool {
        message.tool_calls.is_some() || message.tool_call_id.is_some()
    }

    /// Check if a message should never be pruned
    pub fn is_important(&self, message: &LLMMessage) -> bool {
        // System messages are always important
        if message.role == MessageRole::System {
            return true;
        }

        // Tool messages may be important
        if self.config.preserve_tool_results && self.is_tool_message(message) {
            return true;
        }

        false
    }
}

/// Result of pruning operation
#[derive(Debug, Clone)]
pub struct PruneResult {
    /// Messages that were kept
    pub kept: Vec<LLMMessage>,
    /// Messages that were removed (available for summarization)
    pub removed: Vec<LLMMessage>,
    /// Estimated tokens in kept messages
    pub kept_tokens: usize,
}

impl PruneResult {
    /// Check if any messages were removed
    pub fn has_removed(&self) -> bool {
        !self.removed.is_empty()
    }

    /// Get count of removed messages
    pub fn removed_count(&self) -> usize {
        self.removed.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_messages(count: usize) -> Vec<LLMMessage> {
        let mut messages = vec![LLMMessage {
            role: MessageRole::System,
            content: "You are a helpful assistant.".to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
            metadata: HashMap::new(),
        }];

        for i in 0..count {
            messages.push(LLMMessage {
                role: if i % 2 == 0 {
                    MessageRole::User
                } else {
                    MessageRole::Assistant
                },
                content: format!("Message number {}", i),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            cache_control: None,
                metadata: HashMap::new(),
            });
        }

        messages
    }

    #[test]
    fn test_prune_truncate_keeps_system() {
        let config = ContextConfig::new()
            .with_strategy(OverflowStrategy::Truncate)
            .with_min_messages(3);

        let pruner = MessagePruner::new(config);
        let messages = create_messages(10);

        let result = pruner.prune(messages, 100);

        // Should keep system message
        assert!(result.kept.iter().any(|m| m.role == MessageRole::System));
    }

    #[test]
    fn test_prune_truncate_keeps_minimum() {
        let config = ContextConfig::new()
            .with_strategy(OverflowStrategy::Truncate)
            .with_min_messages(5);

        let pruner = MessagePruner::new(config);
        let messages = create_messages(20);

        let result = pruner.prune(messages, 50); // Very low target

        // Should keep at least min_messages + system
        assert!(result.kept.len() >= 6); // 5 + 1 system
    }

    #[test]
    fn test_prune_sliding_window() {
        let mut config = ContextConfig::new().with_strategy(OverflowStrategy::SlidingWindow);
        config.sliding_window_first = 2;
        config.sliding_window_last = 3;

        let pruner = MessagePruner::new(config);
        let messages = create_messages(10);

        let result = pruner.prune(messages, 1000);

        // Should keep 1 system + 2 first + 3 last = 6
        assert_eq!(result.kept.len(), 6);
        assert!(result.removed.len() > 0);
    }

    #[test]
    fn test_prune_for_summarization() {
        let config = ContextConfig::new()
            .with_strategy(OverflowStrategy::Summarize)
            .with_min_messages(3);

        let pruner = MessagePruner::new(config);
        let messages = create_messages(10);

        let result = pruner.prune(messages, 1000);

        // Should separate old messages for summarization
        assert!(result.has_removed());
        assert!(result.kept.len() >= 4); // At least 3 + 1 system
    }

    #[test]
    fn test_no_prune_when_under_limit() {
        let config = ContextConfig::new()
            .with_strategy(OverflowStrategy::SlidingWindow);

        let pruner = MessagePruner::new(config);
        let messages = create_messages(5); // Small conversation

        let result = pruner.prune(messages.clone(), 10000);

        // Should keep all when under limit
        assert_eq!(result.kept.len(), messages.len());
        assert!(result.removed.is_empty());
    }

    #[test]
    fn test_is_important() {
        let config = ContextConfig::new().with_preserve_tools(true);
        let pruner = MessagePruner::new(config);

        let system = LLMMessage {
            role: MessageRole::System,
            content: "System".to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
            metadata: HashMap::new(),
        };

        let user = LLMMessage {
            role: MessageRole::User,
            content: "User".to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
            metadata: HashMap::new(),
        };

        let tool_result = LLMMessage {
            role: MessageRole::Tool,
            content: "Tool result".to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: Some("call_123".to_string()),
            cache_control: None,
            metadata: HashMap::new(),
        };

        assert!(pruner.is_important(&system));
        assert!(!pruner.is_important(&user));
        assert!(pruner.is_important(&tool_result));
    }
}
