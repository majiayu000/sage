//! Session recording, file tracking, and session restore for the unified executor

use crate::error::{SageError, SageResult};
use crate::llm::messages::LlmMessage;
use crate::session::{
    EnhancedMessage, EnhancedMessageType, EnhancedTokenUsage, EnhancedToolCall,
    JsonlSessionStorage, SessionMetadata, SummaryGenerator, TodoItem,
};
use crate::tools::ToolCall;
use anyhow::Context;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::instrument;

use super::UnifiedExecutor;

impl UnifiedExecutor {
    /// Enable JSONL session recording
    ///
    /// Creates a new session and starts recording enhanced messages.
    #[instrument(skip(self))]
    pub async fn enable_session_recording(&mut self) -> SageResult<String> {
        let session_id = uuid::Uuid::new_v4().to_string();

        if let Some(storage) = &self.jsonl_storage {
            let working_dir = self
                .options
                .working_directory
                .clone()
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

            // Create session
            let mut metadata = storage
                .create_session(&session_id, working_dir.clone())
                .await
                .context(format!(
                    "Failed to create JSONL session with ID: {}",
                    session_id
                ))?;

            // Set model info
            if let Ok(params) = self.config.default_model_parameters() {
                metadata = metadata.with_model(&params.model);
            }
            storage
                .save_metadata(&session_id, &metadata)
                .await
                .context(format!(
                    "Failed to save session metadata for session: {}",
                    session_id
                ))?;

            // Update tracker
            let mut context = crate::session::SessionContext::new(working_dir);
            context.detect_git_branch();
            self.message_tracker = crate::session::MessageChainTracker::new()
                .with_session(&session_id)
                .with_context(context);

            self.current_session_id = Some(session_id.clone());

            tracing::info!("Started JSONL session recording: {}", session_id);
        }

        Ok(self.current_session_id.clone().unwrap_or_default())
    }

    /// Record a user message
    ///
    /// Also captures first_prompt for session metadata (Claude Code style).
    #[instrument(skip(self, content), fields(content_len = %content.len()))]
    pub(super) async fn record_user_message(
        &mut self,
        content: &str,
    ) -> SageResult<Option<EnhancedMessage>> {
        if self.current_session_id.is_none() || self.jsonl_storage.is_none() {
            return Ok(None);
        }

        let msg = self.message_tracker.create_user_message(content);

        if let (Some(storage), Some(session_id)) = (&self.jsonl_storage, &self.current_session_id) {
            storage
                .append_message(session_id, &msg)
                .await
                .context(format!(
                    "Failed to append user message to JSONL session: {}",
                    session_id
                ))?;

            // Capture first_prompt and last_prompt for session metadata
            if let Ok(Some(mut metadata)) = storage.load_metadata(session_id).await {
                // Set first_prompt only once (for session list display)
                metadata.set_first_prompt_if_empty(content);
                // Always update last_prompt (for resume display)
                metadata.set_last_prompt(content);
                let _ = storage.save_metadata(session_id, &metadata).await;
            }
        }

        Ok(Some(msg))
    }

    /// Record an assistant message
    ///
    /// Also triggers summary generation when appropriate (Claude Code style).
    #[instrument(skip(self, content, tool_calls, usage), fields(content_len = %content.len(), tool_calls_count = tool_calls.as_ref().map(|tc| tc.len()).unwrap_or(0)))]
    pub(super) async fn record_assistant_message(
        &mut self,
        content: &str,
        tool_calls: Option<Vec<EnhancedToolCall>>,
        usage: Option<EnhancedTokenUsage>,
    ) -> SageResult<Option<EnhancedMessage>> {
        if self.current_session_id.is_none() || self.jsonl_storage.is_none() {
            return Ok(None);
        }

        let mut msg = self.message_tracker.create_assistant_message(content);

        if let Some(calls) = tool_calls {
            msg = msg.with_tool_calls(calls);
        }
        if let Some(u) = usage {
            msg = msg.with_usage(u);
        }

        // Clone values to avoid borrow conflicts
        let storage = self.jsonl_storage.clone();
        let session_id = self.current_session_id.clone();

        if let (Some(storage), Some(session_id)) = (storage, session_id) {
            storage
                .append_message(&session_id, &msg)
                .await
                .context(format!(
                    "Failed to append assistant message to JSONL session: {}",
                    session_id
                ))?;

            // Auto-generate summary when appropriate (Claude Code style)
            self.maybe_update_summary(&storage, &session_id).await;
        }

        Ok(Some(msg))
    }

    /// Record an error message to the session
    ///
    /// This ensures errors are visible in the session history for debugging.
    #[instrument(skip(self, error_message), fields(error_type = %error_type))]
    pub(super) async fn record_error_message(
        &mut self,
        error_type: &str,
        error_message: &str,
    ) -> SageResult<Option<EnhancedMessage>> {
        if self.current_session_id.is_none() || self.jsonl_storage.is_none() {
            return Ok(None);
        }

        let session_id = self.current_session_id.clone().unwrap();
        let context = self.message_tracker.context().clone();
        let parent_uuid = self
            .message_tracker
            .last_message_uuid()
            .map(|s| s.to_string());

        let msg =
            EnhancedMessage::error(error_type, error_message, &session_id, context, parent_uuid);

        if let Some(storage) = &self.jsonl_storage {
            storage
                .append_message(&session_id, &msg)
                .await
                .context(format!(
                    "Failed to append error message to JSONL session: {}",
                    session_id
                ))?;

            // Update metadata to reflect error state
            if let Ok(Some(mut metadata)) = storage.load_metadata(&session_id).await {
                metadata.state = "failed".to_string();
                let _ = storage.save_metadata(&session_id, &metadata).await;
            }
        }

        tracing::error!(
            "Recorded error in session: [{}] {}",
            error_type,
            error_message
        );
        Ok(Some(msg))
    }

    /// Check and update session summary if needed
    async fn maybe_update_summary(
        &mut self,
        storage: &crate::session::JsonlSessionStorage,
        session_id: &str,
    ) {
        // Load messages to check if summary update is needed
        if let Ok(messages) = storage.load_messages(&session_id.to_string()).await {
            if SummaryGenerator::should_update_summary(&messages, self.last_summary_msg_count) {
                // Generate new summary
                if let Some(summary) = SummaryGenerator::generate_simple(&messages) {
                    // Update metadata with new summary
                    if let Ok(Some(mut metadata)) =
                        storage.load_metadata(&session_id.to_string()).await
                    {
                        metadata.set_summary(&summary);
                        if storage
                            .save_metadata(&session_id.to_string(), &metadata)
                            .await
                            .is_ok()
                        {
                            self.last_summary_msg_count = messages.len();
                            tracing::debug!("Updated session summary: {}", summary);
                        }
                    }
                }
            }
        }
    }

    /// Create and record a file snapshot for the current message
    #[instrument(skip(self), fields(message_uuid = %message_uuid))]
    pub(super) async fn record_file_snapshot(&mut self, message_uuid: &str) -> SageResult<()> {
        if self.current_session_id.is_none() || self.jsonl_storage.is_none() {
            return Ok(());
        }

        // Only create snapshot if files were tracked
        if self.file_tracker.is_empty() {
            return Ok(());
        }

        let snapshot = self
            .file_tracker
            .create_snapshot(message_uuid)
            .await
            .context(format!(
                "Failed to create file snapshot for message: {}",
                message_uuid
            ))?;

        if let (Some(storage), Some(session_id)) = (&self.jsonl_storage, &self.current_session_id) {
            storage
                .append_snapshot(session_id, &snapshot)
                .await
                .context(format!(
                    "Failed to append file snapshot to JSONL session: {}",
                    session_id
                ))?;
        }

        // Clear tracker for next round
        self.file_tracker.clear();

        Ok(())
    }

    /// Update todos in the message tracker
    pub fn update_todos(&mut self, todos: Vec<TodoItem>) {
        self.message_tracker.set_todos(todos);
    }

    /// Track a file for snapshot capability
    ///
    /// Call this before modifying files to enable undo.
    pub async fn track_file(&mut self, path: impl AsRef<std::path::Path>) -> SageResult<()> {
        self.file_tracker.track_file(path).await
    }

    /// Create a sidechain session for branching
    ///
    /// Creates a new session that is marked as a sidechain (branch) of the current session.
    /// This is used for conversation branching (Claude Code style).
    #[instrument(skip(self))]
    pub async fn create_sidechain_session(&mut self) -> SageResult<String> {
        let parent_session_id = match &self.current_session_id {
            Some(id) => id.clone(),
            None => {
                return Err(crate::error::SageError::agent(
                    "No active session to branch from",
                ));
            }
        };

        let sidechain_id = uuid::Uuid::new_v4().to_string();

        if let Some(storage) = &self.jsonl_storage {
            let working_dir = self
                .options
                .working_directory
                .clone()
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

            // Create sidechain session
            let mut metadata = storage
                .create_sidechain_session(&sidechain_id, &parent_session_id, working_dir.clone())
                .await
                .context(format!(
                    "Failed to create sidechain session with ID: {}",
                    sidechain_id
                ))?;

            // Set model info
            if let Ok(params) = self.config.default_model_parameters() {
                metadata = metadata.with_model(&params.model);
            }
            storage
                .save_metadata(&sidechain_id, &metadata)
                .await
                .context(format!(
                    "Failed to save sidechain session metadata: {}",
                    sidechain_id
                ))?;

            // Update tracker for new sidechain session
            let mut context = crate::session::SessionContext::new(working_dir);
            context.detect_git_branch();
            self.message_tracker = crate::session::MessageChainTracker::new()
                .with_session(&sidechain_id)
                .with_context(context);

            self.current_session_id = Some(sidechain_id.clone());
            self.last_summary_msg_count = 0;

            tracing::info!(
                "Created sidechain session: {} (parent: {})",
                sidechain_id,
                parent_session_id
            );
        }

        Ok(self.current_session_id.clone().unwrap_or_default())
    }

    /// Restore session from storage
    ///
    /// Loads messages from a previous session and restores the conversation state.
    /// This enables the `-c` (continue) and `-r` (resume) CLI flags.
    #[instrument(skip(self))]
    pub async fn restore_session(&mut self, session_id: &str) -> SageResult<Vec<LlmMessage>> {
        let storage = self.jsonl_storage.as_ref().ok_or_else(|| {
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

        // Convert EnhancedMessages to LlmMessages for the execution loop
        let llm_messages = Self::convert_messages_for_resume(&enhanced_messages);

        // Update executor state
        self.current_session_id = Some(session_id.to_string());

        // Restore message tracker context
        let working_dir = metadata.working_directory.clone();
        let mut context = crate::session::SessionContext::new(working_dir);
        context.detect_git_branch();
        self.message_tracker = crate::session::MessageChainTracker::new()
            .with_session(session_id)
            .with_context(context);

        // Set last parent UUID from restored messages
        if let Some(last_msg) = enhanced_messages.last() {
            self.message_tracker.set_last_uuid(&last_msg.uuid);
        }

        self.last_summary_msg_count = enhanced_messages.len();

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
            .jsonl_storage
            .as_ref()
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

    /// Convert EnhancedMessages to LlmMessages for continuing execution
    ///
    /// This preserves the conversation history including tool calls and results.
    fn convert_messages_for_resume(messages: &[EnhancedMessage]) -> Vec<LlmMessage> {
        let mut llm_messages = Vec::new();

        for msg in messages {
            // Skip metadata messages (summary, custom_title, file_history_snapshot)
            if msg.message_type.is_metadata() {
                continue;
            }

            match msg.message_type {
                EnhancedMessageType::User => {
                    llm_messages.push(LlmMessage::user(&msg.message.content));
                }
                EnhancedMessageType::Assistant => {
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
                EnhancedMessageType::ToolResult => {
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
                EnhancedMessageType::System => {
                    llm_messages.push(LlmMessage::system(&msg.message.content));
                }
                // Metadata types are skipped above
                _ => {}
            }
        }

        llm_messages
    }

    /// Set JSONL storage (for external configuration)
    pub fn set_jsonl_storage(&mut self, storage: Arc<JsonlSessionStorage>) {
        self.jsonl_storage = Some(storage);
    }
}
