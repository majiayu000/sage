//! Checkpoint manager types and construction

use crate::error::SageResult;
use std::sync::Arc;

use super::super::config::CheckpointManagerConfig;
use super::super::diff::ChangeDetector;
use super::super::storage::{CheckpointStorage, FileCheckpointStorage};
use super::super::types::FileSnapshot;

/// Checkpoint manager for creating and restoring checkpoints
pub struct CheckpointManager {
    pub(super) config: CheckpointManagerConfig,
    pub(super) storage: Arc<dyn CheckpointStorage>,
    pub(super) change_detector: ChangeDetector,
    /// Last known file states (for incremental checkpoints)
    pub(super) last_states: tokio::sync::RwLock<Vec<FileSnapshot>>,
}

impl CheckpointManager {
    /// Create a new checkpoint manager
    pub fn new(config: CheckpointManagerConfig) -> Self {
        let storage = Arc::new(FileCheckpointStorage::new(&config.storage_path));
        let change_detector = ChangeDetector::new(&config.project_root);

        Self {
            config,
            storage,
            change_detector,
            last_states: tokio::sync::RwLock::new(Vec::new()),
        }
    }

    /// Create with custom storage
    pub fn with_storage(
        config: CheckpointManagerConfig,
        storage: Arc<dyn CheckpointStorage>,
    ) -> Self {
        let change_detector = ChangeDetector::new(&config.project_root);

        Self {
            config,
            storage,
            change_detector,
            last_states: tokio::sync::RwLock::new(Vec::new()),
        }
    }

    /// Get the configuration
    pub fn config(&self) -> &CheckpointManagerConfig {
        &self.config
    }

    /// Check if a tool should trigger auto-checkpoint
    pub fn should_checkpoint_for_tool(&self, tool_name: &str) -> bool {
        self.config.auto_checkpoint_before_tools
            && self
                .config
                .checkpoint_tools
                .iter()
                .any(|t| t.eq_ignore_ascii_case(tool_name))
    }

    // Internal helper methods

    pub(super) async fn update_last_states(&self, snapshots: &[FileSnapshot]) {
        let mut last = self.last_states.write().await;
        *last = snapshots.to_vec();
    }

    pub(super) async fn cleanup_old_checkpoints(&self) -> SageResult<()> {
        let summaries = self.storage.list().await?;

        if summaries.len() > self.config.max_checkpoints {
            let to_remove = summaries.len() - self.config.max_checkpoints;
            tracing::debug!("Cleaning up {} old checkpoints", to_remove);

            for summary in summaries.iter().rev().take(to_remove) {
                self.storage.delete(&summary.id).await?;
            }
        }

        Ok(())
    }
}
