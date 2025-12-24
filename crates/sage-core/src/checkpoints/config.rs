//! Checkpoint manager configuration

use std::path::PathBuf;

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
