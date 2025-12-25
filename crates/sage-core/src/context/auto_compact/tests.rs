//! Tests for auto-compact functionality

#[cfg(test)]
mod tests {
    use super::super::config::{AutoCompactConfig, DEFAULT_RESERVED_FOR_RESPONSE};
    use super::super::manager::AutoCompact;
    use super::super::result::CompactResult;
    use crate::context::compact::create_compact_boundary;
    use crate::llm::{LlmMessage, MessageRole};
    use chrono::Utc;
    use std::collections::HashMap;
    use uuid::Uuid;

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
                    &format!("User message {} with some content to fill space", i),
                ));
            } else {
                messages.push(create_message(
                    MessageRole::Assistant,
                    &format!("Assistant response {} with additional content", i),
                ));
            }
        }

        messages
    }

    #[test]
    fn test_config_default() {
        let config = AutoCompactConfig::default();
        assert!(config.enabled);
        assert_eq!(config.reserved_for_response, DEFAULT_RESERVED_FOR_RESPONSE);
        // Default: 128K - 13K = 115K threshold (~89.8%)
        assert_eq!(config.threshold_tokens(), 128_000 - 13_000);
    }

    #[test]
    fn test_config_for_provider() {
        let config = AutoCompactConfig::for_provider("anthropic", "claude-3.5-sonnet");
        assert_eq!(config.max_context_tokens, 200_000);
        assert_eq!(config.reserved_for_response, 13_000);
        // Claude 3.5: 200K - 13K = 187K threshold (~93.5%, matches Claude Code)
        assert_eq!(config.threshold_tokens(), 187_000);

        let config = AutoCompactConfig::for_provider("openai", "gpt-4-turbo");
        assert_eq!(config.max_context_tokens, 128_000);
        assert_eq!(config.reserved_for_response, 10_000);
    }

    #[test]
    fn test_threshold_percentage() {
        let config = AutoCompactConfig::for_provider("anthropic", "claude-3.5-sonnet");
        let pct = config.threshold_percentage();
        // 187K / 200K = 0.935 (93.5%)
        assert!((pct - 0.935).abs() < 0.01);
    }

    #[test]
    fn test_needs_compaction() {
        let config = AutoCompactConfig::default()
            .with_max_tokens(100)
            .with_reserved_tokens(50); // 50 tokens threshold

        let auto_compact = AutoCompact::new(config);

        // Small messages - no compaction needed
        let small = vec![create_message(MessageRole::User, "Hi")];
        assert!(!auto_compact.needs_compaction(&small));

        // Large messages - compaction needed
        let large = vec![create_message(MessageRole::User, &"x".repeat(300))];
        assert!(auto_compact.needs_compaction(&large));
    }

    #[test]
    fn test_partition_messages() {
        let config = AutoCompactConfig {
            preserve_recent_count: 2,
            min_messages_to_keep: 3,
            preserve_system_messages: true,
            ..Default::default()
        };
        let auto_compact = AutoCompact::new(config);

        let messages = create_test_messages(10);
        let (to_keep, to_compact) = auto_compact.partition_messages(&messages);

        // Should keep system message + recent messages
        assert!(to_keep.len() >= 3);
        assert!(!to_compact.is_empty());

        // System message should be preserved
        assert!(to_keep.iter().any(|m| m.role == MessageRole::System));
    }

    #[tokio::test]
    async fn test_force_compact() {
        let config = AutoCompactConfig::default();
        let mut auto_compact = AutoCompact::new(config);

        let mut messages = create_test_messages(20);
        let result = auto_compact.force_compact(&mut messages).await.unwrap();

        assert!(result.was_compacted);
        assert!(result.messages_after < result.messages_before);
        assert!(result.tokens_after < result.tokens_before);
        assert!(result.compact_id.is_some());
    }

    #[test]
    fn test_get_usage_percentage() {
        let mut config = AutoCompactConfig::default();
        config.max_context_tokens = 1000;
        let auto_compact = AutoCompact::new(config);

        // Create messages worth roughly 250 tokens
        let messages = vec![create_message(MessageRole::User, &"x".repeat(1000))];
        let usage = auto_compact.get_usage_percentage(&messages);

        assert!(usage > 0.0);
        assert!(usage <= 100.0);
    }

    #[test]
    fn test_compact_result_metrics() {
        let result = CompactResult {
            was_compacted: true,
            messages_before: 100,
            messages_after: 20,
            tokens_before: 50000,
            tokens_after: 10000,
            messages_compacted: 80,
            compacted_at: Some(Utc::now()),
            summary_preview: Some("Test summary...".to_string()),
            compact_id: Some(Uuid::new_v4()),
        };

        assert_eq!(result.tokens_saved(), 40000);
        assert!((result.compression_ratio() - 0.2).abs() < 0.01);
    }

    #[test]
    fn test_needs_compaction_respects_boundary() {
        let config = AutoCompactConfig::default()
            .with_max_tokens(100)
            .with_reserved_tokens(50); // 50 token threshold

        let auto_compact = AutoCompact::new(config);

        // Create messages with a boundary in the middle
        let old_large = create_message(MessageRole::User, &"x".repeat(500));
        let boundary = create_compact_boundary(Uuid::new_v4(), Utc::now());
        let new_small = create_message(MessageRole::User, "small");

        let messages = vec![old_large, boundary, new_small];

        // Should only consider messages after boundary, so no compaction needed
        assert!(!auto_compact.needs_compaction(&messages));
    }

    #[test]
    fn test_extract_summary() {
        use super::super::summary::extract_summary;

        // Test with tags
        let with_tags =
            "<analysis>thinking...</analysis>\n<summary>The actual summary</summary>\nextra";
        assert_eq!(extract_summary(with_tags), "The actual summary");

        // Test without tags
        let without_tags = "Just a plain summary";
        assert_eq!(extract_summary(without_tags), "Just a plain summary");
    }
}
