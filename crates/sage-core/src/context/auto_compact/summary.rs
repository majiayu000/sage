//! Summary generation utilities for auto-compact

use crate::context::compact::{SummaryPromptConfig, build_summary_prompt};
use crate::error::SageResult;
use crate::llm::{LlmClient, LlmMessage, MessageRole};

/// Generate a summary of messages using Claude Code style prompt
pub async fn generate_summary(
    messages: &[LlmMessage],
    custom_instructions: Option<&str>,
    llm_client: Option<&LlmClient>,
) -> SageResult<String> {
    if let Some(client) = llm_client {
        // Use LLM for intelligent summarization with Claude Code style prompt
        let prompt_config = SummaryPromptConfig {
            custom_instructions: custom_instructions.map(|s| s.to_string()),
        };
        let prompt = build_summary_prompt(&prompt_config);

        // Format conversation for the prompt
        let conversation = format_messages_for_summary(messages);
        let full_prompt = format!(
            "{}\n\n---\nCONVERSATION TO SUMMARIZE:\n{}\n---",
            prompt, conversation
        );

        let summary_request = vec![LlmMessage::user(full_prompt)];
        let response = client.chat(&summary_request, None).await?;

        // Extract summary from response (handle <summary> tags if present)
        let summary = extract_summary(&response.content);

        Ok(format!(
            "# Previous Conversation Summary\n\n{}\n\n---\n*Summarized {} messages via auto-compact*",
            summary,
            messages.len()
        ))
    } else {
        // Fallback to simple extraction
        Ok(create_simple_summary(messages))
    }
}

/// Extract summary from LLM response, handling <summary> tags
pub fn extract_summary(response: &str) -> String {
    // Try to extract content between <summary> tags
    if let Some(start) = response.find("<summary>") {
        if let Some(end) = response.find("</summary>") {
            let summary_start = start + "<summary>".len();
            if summary_start < end {
                return response[summary_start..end].trim().to_string();
            }
        }
    }
    // If no tags, return the whole response
    response.trim().to_string()
}

/// Format messages for the summarization prompt
pub fn format_messages_for_summary(messages: &[LlmMessage]) -> String {
    messages
        .iter()
        .filter(|m| m.role != MessageRole::System)
        .map(|m| {
            let role = match m.role {
                MessageRole::User => "USER",
                MessageRole::Assistant => "ASSISTANT",
                MessageRole::Tool => "TOOL",
                MessageRole::System => "SYSTEM",
            };

            let content = truncate_content(&m.content, 1000);

            // Include tool info if present
            let tool_info = if let Some(ref tool_calls) = m.tool_calls {
                let tools: Vec<_> = tool_calls.iter().map(|tc| tc.name.as_str()).collect();
                format!(" [Tools: {}]", tools.join(", "))
            } else if let Some(ref tool_id) = m.tool_call_id {
                format!(" [Response to: {}]", tool_id)
            } else {
                String::new()
            };

            format!("[{}{}]: {}", role, tool_info, content)
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Truncate content to max characters (UTF-8 safe)
pub fn truncate_content(content: &str, max_chars: usize) -> String {
    crate::utils::truncate_with_ellipsis(content, max_chars)
}

/// Create a simple summary without LLM
pub fn create_simple_summary(messages: &[LlmMessage]) -> String {
    let mut user_count = 0;
    let mut assistant_count = 0;
    let mut tool_count = 0;
    let mut user_messages = Vec::new();

    for msg in messages {
        match msg.role {
            MessageRole::User => {
                user_count += 1;
                // Collect user messages (Claude Code requires all user messages)
                if let Some(first_line) = msg.content.lines().next() {
                    if first_line.len() > 10 && user_messages.len() < 10 {
                        user_messages.push(format!("- {}", truncate_content(first_line, 100)));
                    }
                }
            }
            MessageRole::Assistant => assistant_count += 1,
            MessageRole::Tool => tool_count += 1,
            _ => {}
        }
    }

    format!(
        r#"# Previous Conversation Summary

## Overview
- {} user messages
- {} assistant responses
- {} tool interactions

## User Messages
{}

---
*Simple summary of {} messages (auto-compact without LLM)*"#,
        user_count,
        assistant_count,
        tool_count,
        if user_messages.is_empty() {
            "- (No significant user messages captured)".to_string()
        } else {
            user_messages.join("\n")
        },
        messages.len()
    )
}
