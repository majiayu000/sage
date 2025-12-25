//! Checkpoint creation and change tracking operations

use crate::error::SageResult;
use std::path::PathBuf;

use super::super::diff::{FileChange, changes_to_snapshots, compare_snapshots};
use super::super::types::{Checkpoint, CheckpointType};
use super::types::CheckpointManager;

impl CheckpointManager {
    /// Create a checkpoint with specific files
    pub async fn create_checkpoint(
        &self,
        description: impl Into<String>,
        checkpoint_type: CheckpointType,
        files: Vec<PathBuf>,
    ) -> SageResult<Checkpoint> {
        let description = description.into();
        tracing::info!("Creating {} checkpoint: {}", checkpoint_type, description);

        let snapshots = self.change_detector.capture_files(&files).await?;
        let checkpoint = Checkpoint::new(&description, checkpoint_type).with_files(snapshots);

        self.storage.save(&checkpoint).await?;
        self.update_last_states(&checkpoint.files).await;
        self.cleanup_old_checkpoints().await?;

        tracing::info!(
            "Created checkpoint {} with {} files",
            checkpoint.short_id(),
            checkpoint.file_count()
        );

        Ok(checkpoint)
    }

    /// Create a full project checkpoint
    pub async fn create_full_checkpoint(
        &self,
        description: impl Into<String>,
        checkpoint_type: CheckpointType,
    ) -> SageResult<Checkpoint> {
        let description = description.into();
        tracing::info!(
            "Creating full {} checkpoint: {}",
            checkpoint_type,
            description
        );

        let snapshots = self
            .change_detector
            .scan_directory(&self.config.project_root)
            .await?;

        let checkpoint = Checkpoint::new(&description, checkpoint_type).with_files(snapshots);

        self.storage.save(&checkpoint).await?;
        self.update_last_states(&checkpoint.files).await;
        self.cleanup_old_checkpoints().await?;

        tracing::info!(
            "Created full checkpoint {} with {} files",
            checkpoint.short_id(),
            checkpoint.file_count()
        );

        Ok(checkpoint)
    }

    /// Create an incremental checkpoint (only changed files)
    pub async fn create_incremental_checkpoint(
        &self,
        description: impl Into<String>,
        checkpoint_type: CheckpointType,
    ) -> SageResult<Checkpoint> {
        let description = description.into();

        let current_snapshots = self
            .change_detector
            .scan_directory(&self.config.project_root)
            .await?;

        let last_states = self.last_states.read().await;
        let changes = compare_snapshots(&last_states, &current_snapshots);
        drop(last_states);

        if changes.is_empty() {
            tracing::debug!("No changes detected, skipping checkpoint");
            if let Some(latest) = self.storage.latest().await? {
                return Ok(latest);
            }
        }

        let change_snapshots = changes_to_snapshots(&changes);
        let checkpoint =
            Checkpoint::new(&description, checkpoint_type).with_files(change_snapshots);

        self.storage.save(&checkpoint).await?;
        self.update_last_states(&current_snapshots).await;
        self.cleanup_old_checkpoints().await?;

        tracing::info!(
            "Created incremental checkpoint {} with {} changes",
            checkpoint.short_id(),
            changes.len()
        );

        Ok(checkpoint)
    }

    /// Create pre-tool checkpoint
    pub async fn create_pre_tool_checkpoint(
        &self,
        tool_name: &str,
        affected_files: &[PathBuf],
    ) -> SageResult<Checkpoint> {
        let description = format!("Pre-{} checkpoint", tool_name);
        self.create_checkpoint(
            description,
            CheckpointType::PreTool,
            affected_files.to_vec(),
        )
        .await
    }

    /// Create session start checkpoint
    pub async fn create_session_start_checkpoint(
        &self,
        session_id: &str,
    ) -> SageResult<Checkpoint> {
        let description = format!("Session start: {}", &session_id[..8.min(session_id.len())]);
        self.create_full_checkpoint(description, CheckpointType::SessionStart)
            .await
    }

    /// Get changes since last checkpoint
    pub async fn get_pending_changes(&self) -> SageResult<Vec<FileChange>> {
        let current_snapshots = self
            .change_detector
            .scan_directory(&self.config.project_root)
            .await?;

        let last_states = self.last_states.read().await;
        Ok(compare_snapshots(&last_states, &current_snapshots))
    }
}
