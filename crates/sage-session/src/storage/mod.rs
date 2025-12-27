//! Session storage abstraction and implementations
//!
//! Provides trait-based storage for session persistence with
//! local filesystem implementation.

mod local;

pub use local::LocalSessionStorage;

use crate::{Session, SessionMetadata};
use async_trait::async_trait;
use std::path::PathBuf;
use thiserror::Error;

/// Storage operation errors
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Session not found: {0}")]
    NotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Invalid session data: {0}")]
    InvalidData(String),

    #[error("Storage path not available")]
    PathUnavailable,
}

/// Result type for storage operations
pub type StorageResult<T> = Result<T, StorageError>;

/// Filter criteria for session listing
#[derive(Debug, Default, Clone)]
pub struct SessionFilter {
    /// Filter by project path
    pub project_path: Option<PathBuf>,

    /// Filter by git branch
    pub git_branch: Option<String>,

    /// Maximum number of results
    pub limit: Option<usize>,

    /// Include sidechain sessions
    pub include_sidechains: bool,

    /// Text search in title
    pub title_contains: Option<String>,
}

impl SessionFilter {
    /// Create a new empty filter
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by project path
    pub fn with_project(mut self, path: PathBuf) -> Self {
        self.project_path = Some(path);
        self
    }

    /// Filter by git branch
    pub fn with_branch(mut self, branch: impl Into<String>) -> Self {
        self.git_branch = Some(branch.into());
        self
    }

    /// Limit results
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Include sidechain sessions
    pub fn include_sidechains(mut self) -> Self {
        self.include_sidechains = true;
        self
    }

    /// Search by title
    pub fn with_title(mut self, search: impl Into<String>) -> Self {
        self.title_contains = Some(search.into());
        self
    }

    /// Check if a session matches this filter
    pub fn matches(&self, metadata: &SessionMetadata) -> bool {
        // Check project path
        if let Some(ref path) = self.project_path {
            if metadata.project_path.as_ref() != Some(path) {
                return false;
            }
        }

        // Check git branch
        if let Some(ref branch) = self.git_branch {
            if metadata.git_branch.as_ref() != Some(branch) {
                return false;
            }
        }

        // Check sidechain filter
        if !self.include_sidechains && metadata.is_sidechain {
            return false;
        }

        // Check title search
        if let Some(ref search) = self.title_contains {
            if !metadata.title.to_lowercase().contains(&search.to_lowercase()) {
                return false;
            }
        }

        true
    }
}

/// Session storage trait for different backends
#[async_trait]
pub trait SessionStorage: Send + Sync {
    /// Save a session
    async fn save(&self, session: &Session) -> StorageResult<()>;

    /// Load a session by ID
    async fn load(&self, id: &str) -> StorageResult<Session>;

    /// Delete a session by ID
    async fn delete(&self, id: &str) -> StorageResult<()>;

    /// List session metadata with optional filtering
    async fn list(&self, filter: &SessionFilter) -> StorageResult<Vec<SessionMetadata>>;

    /// Check if a session exists
    async fn exists(&self, id: &str) -> StorageResult<bool>;

    /// Get the most recent session matching the filter
    async fn get_recent(&self, filter: &SessionFilter) -> StorageResult<Option<Session>> {
        let mut filter = filter.clone();
        filter.limit = Some(1);
        let sessions = self.list(&filter).await?;
        if let Some(metadata) = sessions.first() {
            Ok(Some(self.load(&metadata.id).await?))
        } else {
            Ok(None)
        }
    }
}
