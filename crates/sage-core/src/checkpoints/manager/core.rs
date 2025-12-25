//! Core checkpoint management operations

use crate::checkpoints::restore::RestorePreview;
use crate::checkpoints::storage::CheckpointSummary;
use crate::checkpoints::types::{
    Checkpoint, CheckpointId, ConversationSnapshot, RestoreOptions, RestoreResult,
    ToolExecutionRecord,
};
use crate::error::{SageError, SageResult};

use super::types::CheckpointManager;

impl CheckpointManager {
    /// List all checkpoints
    pub async fn list_checkpoints(&self) -> SageResult<Vec<CheckpointSummary>> {
        self.storage.list().await
    }

    /// Get a specific checkpoint
    pub async fn get_checkpoint(&self, id: &CheckpointId) -> SageResult<Option<Checkpoint>> {
        self.storage.load(id).await
    }

    /// Get the latest checkpoint
    pub async fn latest_checkpoint(&self) -> SageResult<Option<Checkpoint>> {
        self.storage.latest().await
    }

    /// Delete a checkpoint
    pub async fn delete_checkpoint(&self, id: &CheckpointId) -> SageResult<()> {
        self.storage.delete(id).await
    }

    /// Delete all checkpoints
    pub async fn clear_all_checkpoints(&self) -> SageResult<usize> {
        let summaries = self.storage.list().await?;
        let count = summaries.len();

        for summary in summaries {
            self.storage.delete(&summary.id).await?;
        }

        tracing::info!("Cleared {} checkpoints", count);
        Ok(count)
    }

    /// Get checkpoint by short ID (prefix match)
    pub async fn find_by_short_id(&self, short_id: &str) -> SageResult<Option<Checkpoint>> {
        let summaries = self.storage.list().await?;

        for summary in summaries {
            if summary.id.as_str().starts_with(short_id) {
                return self.storage.load(&summary.id).await;
            }
        }

        Ok(None)
    }

    /// Add conversation snapshot to a checkpoint
    pub async fn add_conversation_snapshot(
        &self,
        checkpoint_id: &CheckpointId,
        conversation: ConversationSnapshot,
    ) -> SageResult<()> {
        let mut checkpoint = self.load_checkpoint_or_error(checkpoint_id).await?;
        checkpoint.conversation = Some(conversation);
        self.storage.save(&checkpoint).await
    }

    /// Add tool execution record to a checkpoint
    pub async fn add_tool_record(
        &self,
        checkpoint_id: &CheckpointId,
        record: ToolExecutionRecord,
    ) -> SageResult<()> {
        let mut checkpoint = self.load_checkpoint_or_error(checkpoint_id).await?;
        checkpoint.tool_history.push(record);
        self.storage.save(&checkpoint).await
    }

    /// Preview what would be restored
    pub async fn preview_restore(
        &self,
        checkpoint_id: &CheckpointId,
    ) -> SageResult<Vec<RestorePreview>> {
        let checkpoint = self.load_checkpoint_or_error(checkpoint_id).await?;
        let mut previews = Vec::new();

        for snapshot in &checkpoint.files {
            let preview =
                super::super::restore::preview_file_restore(&self.config.project_root, snapshot)
                    .await?;
            previews.push(preview);
        }

        Ok(previews)
    }

    /// Restore to a checkpoint
    pub async fn restore(
        &self,
        checkpoint_id: &CheckpointId,
        options: RestoreOptions,
    ) -> SageResult<RestoreResult> {
        let checkpoint = self.load_checkpoint_or_error(checkpoint_id).await?;

        tracing::info!("Restoring to checkpoint {}", checkpoint.short_id());

        let mut result = RestoreResult {
            checkpoint_id: checkpoint_id.clone(),
            restored_files: Vec::new(),
            failed_files: Vec::new(),
            conversation_restored: false,
            backup_checkpoint_id: None,
            was_dry_run: options.dry_run,
        };

        // Create backup if requested
        if options.create_backup && !options.dry_run {
            let backup = self
                .create_full_checkpoint(
                    format!("Backup before restore to {}", checkpoint.short_id()),
                    super::super::types::CheckpointType::Auto,
                )
                .await?;
            result.backup_checkpoint_id = Some(backup.id);
        }

        // Restore files
        if options.restore_files {
            self.restore_files(&checkpoint, &options, &mut result).await;
        }

        // Mark conversation as restored if present
        if options.restore_conversation && checkpoint.conversation.is_some() {
            result.conversation_restored = true;
        }

        tracing::info!(
            "Restore complete: {} files restored, {} failed",
            result.restored_count(),
            result.failed_count()
        );

        Ok(result)
    }

    // Private helper methods

    pub(super) async fn load_checkpoint_or_error(
        &self,
        id: &CheckpointId,
    ) -> SageResult<Checkpoint> {
        self.storage
            .load(id)
            .await?
            .ok_or_else(|| SageError::not_found(format!("Checkpoint {} not found", id)))
    }

    async fn restore_files(
        &self,
        checkpoint: &Checkpoint,
        options: &RestoreOptions,
        result: &mut RestoreResult,
    ) {
        for file_snapshot in &checkpoint.files {
            // Check file filter
            if !options.file_filter.is_empty() && !options.file_filter.contains(&file_snapshot.path)
            {
                continue;
            }

            if options.dry_run {
                result.restored_files.push(file_snapshot.path.clone());
                continue;
            }

            match super::super::restore::restore_file(&self.config.project_root, file_snapshot)
                .await
            {
                Ok(_) => result.restored_files.push(file_snapshot.path.clone()),
                Err(e) => result
                    .failed_files
                    .push((file_snapshot.path.clone(), e.to_string())),
            }
        }
    }
}
