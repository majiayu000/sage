//! Compact boundary system for conversation context management
//!
//! This module implements a boundary-based compaction system similar to Claude Code.
//! When conversation context is compacted, a boundary marker is inserted that serves as
//! a recovery point for subsequent compactions.
//!
//! ## Key Concepts
//!
//! - **Compact Boundary**: A special marker message inserted after compaction
//! - **Summary**: The condensed representation of previous conversation
//! - **Slice from Boundary**: Only consider messages after the last boundary
//!
//! ## Example
//!
//! ```ignore
//! let messages = vec![old_msg1, old_msg2, boundary, summary, new_msg1, new_msg2];
//! let sliced = slice_from_last_compact_boundary(&messages);
//! // sliced = [boundary, summary, new_msg1, new_msg2]
//! ```

use crate::llm::{LlmMessage, MessageRole};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Metadata key for compact boundary marker
pub const COMPACT_BOUNDARY_KEY: &str = "compact_boundary";

/// Metadata key for compact summary
pub const COMPACT_SUMMARY_KEY: &str = "compact_summary";

/// Metadata key for compact timestamp
pub const COMPACT_TIMESTAMP_KEY: &str = "compact_timestamp";

/// Metadata key for compact ID
pub const COMPACT_ID_KEY: &str = "compact_id";

/// Check if a message is a compact boundary marker
pub fn is_compact_boundary(message: &LlmMessage) -> bool {
    message.role == MessageRole::System
        && message
            .metadata
            .get(COMPACT_BOUNDARY_KEY)
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
}

/// Find the index of the last compact boundary in a message list
pub fn find_last_compact_boundary_index(messages: &[LlmMessage]) -> Option<usize> {
    messages
        .iter()
        .enumerate()
        .rev()
        .find(|(_, msg)| is_compact_boundary(msg))
        .map(|(idx, _)| idx)
}

/// Slice messages from the last compact boundary (inclusive)
///
/// If no boundary exists, returns all messages.
/// This ensures we only process messages after the most recent compaction.
pub fn slice_from_last_compact_boundary(messages: &[LlmMessage]) -> Vec<LlmMessage> {
    match find_last_compact_boundary_index(messages) {
        Some(idx) => messages[idx..].to_vec(),
        None => messages.to_vec(),
    }
}

/// Create a compact boundary marker message
pub fn create_compact_boundary(compact_id: Uuid, timestamp: DateTime<Utc>) -> LlmMessage {
    let mut metadata = HashMap::new();
    metadata.insert(COMPACT_BOUNDARY_KEY.to_string(), serde_json::json!(true));
    metadata.insert(
        COMPACT_ID_KEY.to_string(),
        serde_json::json!(compact_id.to_string()),
    );
    metadata.insert(
        COMPACT_TIMESTAMP_KEY.to_string(),
        serde_json::json!(timestamp.to_rfc3339()),
    );

    LlmMessage {
        role: MessageRole::System,
        content: format!(
            "--- Conversation Compacted at {} (ID: {}) ---",
            timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            &compact_id.to_string()[..8]
        ),
        tool_calls: None,
        tool_call_id: None,
        name: None,
        cache_control: None,
        metadata,
    }
}

/// Create a summary message with proper metadata
pub fn create_compact_summary(
    summary_content: String,
    compact_id: Uuid,
    messages_compacted: usize,
    tokens_before: usize,
    tokens_after: usize,
) -> LlmMessage {
    let mut metadata = HashMap::new();
    metadata.insert(COMPACT_SUMMARY_KEY.to_string(), serde_json::json!(true));
    metadata.insert(
        COMPACT_ID_KEY.to_string(),
        serde_json::json!(compact_id.to_string()),
    );
    metadata.insert(
        "messages_compacted".to_string(),
        serde_json::json!(messages_compacted),
    );
    metadata.insert(
        "tokens_before".to_string(),
        serde_json::json!(tokens_before),
    );
    metadata.insert("tokens_after".to_string(), serde_json::json!(tokens_after));

    LlmMessage {
        role: MessageRole::System,
        content: summary_content,
        tool_calls: None,
        tool_call_id: None,
        name: None,
        cache_control: None,
        metadata,
    }
}

/// Summary prompt template following Claude Code's 9-section structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryPromptConfig {
    /// Custom instructions from user (e.g., "/compact Focus on code")
    pub custom_instructions: Option<String>,
}

impl Default for SummaryPromptConfig {
    fn default() -> Self {
        Self {
            custom_instructions: None,
        }
    }
}

/// Build the summary prompt following Claude Code's detailed structure
///
/// This creates a comprehensive prompt that instructs the LLM to generate
/// a high-quality summary with 9 sections:
/// 1. Primary Request and Intent
/// 2. Key Technical Concepts
/// 3. Files and Code Sections
/// 4. Errors and Fixes
/// 5. Problem Solving
/// 6. All User Messages
/// 7. Pending Tasks
/// 8. Current Work
/// 9. Optional Next Step
pub fn build_summary_prompt(config: &SummaryPromptConfig) -> String {
    let custom_section = if let Some(ref instructions) = config.custom_instructions {
        format!(
            r#"

## Custom Summarization Instructions
{}

Please follow these custom instructions when creating your summary.
"#,
            instructions
        )
    } else {
        String::new()
    };

    format!(
        r#"Your task is to create a detailed summary of the conversation so far, paying close attention to the user's explicit requests and your previous actions.
This summary should be thorough in capturing technical details, code patterns, and architectural decisions that would be essential for continuing development work without losing context.

Before providing your final summary, wrap your analysis in <analysis> tags to organize your thoughts and ensure you've covered all necessary points. In your analysis process:

1. Chronologically analyze each message and section of the conversation. For each section thoroughly identify:
   - The user's explicit requests and intents
   - Your approach to addressing the user's requests
   - Key decisions, technical concepts and code patterns
   - Specific details like:
     - file names
     - full code snippets
     - function signatures
     - file edits
   - Errors that you ran into and how you fixed them
   - Pay special attention to specific user feedback that you received, especially if the user told you to do something differently.
2. Double-check for technical accuracy and completeness, addressing each required element thoroughly.

Your summary should include the following sections:

1. Primary Request and Intent: Capture all of the user's explicit requests and intents in detail
2. Key Technical Concepts: List all important technical concepts, technologies, and frameworks discussed.
3. Files and Code Sections: Enumerate specific files and code sections examined, modified, or created. Pay special attention to the most recent messages and include full code snippets where applicable and include a summary of why this file read or edit is important.
4. Errors and fixes: List all errors that you ran into, and how you fixed them. Pay special attention to specific user feedback that you received, especially if the user told you to do something differently.
5. Problem Solving: Document problems solved and any ongoing troubleshooting efforts.
6. All user messages: List ALL user messages that are not tool results. These are critical for understanding the users' feedback and changing intent.
7. Pending Tasks: Outline any pending tasks that you have explicitly been asked to work on.
8. Current Work: Describe in detail precisely what was being worked on immediately before this summary request, paying special attention to the most recent messages from both user and assistant. Include file names and code snippets where applicable.
9. Optional Next Step: List the next step that you will take that is related to the most recent work you were doing. IMPORTANT: ensure that this step is DIRECTLY in line with the user's most recent explicit requests, and the task you were working on immediately before this summary request. If your last task was concluded, then only list next steps if they are explicitly in line with the users request. Do not start on tangential requests or really old requests that were already completed without confirming with the user first.
   If there is a next step, include direct quotes from the most recent conversation showing exactly what task you were working on and where you left off. This should be verbatim to ensure there's no drift in task interpretation.

Here's an example of how your output should be structured:

<example>
<analysis>
[Your thought process, ensuring all points are covered thoroughly and accurately]
</analysis>

<summary>
1. Primary Request and Intent:
   [Detailed description]

2. Key Technical Concepts:
   - [Concept 1]
   - [Concept 2]
   - [...]

3. Files and Code Sections:
   - [File Name 1]
      - [Summary of why this file is important]
      - [Summary of the changes made to this file, if any]
      - [Important Code Snippet]
   - [File Name 2]
      - [Important Code Snippet]
   - [...]

4. Errors and fixes:
    - [Detailed description of error 1]:
      - [How you fixed the error]
      - [User feedback on the error if any]
    - [...]

5. Problem Solving:
   [Description of solved problems and ongoing troubleshooting]

6. All user messages:
    - [Detailed non tool use user message]
    - [...]

7. Pending Tasks:
   - [Task 1]
   - [Task 2]
   - [...]

8. Current Work:
   [Precise description of current work]

9. Optional Next Step:
   [Optional Next step to take]

</summary>
</example>

Please provide your summary based on the conversation so far, following this structure and ensuring precision and thoroughness in your response.{}"#,
        custom_section
    )
}

/// Result of a compact operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactOperationResult {
    /// Unique ID for this compaction
    pub compact_id: Uuid,
    /// When compaction occurred
    pub timestamp: DateTime<Utc>,
    /// Number of messages before compaction
    pub messages_before: usize,
    /// Number of messages after compaction
    pub messages_after: usize,
    /// Tokens before compaction
    pub tokens_before: usize,
    /// Tokens after compaction
    pub tokens_after: usize,
    /// The boundary marker message
    pub boundary_message: LlmMessage,
    /// The summary message
    pub summary_message: LlmMessage,
    /// Messages to keep after boundary
    pub messages_to_keep: Vec<LlmMessage>,
}

impl CompactOperationResult {
    /// Get the number of tokens saved
    pub fn tokens_saved(&self) -> usize {
        self.tokens_before.saturating_sub(self.tokens_after)
    }

    /// Get the compression ratio (0.0 = full compression, 1.0 = no compression)
    pub fn compression_ratio(&self) -> f32 {
        if self.tokens_before == 0 {
            1.0
        } else {
            self.tokens_after as f32 / self.tokens_before as f32
        }
    }

    /// Build the final message list after compaction
    pub fn build_compacted_messages(&self) -> Vec<LlmMessage> {
        let mut result = vec![self.boundary_message.clone(), self.summary_message.clone()];
        result.extend(self.messages_to_keep.clone());
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_message(role: MessageRole, content: &str) -> LlmMessage {
        LlmMessage {
            role,
            content: content.to_string(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            cache_control: None,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn test_is_compact_boundary() {
        let normal_msg = create_test_message(MessageRole::System, "Normal system message");
        assert!(!is_compact_boundary(&normal_msg));

        let boundary = create_compact_boundary(Uuid::new_v4(), Utc::now());
        assert!(is_compact_boundary(&boundary));
    }

    #[test]
    fn test_find_last_compact_boundary_index() {
        let msg1 = create_test_message(MessageRole::User, "Hello");
        let boundary1 = create_compact_boundary(Uuid::new_v4(), Utc::now());
        let msg2 = create_test_message(MessageRole::Assistant, "Hi");
        let boundary2 = create_compact_boundary(Uuid::new_v4(), Utc::now());
        let msg3 = create_test_message(MessageRole::User, "Thanks");

        let messages = vec![msg1, boundary1, msg2, boundary2.clone(), msg3];

        let idx = find_last_compact_boundary_index(&messages);
        assert_eq!(idx, Some(3)); // boundary2 is at index 3
    }

    #[test]
    fn test_find_no_boundary() {
        let messages = vec![
            create_test_message(MessageRole::User, "Hello"),
            create_test_message(MessageRole::Assistant, "Hi"),
        ];

        assert_eq!(find_last_compact_boundary_index(&messages), None);
    }

    #[test]
    fn test_slice_from_last_compact_boundary() {
        let msg1 = create_test_message(MessageRole::User, "Old message");
        let boundary = create_compact_boundary(Uuid::new_v4(), Utc::now());
        let summary = create_test_message(MessageRole::System, "Summary");
        let msg2 = create_test_message(MessageRole::User, "New message");

        let messages = vec![msg1, boundary.clone(), summary.clone(), msg2.clone()];
        let sliced = slice_from_last_compact_boundary(&messages);

        assert_eq!(sliced.len(), 3);
        assert!(is_compact_boundary(&sliced[0]));
        assert_eq!(sliced[1].content, "Summary");
        assert_eq!(sliced[2].content, "New message");
    }

    #[test]
    fn test_slice_no_boundary() {
        let messages = vec![
            create_test_message(MessageRole::User, "Hello"),
            create_test_message(MessageRole::Assistant, "Hi"),
        ];

        let sliced = slice_from_last_compact_boundary(&messages);
        assert_eq!(sliced.len(), 2);
    }

    #[test]
    fn test_build_summary_prompt_default() {
        let config = SummaryPromptConfig::default();
        let prompt = build_summary_prompt(&config);

        assert!(prompt.contains("Primary Request and Intent"));
        assert!(prompt.contains("Key Technical Concepts"));
        assert!(prompt.contains("Files and Code Sections"));
        assert!(prompt.contains("Errors and fixes"));
        assert!(prompt.contains("Problem Solving"));
        assert!(prompt.contains("All user messages"));
        assert!(prompt.contains("Pending Tasks"));
        assert!(prompt.contains("Current Work"));
        assert!(prompt.contains("Optional Next Step"));
        assert!(prompt.contains("<analysis>"));
    }

    #[test]
    fn test_build_summary_prompt_with_custom() {
        let config = SummaryPromptConfig {
            custom_instructions: Some("Focus on TypeScript code changes".to_string()),
        };
        let prompt = build_summary_prompt(&config);

        assert!(prompt.contains("Focus on TypeScript code changes"));
        assert!(prompt.contains("Custom Summarization Instructions"));
    }

    #[test]
    fn test_create_compact_summary() {
        let summary =
            create_compact_summary("Test summary".to_string(), Uuid::new_v4(), 50, 10000, 2000);

        assert_eq!(summary.role, MessageRole::System);
        assert_eq!(summary.content, "Test summary");
        assert!(summary.metadata.contains_key(COMPACT_SUMMARY_KEY));
        assert_eq!(
            summary.metadata.get("messages_compacted"),
            Some(&serde_json::json!(50))
        );
    }

    #[test]
    fn test_compact_operation_result() {
        let compact_id = Uuid::new_v4();
        let timestamp = Utc::now();

        let result = CompactOperationResult {
            compact_id,
            timestamp,
            messages_before: 100,
            messages_after: 10,
            tokens_before: 50000,
            tokens_after: 5000,
            boundary_message: create_compact_boundary(compact_id, timestamp),
            summary_message: create_compact_summary(
                "Summary".to_string(),
                compact_id,
                90,
                50000,
                5000,
            ),
            messages_to_keep: vec![create_test_message(MessageRole::User, "Recent msg")],
        };

        assert_eq!(result.tokens_saved(), 45000);
        assert!((result.compression_ratio() - 0.1).abs() < 0.01);

        let compacted = result.build_compacted_messages();
        assert_eq!(compacted.len(), 3); // boundary + summary + 1 kept message
    }
}
