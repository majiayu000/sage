//! Tests for context manager

#[cfg(test)]
mod tests {
    use crate::context::config::{ContextConfig, OverflowStrategy};
    use crate::context::manager::{ContextManager, ContextUsageStats, PrepareResult};
    use crate::llm::{LlmMessage, MessageRole};
    use std::collections::HashMap;

    fn create_message(role: MessageRole, content: &str) -> LlmMessage {
        LlmMessage {
            role,
            content: content.to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
            metadata: HashMap::new(),
        }
    }

    fn create_test_messages(count: usize) -> Vec<LlmMessage> {
        let mut messages = vec![create_message(
            MessageRole::System,
            "You are a helpful assistant.",
        )];

        for i in 0..count {
            if i % 2 == 0 {
                messages.push(create_message(
                    MessageRole::User,
                    &format!("User message number {} with some content to fill tokens", i),
                ));
            } else {
                messages.push(create_message(
                    MessageRole::Assistant,
                    &format!(
                        "Assistant response number {} with additional content for testing",
                        i
                    ),
                ));
            }
        }

        messages
    }

    #[test]
    fn test_create_manager() {
        let config = ContextConfig::default();
        let manager = ContextManager::new(config);

        assert_eq!(manager.config().max_context_tokens, 128_000);
    }

    #[test]
    fn test_for_provider() {
        let manager = ContextManager::for_provider("anthropic", "claude-3.5-sonnet");
        assert_eq!(manager.config().max_context_tokens, 200_000);

        let manager = ContextManager::for_provider("openai", "gpt-4-turbo");
        assert_eq!(manager.config().max_context_tokens, 128_000);
    }

    #[test]
    fn test_estimate_tokens() {
        let manager = ContextManager::new(ContextConfig::default());
        let messages = create_test_messages(5);

        let tokens = manager.estimate_tokens(&messages);
        assert!(tokens > 0);
    }

    #[test]
    fn test_is_approaching_limit() {
        // Create a config with very low limit for testing
        // reserved_for_response: 50 tokens, threshold = 100 - 50 = 50 tokens
        let config = ContextConfig::new()
            .with_max_tokens(100)
            .with_reserved_tokens(50);

        let manager = ContextManager::new(config);

        // Small message should be under threshold
        let small = vec![create_message(MessageRole::User, "Hi")];
        assert!(!manager.is_approaching_limit(&small));

        // Large message should be over threshold
        let large = vec![create_message(
            MessageRole::User,
            &"x".repeat(500), // ~125 tokens
        )];
        assert!(manager.is_approaching_limit(&large));
    }

    #[test]
    fn test_get_usage_stats() {
        let config = ContextConfig::new().with_max_tokens(1000);
        let manager = ContextManager::new(config);

        let messages = create_test_messages(5);
        let stats = manager.get_usage_stats(&messages);

        assert!(stats.current_tokens > 0);
        assert_eq!(stats.max_tokens, 1000);
        assert_eq!(stats.messages_count, 6); // 5 + 1 system
        assert!(stats.usage_percentage >= 0.0);
    }

    #[tokio::test]
    async fn test_prepare_messages_under_threshold() {
        let config = ContextConfig::new().with_max_tokens(100_000);
        let manager = ContextManager::new(config);

        let messages = create_test_messages(5);
        let result = manager
            .prepare_messages(messages.clone(), None)
            .await
            .unwrap();

        // Should return unchanged when under threshold
        assert!(!result.was_pruned);
        assert!(!result.was_summarized);
        assert_eq!(result.messages.len(), messages.len());
    }

    #[tokio::test]
    async fn test_prepare_messages_over_threshold_truncate() {
        // reserved_for_response: 50 tokens, threshold = 100 - 50 = 50 tokens
        let config = ContextConfig::new()
            .with_max_tokens(100)
            .with_reserved_tokens(50)
            .with_strategy(OverflowStrategy::Truncate)
            .with_min_messages(2);

        let manager = ContextManager::new(config);
        let messages = create_test_messages(10);

        let result = manager.prepare_messages(messages, None).await.unwrap();

        // Should be pruned
        assert!(result.was_pruned);
    }

    #[tokio::test]
    async fn test_prepare_messages_sliding_window() {
        // reserved_for_response: 50 tokens, threshold = 100 - 50 = 50 tokens
        let mut config = ContextConfig::new()
            .with_max_tokens(100)
            .with_reserved_tokens(50)
            .with_strategy(OverflowStrategy::SlidingWindow);
        config.sliding_window_first = 2;
        config.sliding_window_last = 2;

        let manager = ContextManager::new(config);
        let messages = create_test_messages(10);

        let result = manager.prepare_messages(messages, None).await.unwrap();

        assert!(result.was_pruned);
    }

    #[test]
    fn test_prune_direct() {
        let config = ContextConfig::new().with_min_messages(3);
        let manager = ContextManager::new(config);

        let messages = create_test_messages(10);
        let result = manager.prune(messages, 50);

        assert!(result.kept.len() >= 4); // At least 3 + system
    }

    #[test]
    fn test_prepare_result_tokens_saved() {
        let result = PrepareResult {
            messages: vec![],
            was_pruned: true,
            was_summarized: false,
            original_tokens: 1000,
            final_tokens: 600,
            removed_count: 5,
        };

        assert_eq!(result.tokens_saved(), 400);
        assert!((result.compression_ratio() - 0.6).abs() < 0.01);
    }

    #[test]
    fn test_usage_stats_calculations() {
        let stats = ContextUsageStats {
            current_tokens: 5000,
            max_tokens: 10000,
            threshold_tokens: 7500,
            usage_percentage: 50.0,
            messages_count: 20,
            is_approaching_limit: false,
            is_over_limit: false,
        };

        assert_eq!(stats.tokens_until_threshold(), 2500);
        assert_eq!(stats.tokens_until_limit(), 5000);
    }
}
