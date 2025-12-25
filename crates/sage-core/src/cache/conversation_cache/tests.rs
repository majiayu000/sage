//! Tests for conversation caching

#[cfg(test)]
mod tests {
    use crate::cache::conversation_cache::{ConversationCache, ConversationCacheConfig};
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

    #[tokio::test]
    async fn test_conversation_cache_basic() {
        let cache = ConversationCache::default();
        let conv_id = "test-conv-1";

        let messages = vec![
            create_message(MessageRole::System, "You are a helpful assistant."),
            create_message(MessageRole::User, "Hello!"),
            create_message(MessageRole::Assistant, "Hi there! How can I help you?"),
        ];

        // Initially, no cache
        let result = cache.find_cached_prefix(conv_id, &messages).await.unwrap();
        assert!(result.is_none());

        // Record checkpoint
        cache
            .record_checkpoint(conv_id, &messages, 2000)
            .await
            .unwrap();

        // Now should find cache
        let result = cache.find_cached_prefix(conv_id, &messages).await.unwrap();
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.cached_message_count, 3);
        assert_eq!(result.cached_token_count, 2000);
    }

    #[tokio::test]
    async fn test_incremental_caching() {
        let cache = ConversationCache::default();
        let conv_id = "test-conv-2";

        let messages_v1 = vec![
            create_message(MessageRole::System, "You are a helpful assistant."),
            create_message(MessageRole::User, "Hello!"),
        ];

        // Record first checkpoint
        cache
            .record_checkpoint(conv_id, &messages_v1, 1500)
            .await
            .unwrap();

        // Add more messages
        let mut messages_v2 = messages_v1.clone();
        messages_v2.push(create_message(
            MessageRole::Assistant,
            "Hi there! How can I help?",
        ));
        messages_v2.push(create_message(MessageRole::User, "What's the weather?"));

        // Should find the original cached prefix
        let result = cache
            .find_cached_prefix(conv_id, &messages_v2)
            .await
            .unwrap();
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.cached_message_count, 2); // Original 2 messages cached
    }

    #[tokio::test]
    async fn test_min_tokens_requirement() {
        let mut config = ConversationCacheConfig::default();
        config.min_tokens_for_cache = 1000;
        let cache = ConversationCache::new(config);
        let conv_id = "test-conv-3";

        let messages = vec![create_message(MessageRole::User, "Hi")];

        // Should not cache (below minimum)
        cache
            .record_checkpoint(conv_id, &messages, 500)
            .await
            .unwrap();

        let result = cache.find_cached_prefix(conv_id, &messages).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_statistics() {
        let cache = ConversationCache::default();
        let conv_id = "test-conv-4";

        let messages = vec![
            create_message(MessageRole::System, "System prompt"),
            create_message(MessageRole::User, "User message"),
        ];

        // Initial stats
        let stats = cache.statistics().await;
        assert_eq!(stats.total_hits, 0);
        assert_eq!(stats.total_misses, 0);

        // Miss
        cache.find_cached_prefix(conv_id, &messages).await.unwrap();
        let stats = cache.statistics().await;
        assert_eq!(stats.total_misses, 1);

        // Record and hit
        cache
            .record_checkpoint(conv_id, &messages, 2000)
            .await
            .unwrap();
        cache.find_cached_prefix(conv_id, &messages).await.unwrap();
        let stats = cache.statistics().await;
        assert_eq!(stats.total_hits, 1);
    }
}
