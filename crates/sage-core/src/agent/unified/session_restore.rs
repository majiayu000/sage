//! Session restoration and message conversion for resume functionality.

use crate::error::{SageError, SageResult};
use crate::llm::messages::LlmMessage;
use crate::session::{SessionMessage, SessionMessageType, SessionMetadata};
use crate::tools::ToolCall;
use std::collections::HashMap;
use tracing::instrument;

use super::UnifiedExecutor;

impl UnifiedExecutor {
    /// Restore session from storage
    ///
    /// Loads messages from a previous session and restores the conversation state.
    /// This enables the `-c` (continue) and `-r` (resume) CLI flags.
    #[instrument(skip(self))]
    pub async fn restore_session(&mut self, session_id: &str) -> SageResult<Vec<LlmMessage>> {
        let storage = self.session_manager.jsonl_storage().ok_or_else(|| {
            SageError::config("JSONL storage not configured - cannot restore session")
        })?;

        let session_id_string = session_id.to_string();

        // Verify session exists
        if !storage.session_exists(&session_id_string).await {
            return Err(SageError::config(format!(
                "Session not found: {}",
                session_id
            )));
        }

        // Load session metadata
        let metadata = storage
            .load_metadata(&session_id_string)
            .await?
            .ok_or_else(|| {
                SageError::config(format!("Session metadata not found: {}", session_id))
            })?;

        // Load all messages from the session
        let enhanced_messages = storage.load_messages(&session_id_string).await?;

        tracing::info!(
            "Restoring session {} with {} messages",
            session_id,
            enhanced_messages.len()
        );

        // Convert SessionMessages to LlmMessages for the execution loop
        let llm_messages = Self::convert_messages_for_resume(&enhanced_messages);

        // Update executor state
        self.session_manager
            .set_current_session_id(Some(session_id.to_string()));

        // Restore message tracker context
        let working_dir = metadata.working_directory.clone();
        self.session_manager
            .reset_message_tracker(session_id, working_dir);

        // Set last parent UUID from restored messages
        if let Some(last_msg) = enhanced_messages.last() {
            self.session_manager
                .message_tracker_mut()
                .set_last_uuid(&last_msg.uuid);
        }

        self.session_manager
            .set_last_summary_msg_count(enhanced_messages.len());

        tracing::info!(
            "Session {} restored successfully (title: {})",
            session_id,
            metadata.display_title()
        );

        Ok(llm_messages)
    }

    /// Get the most recent session for the current working directory
    #[instrument(skip(self))]
    pub async fn get_most_recent_session(&self) -> SageResult<Option<SessionMetadata>> {
        let storage = self
            .session_manager
            .jsonl_storage()
            .ok_or_else(|| SageError::config("JSONL storage not configured"))?;

        let sessions = storage.list_sessions().await?;

        // Sessions are already sorted by updated_at descending
        // Filter to current working directory if available
        let working_dir = self.options.working_directory.as_ref();

        let session = if let Some(wd) = working_dir {
            sessions.into_iter().find(|s| &s.working_directory == wd)
        } else {
            sessions.into_iter().next()
        };

        Ok(session)
    }

    /// Convert SessionMessages to LlmMessages for continuing execution
    ///
    /// This preserves the conversation history including tool calls and results.
    fn convert_messages_for_resume(messages: &[SessionMessage]) -> Vec<LlmMessage> {
        let mut llm_messages = Vec::new();

        for msg in messages {
            // Skip metadata messages (summary, custom_title, file_history_snapshot)
            if msg.message_type.is_metadata() {
                continue;
            }

            match msg.message_type {
                SessionMessageType::User => {
                    llm_messages.push(LlmMessage::user(&msg.message.content));
                }
                SessionMessageType::Assistant => {
                    if let Some(ref tool_calls) = msg.message.tool_calls {
                        // Convert enhanced tool calls to ToolCall
                        let calls: Vec<ToolCall> = tool_calls
                            .iter()
                            .map(|tc| {
                                let args: HashMap<String, serde_json::Value> =
                                    if let serde_json::Value::Object(map) = &tc.arguments {
                                        map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
                                    } else {
                                        HashMap::new()
                                    };
                                ToolCall::new(&tc.id, &tc.name, args)
                            })
                            .collect();
                        llm_messages.push(LlmMessage::assistant_with_tools(
                            &msg.message.content,
                            calls,
                        ));
                    } else {
                        llm_messages.push(LlmMessage::assistant(&msg.message.content));
                    }
                }
                SessionMessageType::ToolResult => {
                    // Handle tool results
                    if let Some(ref tool_results) = msg.message.tool_results {
                        for result in tool_results {
                            llm_messages.push(LlmMessage::tool(
                                &result.content,
                                &result.tool_call_id,
                                Some(&result.tool_name),
                            ));
                        }
                    }
                }
                SessionMessageType::System => {
                    llm_messages.push(LlmMessage::system(&msg.message.content));
                }
                // Metadata types are skipped above
                _ => {}
            }
        }

        llm_messages
    }
}
