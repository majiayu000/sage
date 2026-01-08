//! Core storage struct and session management

use crate::error::{SageError, SageResult};
use std::path::PathBuf;
use tokio::fs;
use tracing::{error, info, warn};

use super::super::super::types::{SessionContext, SessionId};
use super::super::metadata::SessionMetadata;

/// JSONL session storage
///
/// Stores sessions in a directory structure:
/// ```text
/// .sage/sessions/
///   session-123/
///     messages.jsonl
///     snapshots.jsonl
///     metadata.json
/// ```
pub struct JsonlSessionStorage {
    /// Base directory for storing sessions
    base_path: PathBuf,
}

impl JsonlSessionStorage {
    /// Create a new JSONL session storage
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
        }
    }

    /// Create storage with default path (~/.sage/sessions)
    pub fn default_path() -> SageResult<Self> {
        let home = dirs::home_dir()
            .ok_or_else(|| SageError::config("Could not determine home directory".to_string()))?;
        let base_path = home.join(".sage").join("sessions");
        Ok(Self::new(base_path))
    }

    /// Get the directory path for a session
    pub(super) fn session_dir(&self, id: &SessionId) -> PathBuf {
        self.base_path.join(id)
    }

    /// Get the messages file path
    pub(super) fn messages_path(&self, id: &SessionId) -> PathBuf {
        self.session_dir(id).join("messages.jsonl")
    }

    /// Get the snapshots file path
    pub(super) fn snapshots_path(&self, id: &SessionId) -> PathBuf {
        self.session_dir(id).join("snapshots.jsonl")
    }

    /// Get the metadata file path
    pub(super) fn metadata_path(&self, id: &SessionId) -> PathBuf {
        self.session_dir(id).join("metadata.json")
    }

    /// Ensure the session directory exists
    pub(super) async fn ensure_session_dir(&self, id: &SessionId) -> SageResult<()> {
        let dir = self.session_dir(id);
        if !dir.exists() {
            fs::create_dir_all(&dir)
                .await
                .map_err(|e| SageError::io(format!("Failed to create session directory: {}", e)))?;
        }
        Ok(())
    }

    /// Initialize a new session
    pub async fn create_session(
        &self,
        id: impl Into<String>,
        working_directory: PathBuf,
    ) -> SageResult<SessionMetadata> {
        let id = id.into();
        self.ensure_session_dir(&id).await?;

        let mut metadata = SessionMetadata::new(&id, working_directory.clone());

        // Detect git branch
        let mut context = SessionContext::new(working_directory);
        context.detect_git_branch();
        if let Some(branch) = context.git_branch {
            metadata = metadata.with_git_branch(branch);
        }

        // Save initial metadata
        self.save_metadata(&id, &metadata).await?;

        info!("Created new session: {}", id);
        Ok(metadata)
    }

    /// Initialize a new sidechain session (branched from a parent session)
    ///
    /// Creates a new session and marks it as a sidechain of the parent session.
    /// This is used for conversation branching (Claude Code style).
    pub async fn create_sidechain_session(
        &self,
        id: impl Into<String>,
        parent_session_id: impl Into<String>,
        working_directory: PathBuf,
    ) -> SageResult<SessionMetadata> {
        let id = id.into();
        let parent_id = parent_session_id.into();
        self.ensure_session_dir(&id).await?;

        let mut metadata = SessionMetadata::new(&id, working_directory.clone());

        // Detect git branch
        let mut context = SessionContext::new(working_directory);
        context.detect_git_branch();
        if let Some(branch) = context.git_branch {
            metadata = metadata.with_git_branch(branch);
        }

        // Mark as sidechain with parent reference
        metadata = metadata.as_sidechain(&parent_id);

        // Save initial metadata
        self.save_metadata(&id, &metadata).await?;

        info!("Created sidechain session: {} (parent: {})", id, parent_id);
        Ok(metadata)
    }

    /// Delete a session
    pub async fn delete_session(&self, id: &SessionId) -> SageResult<()> {
        let dir = self.session_dir(id);

        if dir.exists() {
            fs::remove_dir_all(&dir)
                .await
                .map_err(|e| SageError::io(format!("Failed to delete session directory: {}", e)))?;
            info!("Deleted session {}", id);
        } else {
            warn!("Session {} not found", id);
        }

        Ok(())
    }

    /// List all sessions
    pub async fn list_sessions(&self) -> SageResult<Vec<SessionMetadata>> {
        if !self.base_path.exists() {
            return Ok(Vec::new());
        }

        let mut sessions = Vec::new();
        let mut entries = fs::read_dir(&self.base_path)
            .await
            .map_err(|e| SageError::io(format!("Failed to read sessions directory: {}", e)))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| SageError::io(format!("Failed to read directory entry: {}", e)))?
        {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name() {
                    let id = name.to_string_lossy().to_string();
                    match self.load_metadata(&id).await {
                        Ok(Some(metadata)) => sessions.push(metadata),
                        Ok(None) => {
                            warn!("Session directory exists but no metadata: {}", id);
                        }
                        Err(e) => {
                            error!("Failed to load session metadata for {}: {}", id, e);
                        }
                    }
                }
            }
        }

        // Sort by updated_at descending
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        Ok(sessions)
    }

    /// Check if a session exists
    pub async fn session_exists(&self, id: &SessionId) -> bool {
        self.metadata_path(id).exists()
    }
}
