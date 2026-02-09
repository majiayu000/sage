//! Session recording functionality for user and assistant messages.

use crate::error::SageResult;
use crate::session::{EnhancedMessage, EnhancedTokenUsage, EnhancedToolCall};
use anyhow::Context;
use tracing::instrument;

use super::UnifiedExecutor;

impl UnifiedExecutor {
    /// Enable JSONL session recording
    ///
    /// Creates a new session and starts recording enhanced messages.
    #[instrument(skip(self))]
    pub async fn enable_session_recording(&mut self) -> SageResult<String> {
        let session_id = uuid::Uuid::new_v4().to_string();

        if let Some(storage) = self.session_manager.jsonl_storage() {
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
            self.session_manager
                .reset_message_tracker(&session_id, working_dir);

            self.session_manager
                .set_current_session_id(Some(session_id.clone()));

            tracing::info!("Started JSONL session recording: {}", session_id);
        }

        Ok(self
            .session_manager
            .current_session_id()
            .map(|s| s.to_string())
            .unwrap_or_default())
    }

    /// Record a user message
    ///
    /// Also captures first_prompt for session metadata (Claude Code style).
    #[instrument(skip(self, content), fields(content_len = %content.len()))]
    pub(super) async fn record_user_message(
        &mut self,
        content: &str,
    ) -> SageResult<Option<EnhancedMessage>> {
        if !self.session_manager.is_recording_active() {
            return Ok(None);
        }

        let msg = self
            .session_manager
            .message_tracker_mut()
            .create_user_message(content);

        let storage = self.session_manager.jsonl_storage().cloned();
        let session_id = self
            .session_manager
            .current_session_id()
            .map(|s| s.to_string());

        if let (Some(storage), Some(session_id)) = (storage, session_id) {
            storage
                .append_message(&session_id, &msg)
                .await
                .context(format!(
                    "Failed to append user message to JSONL session: {}",
                    session_id
                ))?;

            // Capture first_prompt and last_prompt for session metadata
            if let Ok(Some(mut metadata)) = storage.load_metadata(&session_id).await {
                // Set first_prompt only once (for session list display)
                metadata.set_first_prompt_if_empty(content);
                // Always update last_prompt (for resume display)
                metadata.set_last_prompt(content);
                if let Err(e) = storage.save_metadata(&session_id, &metadata).await {
                    tracing::warn!(
                        error = %e,
                        session_id = %session_id,
                        "Failed to save session metadata (non-fatal)"
                    );
                }
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
        if !self.session_manager.is_recording_active() {
            return Ok(None);
        }

        let mut msg = self
            .session_manager
            .message_tracker_mut()
            .create_assistant_message(content);

        if let Some(calls) = tool_calls {
            msg = msg.with_tool_calls(calls);
        }
        if let Some(u) = usage {
            msg = msg.with_usage(u);
        }

        // Clone values to avoid borrow conflicts
        let storage = self.session_manager.jsonl_storage().cloned();
        let session_id = self
            .session_manager
            .current_session_id()
            .map(|s| s.to_string());

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
        if !self.session_manager.is_recording_active() {
            return Ok(None);
        }

        let session_id = self
            .session_manager
            .current_session_id()
            .map(|s| s.to_string())
            .unwrap();
        let context = self.session_manager.message_tracker().context().clone();
        let parent_uuid = self
            .session_manager
            .message_tracker()
            .last_message_uuid()
            .map(|s| s.to_string());

        let msg =
            EnhancedMessage::error(error_type, error_message, &session_id, context, parent_uuid);

        if let Some(storage) = self.session_manager.jsonl_storage() {
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
                if let Err(e) = storage.save_metadata(&session_id, &metadata).await {
                    tracing::warn!(
                        error = %e,
                        session_id = %session_id,
                        "Failed to save error state metadata (non-fatal)"
                    );
                }
            }
        }

        tracing::error!(
            "Recorded error in session: [{}] {}",
            error_type,
            error_message
        );
        Ok(Some(msg))
    }
}
