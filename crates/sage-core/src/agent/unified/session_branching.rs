//! Session branching and file tracking functionality.

use crate::error::SageResult;
use crate::session::{JsonlSessionStorage, TodoItem};
use anyhow::Context;
use std::sync::Arc;
use tracing::instrument;

use super::UnifiedExecutor;

impl UnifiedExecutor {
    /// Create and record a file snapshot for the current message
    #[instrument(skip(self), fields(message_uuid = %message_uuid))]
    pub(super) async fn record_file_snapshot(&mut self, message_uuid: &str) -> SageResult<()> {
        if !self.session_manager.is_recording_active() {
            return Ok(());
        }

        // Only create snapshot if files were tracked
        if self.session_manager.is_file_tracker_empty() {
            return Ok(());
        }

        let snapshot = self
            .session_manager
            .create_file_snapshot(message_uuid)
            .await
            .context(format!(
                "Failed to create file snapshot for message: {}",
                message_uuid
            ))?;

        let storage = self.session_manager.jsonl_storage().cloned();
        let session_id = self
            .session_manager
            .current_session_id()
            .map(|s| s.to_string());

        if let (Some(storage), Some(session_id)) = (storage, session_id) {
            storage
                .append_snapshot(&session_id, &snapshot)
                .await
                .context(format!(
                    "Failed to append file snapshot to JSONL session: {}",
                    session_id
                ))?;
        }

        // Clear tracker for next round
        self.session_manager.clear_file_tracker();

        Ok(())
    }

    /// Update todos in the message tracker
    pub fn update_todos(&mut self, todos: Vec<TodoItem>) {
        self.session_manager.set_todos(todos);
    }

    /// Track a file for snapshot capability
    ///
    /// Call this before modifying files to enable undo.
    pub async fn track_file(&mut self, path: impl AsRef<std::path::Path>) -> SageResult<()> {
        self.session_manager.track_file(path).await
    }

    /// Create a sidechain session for branching
    ///
    /// Creates a new session that is marked as a sidechain (branch) of the current session.
    /// This is used for conversation branching (Claude Code style).
    #[instrument(skip(self))]
    pub async fn create_sidechain_session(&mut self) -> SageResult<String> {
        let parent_session_id = match self.session_manager.current_session_id() {
            Some(id) => id.to_string(),
            None => {
                return Err(crate::error::SageError::agent(
                    "No active session to branch from",
                ));
            }
        };

        let sidechain_id = uuid::Uuid::new_v4().to_string();

        if let Some(storage) = self.session_manager.jsonl_storage() {
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
            self.session_manager
                .reset_message_tracker(&sidechain_id, working_dir);

            self.session_manager
                .set_current_session_id(Some(sidechain_id.clone()));
            self.session_manager.set_last_summary_msg_count(0);

            tracing::info!(
                "Created sidechain session: {} (parent: {})",
                sidechain_id,
                parent_session_id
            );
        }

        Ok(self
            .session_manager
            .current_session_id()
            .map(|s| s.to_string())
            .unwrap_or_default())
    }

    /// Set JSONL storage (for external configuration)
    pub fn set_jsonl_storage(&mut self, storage: Arc<JsonlSessionStorage>) {
        self.session_manager.set_jsonl_storage(storage);
    }
}
