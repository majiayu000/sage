//! Session summary generation
//!
//! Following Claude Code's pattern of auto-generating conversation summaries.
//! Summaries help users quickly identify sessions without reading full history.

use super::enhanced::{EnhancedMessage, EnhancedMessageType};

/// Maximum length for auto-generated summary
const MAX_SUMMARY_LENGTH: usize = 150;

/// Minimum messages needed before generating summary
const MIN_MESSAGES_FOR_SUMMARY: usize = 2;

/// Summary generator for sessions
pub struct SummaryGenerator;

impl SummaryGenerator {
    /// Generate a summary from conversation messages
    ///
    /// This uses a simple heuristic approach:
    /// 1. Extract the first user message as the primary topic
    /// 2. Look for tool calls to understand what was done
    /// 3. Combine into a brief summary
    ///
    /// For more sophisticated summaries, use LLM-based generation.
    pub fn generate_simple(messages: &[EnhancedMessage]) -> Option<String> {
        // Need at least some messages
        if messages.len() < MIN_MESSAGES_FOR_SUMMARY {
            return None;
        }

        // Find first user message
        let first_user_msg = messages
            .iter()
            .find(|m| m.message_type == EnhancedMessageType::User)?;

        let user_content = &first_user_msg.message.content;

        // Extract tools used
        let tools_used: Vec<&str> = messages
            .iter()
            .filter(|m| m.message_type == EnhancedMessageType::Assistant)
            .filter_map(|m| m.message.tool_calls.as_ref())
            .flatten()
            .map(|tc| tc.name.as_str())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .take(3)
            .collect();

        // Build summary
        let mut summary = if user_content.len() > 80 {
            format!("{}...", &user_content[..77])
        } else {
            user_content.clone()
        };

        // Add tool info if available
        if !tools_used.is_empty() {
            let tools_str = tools_used.join(", ");
            let suffix = format!(" [used: {}]", tools_str);
            if summary.len() + suffix.len() <= MAX_SUMMARY_LENGTH {
                summary.push_str(&suffix);
            }
        }

        Some(summary)
    }

    /// Generate a summary from the first user prompt
    ///
    /// This is a simpler approach that just uses the first prompt.
    pub fn from_first_prompt(first_prompt: &str) -> String {
        if first_prompt.len() > MAX_SUMMARY_LENGTH {
            format!("{}...", &first_prompt[..MAX_SUMMARY_LENGTH - 3])
        } else {
            first_prompt.to_string()
        }
    }

    /// Check if messages warrant a summary update
    ///
    /// Returns true if:
    /// - There are enough new messages since last summary
    /// - Significant tool activity has occurred
    pub fn should_update_summary(
        messages: &[EnhancedMessage],
        last_summary_msg_count: usize,
    ) -> bool {
        let current_count = messages.len();
        let new_messages = current_count.saturating_sub(last_summary_msg_count);

        // Update every 10 messages or if significant activity
        if new_messages >= 10 {
            return true;
        }

        // Check for significant tool activity in new messages
        let tool_calls_in_new: usize = messages
            .iter()
            .skip(last_summary_msg_count)
            .filter(|m| m.message_type == EnhancedMessageType::Assistant)
            .filter_map(|m| m.message.tool_calls.as_ref())
            .map(|tc| tc.len())
            .sum();

        tool_calls_in_new >= 5
    }
}

/// Trait for LLM-based summary generation
///
/// Implement this trait to use an LLM for generating more sophisticated summaries.
#[async_trait::async_trait]
pub trait LlmSummaryGenerator: Send + Sync {
    /// Generate a summary using an LLM
    async fn generate(&self, messages: &[EnhancedMessage]) -> Option<String>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::enhanced::{MessageContent, EnhancedMessage, EnhancedMessageType, SessionContext};
    use chrono::Utc;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn create_test_message(msg_type: EnhancedMessageType, content: &str) -> EnhancedMessage {
        EnhancedMessage {
            message_type: msg_type,
            uuid: uuid::Uuid::new_v4().to_string(),
            parent_uuid: None,
            timestamp: Utc::now(),
            session_id: "test-session".to_string(),
            version: "0.1.0".to_string(),
            context: SessionContext::new(PathBuf::from("/tmp")),
            message: MessageContent {
                role: match msg_type {
                    EnhancedMessageType::User => "user".to_string(),
                    EnhancedMessageType::Assistant => "assistant".to_string(),
                    _ => "system".to_string(),
                },
                content: content.to_string(),
                tool_calls: None,
                tool_results: None,
            },
            usage: None,
            thinking_metadata: None,
            todos: vec![],
            is_sidechain: false,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn test_generate_simple_summary() {
        let messages = vec![
            create_test_message(EnhancedMessageType::User, "Help me fix the login bug"),
            create_test_message(
                EnhancedMessageType::Assistant,
                "I'll help you fix that bug.",
            ),
        ];

        let summary = SummaryGenerator::generate_simple(&messages);
        assert!(summary.is_some());
        assert!(summary.unwrap().contains("login bug"));
    }

    #[test]
    fn test_from_first_prompt() {
        let prompt = "This is a test prompt for summary generation";
        let summary = SummaryGenerator::from_first_prompt(prompt);
        assert_eq!(summary, prompt);

        // Test truncation
        let long_prompt = "a".repeat(200);
        let summary = SummaryGenerator::from_first_prompt(&long_prompt);
        assert!(summary.len() <= MAX_SUMMARY_LENGTH);
        assert!(summary.ends_with("..."));
    }

    #[test]
    fn test_should_update_summary() {
        let messages: Vec<EnhancedMessage> = (0..15)
            .map(|i| {
                create_test_message(
                    if i % 2 == 0 {
                        EnhancedMessageType::User
                    } else {
                        EnhancedMessageType::Assistant
                    },
                    &format!("Message {}", i),
                )
            })
            .collect();

        // Should update after 10 new messages
        assert!(SummaryGenerator::should_update_summary(&messages, 0));
        assert!(!SummaryGenerator::should_update_summary(&messages, 10));
    }
}
