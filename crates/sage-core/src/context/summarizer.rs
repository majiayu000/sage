//! Conversation summarization for context compression
//!
//! This module provides functionality to summarize conversation history
//! using an LLM, allowing for context compression while preserving
//! important information.

use crate::error::SageResult;
use crate::llm::{LlmClient, LlmMessage, MessageRole};
use std::collections::HashMap;
use std::sync::Arc;

/// Conversation summarizer that uses an LLM to create concise summaries
#[derive(Clone)]
pub struct ConversationSummarizer {
    /// LLM client for generating summaries
    llm_client: Option<Arc<LlmClient>>,
    /// Maximum tokens for the summary
    max_summary_tokens: usize,
    /// Model to use for summarization (if different from default)
    model: Option<String>,
}

impl ConversationSummarizer {
    /// Create a new summarizer without LLM (for testing)
    pub fn new() -> Self {
        Self {
            llm_client: None,
            max_summary_tokens: 500,
            model: None,
        }
    }

    /// Create a summarizer with an LLM client
    pub fn with_client(client: Arc<LlmClient>) -> Self {
        Self {
            llm_client: Some(client),
            max_summary_tokens: 500,
            model: None,
        }
    }

    /// Set maximum summary tokens
    pub fn with_max_tokens(mut self, max: usize) -> Self {
        self.max_summary_tokens = max;
        self
    }

    /// Set model to use for summarization
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Summarize a list of messages
    pub async fn summarize(&self, messages: &[LlmMessage]) -> SageResult<LlmMessage> {
        if messages.is_empty() {
            return Ok(self.create_empty_summary());
        }

        // If no LLM client, use simple extractive summary
        match &self.llm_client {
            Some(client) => self.summarize_with_llm(client, messages).await,
            None => Ok(self.create_simple_summary(messages)),
        }
    }

    /// Create summary using LLM
    async fn summarize_with_llm(
        &self,
        client: &LlmClient,
        messages: &[LlmMessage],
    ) -> SageResult<LlmMessage> {
        let prompt = self.build_summarization_prompt(messages);

        let summary_request = vec![LlmMessage {
            role: MessageRole::User,
            content: prompt,
            name: None,
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
            metadata: HashMap::new(),
        }];

        let response = client.chat(&summary_request, None).await?;

        Ok(LlmMessage {
            role: MessageRole::System,
            content: format!(
                "# Previous Conversation Summary\n\n{}\n\n---\n*Summarized {} messages*",
                response.content,
                messages.len()
            ),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
            metadata: HashMap::new(),
        })
    }

    /// Build the prompt for summarization
    fn build_summarization_prompt(&self, messages: &[LlmMessage]) -> String {
        let conversation = self.format_messages_for_summary(messages);

        format!(
            r#"Please summarize the following conversation concisely, preserving:

1. Key decisions and outcomes
2. Important tool results and findings
3. Current task context and progress
4. Any critical errors or warnings encountered

Be concise but comprehensive. Focus on information that would be needed to continue the conversation effectively.

Maximum summary length: {} tokens.

---
CONVERSATION TO SUMMARIZE:
{}
---

Provide a structured summary:"#,
            self.max_summary_tokens, conversation
        )
    }

    /// Format messages for the summarization prompt
    fn format_messages_for_summary(&self, messages: &[LlmMessage]) -> String {
        messages
            .iter()
            .filter(|m| m.role != MessageRole::System) // Skip system messages
            .map(|m| {
                let role = match m.role {
                    MessageRole::User => "USER",
                    MessageRole::Assistant => "ASSISTANT",
                    MessageRole::Tool => "TOOL",
                    MessageRole::System => "SYSTEM",
                };

                let content = self.truncate_content(&m.content, 500);

                // Include tool info if present
                let tool_info = if let Some(ref tool_calls) = m.tool_calls {
                    let tools: Vec<_> = tool_calls.iter().map(|tc| tc.name.as_str()).collect();
                    format!(" [Tools: {}]", tools.join(", "))
                } else if let Some(ref tool_id) = m.tool_call_id {
                    format!(" [Response to: {}]", tool_id)
                } else {
                    String::new()
                };

                format!("{}{}: {}", role, tool_info, content)
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Truncate content to max characters
    fn truncate_content(&self, content: &str, max_chars: usize) -> String {
        if content.len() <= max_chars {
            content.to_string()
        } else {
            format!("{}...", &content[..max_chars])
        }
    }

    /// Create a simple summary without LLM
    fn create_simple_summary(&self, messages: &[LlmMessage]) -> LlmMessage {
        let user_count = messages
            .iter()
            .filter(|m| m.role == MessageRole::User)
            .count();
        let assistant_count = messages
            .iter()
            .filter(|m| m.role == MessageRole::Assistant)
            .count();
        let tool_count = messages
            .iter()
            .filter(|m| m.role == MessageRole::Tool)
            .count();

        // Extract first and last non-system messages
        let first_msg = messages
            .iter()
            .find(|m| m.role != MessageRole::System)
            .map(|m| self.truncate_content(&m.content, 100))
            .unwrap_or_default();

        let last_msg = messages
            .iter()
            .rev()
            .find(|m| m.role != MessageRole::System)
            .map(|m| self.truncate_content(&m.content, 100))
            .unwrap_or_default();

        let summary = format!(
            r#"# Previous Conversation Summary

## Statistics
- {} user messages
- {} assistant messages
- {} tool interactions

## First Exchange
{}

## Last Exchange
{}

---
*Simple summary of {} messages (LLM summarization not available)*"#,
            user_count,
            assistant_count,
            tool_count,
            first_msg,
            last_msg,
            messages.len()
        );

        LlmMessage {
            role: MessageRole::System,
            content: summary,
            name: None,
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
            metadata: HashMap::new(),
        }
    }

    /// Create an empty summary message
    fn create_empty_summary(&self) -> LlmMessage {
        LlmMessage {
            role: MessageRole::System,
            content: "# Previous Conversation Summary\n\nNo previous conversation to summarize."
                .to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
            metadata: HashMap::new(),
        }
    }
}

impl Default for ConversationSummarizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_messages() -> Vec<LlmMessage> {
        vec![
            LlmMessage {
                role: MessageRole::User,
                content: "Hello, can you help me with a coding task?".to_string(),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                cache_control: None,
                metadata: HashMap::new(),
            },
            LlmMessage {
                role: MessageRole::Assistant,
                content: "Of course! I'd be happy to help. What are you working on?".to_string(),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                cache_control: None,
                metadata: HashMap::new(),
            },
            LlmMessage {
                role: MessageRole::User,
                content: "I need to implement a function that sorts an array.".to_string(),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                cache_control: None,
                metadata: HashMap::new(),
            },
            LlmMessage {
                role: MessageRole::Assistant,
                content: "Here's a simple sorting function...".to_string(),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                cache_control: None,
                metadata: HashMap::new(),
            },
        ]
    }

    #[tokio::test]
    async fn test_simple_summary() {
        let summarizer = ConversationSummarizer::new();
        let messages = create_test_messages();

        let summary = summarizer.summarize(&messages).await.unwrap();

        assert_eq!(summary.role, MessageRole::System);
        assert!(summary.content.contains("Summary"));
        assert!(summary.content.contains("user messages"));
    }

    #[tokio::test]
    async fn test_empty_messages() {
        let summarizer = ConversationSummarizer::new();
        let summary = summarizer.summarize(&[]).await.unwrap();

        assert_eq!(summary.role, MessageRole::System);
        assert!(summary.content.contains("No previous conversation"));
    }

    #[test]
    fn test_truncate_content() {
        let summarizer = ConversationSummarizer::new();

        let short = "Hello";
        assert_eq!(summarizer.truncate_content(short, 10), "Hello");

        let long = "This is a very long message that should be truncated";
        let truncated = summarizer.truncate_content(long, 20);
        assert!(truncated.ends_with("..."));
        assert!(truncated.len() <= 23); // 20 + "..."
    }

    #[test]
    fn test_format_messages() {
        let summarizer = ConversationSummarizer::new();
        let messages = create_test_messages();

        let formatted = summarizer.format_messages_for_summary(&messages);

        assert!(formatted.contains("USER:"));
        assert!(formatted.contains("ASSISTANT:"));
    }

    #[test]
    fn test_build_prompt() {
        let summarizer = ConversationSummarizer::new().with_max_tokens(300);
        let messages = create_test_messages();

        let prompt = summarizer.build_summarization_prompt(&messages);

        assert!(prompt.contains("300 tokens"));
        assert!(prompt.contains("CONVERSATION TO SUMMARIZE"));
    }

    #[test]
    fn test_builder_pattern() {
        let summarizer = ConversationSummarizer::new()
            .with_max_tokens(1000)
            .with_model("gpt-3.5-turbo");

        assert_eq!(summarizer.max_summary_tokens, 1000);
        assert_eq!(summarizer.model, Some("gpt-3.5-turbo".to_string()));
    }
}
