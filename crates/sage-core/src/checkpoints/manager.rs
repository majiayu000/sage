//! Checkpoint manager
//!
//! This module provides the high-level checkpoint management API,
//! orchestrating checkpoint creation, restoration, and listing.

use crate::error::{SageError, SageResult};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::io::AsyncWriteExt;

use super::diff::{ChangeDetector, FileChange};
use super::storage::{CheckpointStorage, CheckpointSummary, FileCheckpointStorage};
use super::types::{
    Checkpoint, CheckpointId, CheckpointType, ConversationSnapshot, FileSnapshot, FileState,
    RestoreOptions, RestoreResult, ToolExecutionRecord,
};

/// Configuration for checkpoint manager
#[derive(Debug, Clone)]
pub struct CheckpointManagerConfig {
    /// Base directory for checkpoints
    pub storage_path: PathBuf,
    /// Project root directory
    pub project_root: PathBuf,
    /// Maximum number of checkpoints to keep
    pub max_checkpoints: usize,
    /// Auto-create checkpoint before tool execution
    pub auto_checkpoint_before_tools: bool,
    /// Tools that trigger auto-checkpoints
    pub checkpoint_tools: Vec<String>,
}

impl Default for CheckpointManagerConfig {
    fn default() -> Self {
        Self {
            storage_path: PathBuf::from(".sage/checkpoints"),
            project_root: PathBuf::from("."),
            max_checkpoints: 50,
            auto_checkpoint_before_tools: true,
            checkpoint_tools: vec!["Write".to_string(), "Edit".to_string(), "Bash".to_string()],
        }
    }
}

impl CheckpointManagerConfig {
    /// Create config with specific paths
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        let root = project_root.into();
        Self {
            storage_path: root.join(".sage/checkpoints"),
            project_root: root,
            ..Default::default()
        }
    }

    /// Set storage path
    pub fn with_storage_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.storage_path = path.into();
        self
    }

    /// Set max checkpoints
    pub fn with_max_checkpoints(mut self, max: usize) -> Self {
        self.max_checkpoints = max;
        self
    }

    /// Disable auto-checkpointing
    pub fn without_auto_checkpoint(mut self) -> Self {
        self.auto_checkpoint_before_tools = false;
        self
    }
}

/// Checkpoint manager for creating and restoring checkpoints
pub struct CheckpointManager {
    config: CheckpointManagerConfig,
    storage: Arc<dyn CheckpointStorage>,
    change_detector: ChangeDetector,
    /// Last known file states (for incremental checkpoints)
    last_states: tokio::sync::RwLock<Vec<FileSnapshot>>,
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

    /// Create a checkpoint with specific files
    pub async fn create_checkpoint(
        &self,
        description: impl Into<String>,
        checkpoint_type: CheckpointType,
        files: Vec<PathBuf>,
    ) -> SageResult<Checkpoint> {
        let description = description.into();
        tracing::info!("Creating {} checkpoint: {}", checkpoint_type, description);

        // Capture file states
        let snapshots = self.change_detector.capture_files(&files).await?;

        // Create checkpoint
        let checkpoint = Checkpoint::new(&description, checkpoint_type).with_files(snapshots);

        // Save checkpoint
        self.storage.save(&checkpoint).await?;

        // Update last known states
        {
            let mut last = self.last_states.write().await;
            *last = checkpoint.files.clone();
        }

        // Cleanup old checkpoints
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

        // Scan entire project
        let snapshots = self
            .change_detector
            .scan_directory(&self.config.project_root)
            .await?;

        // Create checkpoint
        let checkpoint = Checkpoint::new(&description, checkpoint_type).with_files(snapshots);

        // Save
        self.storage.save(&checkpoint).await?;

        // Update last known states
        {
            let mut last = self.last_states.write().await;
            *last = checkpoint.files.clone();
        }

        // Cleanup
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

        // Get current states
        let current_snapshots = self
            .change_detector
            .scan_directory(&self.config.project_root)
            .await?;

        // Compare with last known states
        let last_states = self.last_states.read().await;
        let changes = ChangeDetector::compare_snapshots(&last_states, &current_snapshots);
        drop(last_states);

        if changes.is_empty() {
            tracing::debug!("No changes detected, skipping checkpoint");
            // Return latest checkpoint if exists
            if let Some(latest) = self.storage.latest().await? {
                return Ok(latest);
            }
        }

        // Create snapshots from changes
        let change_snapshots = ChangeDetector::changes_to_snapshots(&changes);

        // Create checkpoint
        let checkpoint =
            Checkpoint::new(&description, checkpoint_type).with_files(change_snapshots);

        // Save
        self.storage.save(&checkpoint).await?;

        // Update last known states
        {
            let mut last = self.last_states.write().await;
            *last = current_snapshots;
        }

        // Cleanup
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

    /// Add conversation snapshot to a checkpoint
    pub async fn add_conversation_snapshot(
        &self,
        checkpoint_id: &CheckpointId,
        conversation: ConversationSnapshot,
    ) -> SageResult<()> {
        let mut checkpoint = self.storage.load(checkpoint_id).await?.ok_or_else(|| {
            SageError::not_found(format!("Checkpoint {} not found", checkpoint_id))
        })?;

        checkpoint.conversation = Some(conversation);
        self.storage.save(&checkpoint).await?;

        Ok(())
    }

    /// Add tool execution record to a checkpoint
    pub async fn add_tool_record(
        &self,
        checkpoint_id: &CheckpointId,
        record: ToolExecutionRecord,
    ) -> SageResult<()> {
        let mut checkpoint = self.storage.load(checkpoint_id).await?.ok_or_else(|| {
            SageError::not_found(format!("Checkpoint {} not found", checkpoint_id))
        })?;

        checkpoint.tool_history.push(record);
        self.storage.save(&checkpoint).await?;

        Ok(())
    }

    /// Restore to a checkpoint
    pub async fn restore(
        &self,
        checkpoint_id: &CheckpointId,
        options: RestoreOptions,
    ) -> SageResult<RestoreResult> {
        let checkpoint = self.storage.load(checkpoint_id).await?.ok_or_else(|| {
            SageError::not_found(format!("Checkpoint {} not found", checkpoint_id))
        })?;

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
                    CheckpointType::Auto,
                )
                .await?;
            result.backup_checkpoint_id = Some(backup.id);
        }

        // Restore files
        if options.restore_files {
            for file_snapshot in &checkpoint.files {
                // Check file filter
                if !options.file_filter.is_empty()
                    && !options.file_filter.contains(&file_snapshot.path)
                {
                    continue;
                }

                if options.dry_run {
                    result.restored_files.push(file_snapshot.path.clone());
                    continue;
                }

                match self.restore_file(file_snapshot).await {
                    Ok(_) => result.restored_files.push(file_snapshot.path.clone()),
                    Err(e) => result
                        .failed_files
                        .push((file_snapshot.path.clone(), e.to_string())),
                }
            }
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

    /// Restore a single file
    async fn restore_file(&self, snapshot: &FileSnapshot) -> SageResult<()> {
        let full_path = self.config.project_root.join(&snapshot.path);

        match &snapshot.state {
            FileState::Exists { content, .. } | FileState::Created { content, .. } => {
                if let Some(content) = content {
                    // Ensure parent directory exists
                    if let Some(parent) = full_path.parent() {
                        fs::create_dir_all(parent).await.map_err(|e| {
                            SageError::storage(format!("Failed to create directory: {}", e))
                        })?;
                    }

                    // Write content
                    let mut file = fs::File::create(&full_path)
                        .await
                        .map_err(|e| SageError::storage(format!("Failed to create file: {}", e)))?;
                    file.write_all(content.as_bytes())
                        .await
                        .map_err(|e| SageError::storage(format!("Failed to write file: {}", e)))?;

                    // Restore permissions
                    #[cfg(unix)]
                    if let Some(mode) = snapshot.permissions {
                        use std::os::unix::fs::PermissionsExt;
                        let perms = std::fs::Permissions::from_mode(mode);
                        fs::set_permissions(&full_path, perms).await.map_err(|e| {
                            SageError::storage(format!("Failed to set permissions: {}", e))
                        })?;
                    }
                }
            }
            FileState::Modified {
                original_content, ..
            } => {
                // Restore to original content
                if let Some(content) = original_content {
                    let mut file = fs::File::create(&full_path)
                        .await
                        .map_err(|e| SageError::storage(format!("Failed to create file: {}", e)))?;
                    file.write_all(content.as_bytes())
                        .await
                        .map_err(|e| SageError::storage(format!("Failed to write file: {}", e)))?;
                }
            }
            FileState::Deleted => {
                // File was deleted in this snapshot, so restore means... nothing?
                // Or we could delete if it exists now
                if full_path.exists() {
                    fs::remove_file(&full_path)
                        .await
                        .map_err(|e| SageError::storage(format!("Failed to delete file: {}", e)))?;
                }
            }
        }

        tracing::debug!("Restored file: {:?}", snapshot.path);
        Ok(())
    }

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

    /// Cleanup old checkpoints beyond max limit
    async fn cleanup_old_checkpoints(&self) -> SageResult<()> {
        let summaries = self.storage.list().await?;

        if summaries.len() > self.config.max_checkpoints {
            let to_remove = summaries.len() - self.config.max_checkpoints;
            tracing::debug!("Cleaning up {} old checkpoints", to_remove);

            // Remove oldest (they're sorted newest first)
            for summary in summaries.iter().rev().take(to_remove) {
                self.storage.delete(&summary.id).await?;
            }
        }

        Ok(())
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

    /// Get changes since last checkpoint
    pub async fn get_pending_changes(&self) -> SageResult<Vec<FileChange>> {
        let current_snapshots = self
            .change_detector
            .scan_directory(&self.config.project_root)
            .await?;

        let last_states = self.last_states.read().await;
        Ok(ChangeDetector::compare_snapshots(
            &last_states,
            &current_snapshots,
        ))
    }

    /// Preview what would be restored
    pub async fn preview_restore(
        &self,
        checkpoint_id: &CheckpointId,
    ) -> SageResult<Vec<RestorePreview>> {
        let checkpoint = self.storage.load(checkpoint_id).await?.ok_or_else(|| {
            SageError::not_found(format!("Checkpoint {} not found", checkpoint_id))
        })?;

        let mut previews = Vec::new();

        for snapshot in &checkpoint.files {
            let full_path = self.config.project_root.join(&snapshot.path);
            let current_exists = full_path.exists();

            let preview = match &snapshot.state {
                FileState::Exists { .. } | FileState::Created { .. } => {
                    if current_exists {
                        RestorePreview::WillOverwrite(snapshot.path.clone())
                    } else {
                        RestorePreview::WillCreate(snapshot.path.clone())
                    }
                }
                FileState::Modified { .. } => RestorePreview::WillRevert(snapshot.path.clone()),
                FileState::Deleted => {
                    if current_exists {
                        RestorePreview::WillDelete(snapshot.path.clone())
                    } else {
                        RestorePreview::NoChange(snapshot.path.clone())
                    }
                }
            };

            previews.push(preview);
        }

        Ok(previews)
    }
}

/// Preview of what will happen during restore
#[derive(Debug, Clone)]
pub enum RestorePreview {
    /// File will be created
    WillCreate(PathBuf),
    /// File will be overwritten
    WillOverwrite(PathBuf),
    /// File will be reverted to original
    WillRevert(PathBuf),
    /// File will be deleted
    WillDelete(PathBuf),
    /// No change needed
    NoChange(PathBuf),
}

impl RestorePreview {
    /// Get the path
    pub fn path(&self) -> &Path {
        match self {
            Self::WillCreate(p)
            | Self::WillOverwrite(p)
            | Self::WillRevert(p)
            | Self::WillDelete(p)
            | Self::NoChange(p) => p,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs::File;
    use tokio::io::AsyncWriteExt;

    async fn setup_test_project() -> (TempDir, CheckpointManager) {
        let temp_dir = TempDir::new().unwrap();

        let config = CheckpointManagerConfig::new(temp_dir.path()).with_max_checkpoints(10);

        let manager = CheckpointManager::new(config);

        // Create some test files
        let src_dir = temp_dir.path().join("src");
        fs::create_dir_all(&src_dir).await.unwrap();

        let mut main = File::create(src_dir.join("main.rs")).await.unwrap();
        main.write_all(b"fn main() { println!(\"Hello\"); }")
            .await
            .unwrap();

        let mut lib = File::create(src_dir.join("lib.rs")).await.unwrap();
        lib.write_all(b"pub mod utils;").await.unwrap();

        (temp_dir, manager)
    }

    #[tokio::test]
    async fn test_create_full_checkpoint() {
        let (_temp_dir, manager) = setup_test_project().await;

        let checkpoint = manager
            .create_full_checkpoint("Initial checkpoint", CheckpointType::Manual)
            .await
            .unwrap();

        assert_eq!(checkpoint.description, "Initial checkpoint");
        assert_eq!(checkpoint.checkpoint_type, CheckpointType::Manual);
        assert!(checkpoint.file_count() >= 2);
    }

    #[tokio::test]
    async fn test_create_checkpoint_specific_files() {
        let (temp_dir, manager) = setup_test_project().await;

        let files = vec![temp_dir.path().join("src/main.rs")];

        let checkpoint = manager
            .create_checkpoint("Single file", CheckpointType::PreTool, files)
            .await
            .unwrap();

        assert_eq!(checkpoint.file_count(), 1);
    }

    #[tokio::test]
    async fn test_list_checkpoints() {
        let (_temp_dir, manager) = setup_test_project().await;

        manager
            .create_full_checkpoint("First", CheckpointType::Manual)
            .await
            .unwrap();

        manager
            .create_full_checkpoint("Second", CheckpointType::Auto)
            .await
            .unwrap();

        let list = manager.list_checkpoints().await.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_restore_checkpoint() {
        let (temp_dir, manager) = setup_test_project().await;

        // Create initial checkpoint
        let checkpoint = manager
            .create_full_checkpoint("Before edit", CheckpointType::Manual)
            .await
            .unwrap();

        // Modify a file
        let main_path = temp_dir.path().join("src/main.rs");
        let mut file = File::create(&main_path).await.unwrap();
        file.write_all(b"fn main() { println!(\"Modified!\"); }")
            .await
            .unwrap();

        // Restore
        let result = manager
            .restore(
                &checkpoint.id,
                RestoreOptions::files_only().without_backup(),
            )
            .await
            .unwrap();

        assert!(result.is_success());
        assert!(!result.restored_files.is_empty());

        // Verify content restored
        let content = fs::read_to_string(&main_path).await.unwrap();
        assert!(content.contains("Hello"));
    }

    #[tokio::test]
    async fn test_dry_run_restore() {
        let (_temp_dir, manager) = setup_test_project().await;

        let checkpoint = manager
            .create_full_checkpoint("Test", CheckpointType::Manual)
            .await
            .unwrap();

        let result = manager
            .restore(&checkpoint.id, RestoreOptions::dry_run())
            .await
            .unwrap();

        assert!(result.was_dry_run);
        assert!(!result.restored_files.is_empty());
    }

    #[tokio::test]
    async fn test_incremental_checkpoint() {
        let (temp_dir, manager) = setup_test_project().await;

        // Create initial checkpoint
        manager
            .create_full_checkpoint("Initial", CheckpointType::Manual)
            .await
            .unwrap();

        // Modify a file
        let main_path = temp_dir.path().join("src/main.rs");
        let mut file = File::create(&main_path).await.unwrap();
        file.write_all(b"fn main() { println!(\"Changed!\"); }")
            .await
            .unwrap();

        // Create incremental
        let incremental = manager
            .create_incremental_checkpoint("After edit", CheckpointType::Auto)
            .await
            .unwrap();

        // Should only have the changed file
        assert!(incremental.file_count() <= 2);
    }

    #[tokio::test]
    async fn test_find_by_short_id() {
        let (_temp_dir, manager) = setup_test_project().await;

        let checkpoint = manager
            .create_full_checkpoint("Test", CheckpointType::Manual)
            .await
            .unwrap();

        let short_id = checkpoint.short_id();
        let found = manager.find_by_short_id(short_id).await.unwrap();

        assert!(found.is_some());
        assert_eq!(found.unwrap().id, checkpoint.id);
    }

    #[tokio::test]
    async fn test_cleanup_old_checkpoints() {
        let (_temp_dir, manager) = setup_test_project().await;

        // Create more than max_checkpoints
        for i in 0..15 {
            manager
                .create_full_checkpoint(format!("Checkpoint {}", i), CheckpointType::Auto)
                .await
                .unwrap();
        }

        let list = manager.list_checkpoints().await.unwrap();
        assert!(list.len() <= manager.config.max_checkpoints);
    }

    #[tokio::test]
    async fn test_preview_restore() {
        let (_temp_dir, manager) = setup_test_project().await;

        let checkpoint = manager
            .create_full_checkpoint("Test", CheckpointType::Manual)
            .await
            .unwrap();

        let previews = manager.preview_restore(&checkpoint.id).await.unwrap();

        assert!(!previews.is_empty());
    }

    #[tokio::test]
    async fn test_should_checkpoint_for_tool() {
        let (_temp_dir, manager) = setup_test_project().await;

        assert!(manager.should_checkpoint_for_tool("Write"));
        assert!(manager.should_checkpoint_for_tool("Edit"));
        assert!(manager.should_checkpoint_for_tool("Bash"));
        assert!(!manager.should_checkpoint_for_tool("Read"));
        assert!(!manager.should_checkpoint_for_tool("Glob"));
    }

    #[tokio::test]
    async fn test_get_pending_changes() {
        let (temp_dir, manager) = setup_test_project().await;

        // Create initial checkpoint
        manager
            .create_full_checkpoint("Initial", CheckpointType::Manual)
            .await
            .unwrap();

        // Create a new file
        let new_file = temp_dir.path().join("src/new.rs");
        let mut file = File::create(&new_file).await.unwrap();
        file.write_all(b"// New file").await.unwrap();

        let changes = manager.get_pending_changes().await.unwrap();

        assert!(!changes.is_empty());
        assert!(changes.iter().any(|c| c.is_created()));
    }

    #[tokio::test]
    async fn test_config_builder() {
        let config = CheckpointManagerConfig::new("/project")
            .with_storage_path("/custom/storage")
            .with_max_checkpoints(100)
            .without_auto_checkpoint();

        assert_eq!(config.storage_path, PathBuf::from("/custom/storage"));
        assert_eq!(config.max_checkpoints, 100);
        assert!(!config.auto_checkpoint_before_tools);
    }

    #[tokio::test]
    async fn test_delete_checkpoint() {
        let (_temp_dir, manager) = setup_test_project().await;

        let checkpoint = manager
            .create_full_checkpoint("To delete", CheckpointType::Manual)
            .await
            .unwrap();

        manager.delete_checkpoint(&checkpoint.id).await.unwrap();

        let found = manager.get_checkpoint(&checkpoint.id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_clear_all_checkpoints() {
        let (_temp_dir, manager) = setup_test_project().await;

        for i in 0..5 {
            manager
                .create_full_checkpoint(format!("Checkpoint {}", i), CheckpointType::Auto)
                .await
                .unwrap();
        }

        let cleared = manager.clear_all_checkpoints().await.unwrap();
        assert_eq!(cleared, 5);

        let list = manager.list_checkpoints().await.unwrap();
        assert!(list.is_empty());
    }
}
