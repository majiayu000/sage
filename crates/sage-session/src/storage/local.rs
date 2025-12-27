//! Local filesystem session storage
//!
//! Stores sessions as JSON files in the user's config directory.

use super::{SessionFilter, SessionStorage, StorageError, StorageResult};
use crate::{Session, SessionMetadata};
use async_trait::async_trait;
use std::path::PathBuf;
use tokio::fs;
use tracing::{debug, warn};

/// Local filesystem session storage
///
/// Sessions are stored as JSON files in:
/// - `~/.sage/sessions/` (default)
/// - Custom path if specified
pub struct LocalSessionStorage {
    /// Base directory for session files
    base_path: PathBuf,
}

impl LocalSessionStorage {
    /// Create storage with default path (~/.sage/sessions)
    pub fn new() -> StorageResult<Self> {
        let base_path = dirs::home_dir()
            .ok_or(StorageError::PathUnavailable)?
            .join(".sage")
            .join("sessions");

        Ok(Self { base_path })
    }

    /// Create storage with custom base path
    pub fn with_path(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    /// Ensure storage directory exists
    async fn ensure_dir(&self) -> StorageResult<()> {
        fs::create_dir_all(&self.base_path).await?;
        Ok(())
    }

    /// Get file path for a session ID
    fn session_path(&self, id: &str) -> PathBuf {
        self.base_path.join(format!("{}.json", id))
    }

    /// Read metadata from a session file without loading full content
    async fn read_metadata(&self, path: &PathBuf) -> StorageResult<SessionMetadata> {
        let content = fs::read_to_string(path).await?;
        let session: Session = serde_json::from_str(&content)?;
        Ok(session.metadata)
    }
}

impl Default for LocalSessionStorage {
    fn default() -> Self {
        Self::new().expect("Failed to create default storage")
    }
}

#[async_trait]
impl SessionStorage for LocalSessionStorage {
    async fn save(&self, session: &Session) -> StorageResult<()> {
        self.ensure_dir().await?;

        let path = self.session_path(&session.metadata.id);
        let content = serde_json::to_string_pretty(session)?;

        fs::write(&path, content).await?;
        debug!("Saved session {} to {:?}", session.metadata.id, path);

        Ok(())
    }

    async fn load(&self, id: &str) -> StorageResult<Session> {
        let path = self.session_path(id);

        if !path.exists() {
            return Err(StorageError::NotFound(id.to_string()));
        }

        let content = fs::read_to_string(&path).await?;
        let session: Session = serde_json::from_str(&content)?;

        debug!("Loaded session {} from {:?}", id, path);
        Ok(session)
    }

    async fn delete(&self, id: &str) -> StorageResult<()> {
        let path = self.session_path(id);

        if !path.exists() {
            return Err(StorageError::NotFound(id.to_string()));
        }

        fs::remove_file(&path).await?;
        debug!("Deleted session {} at {:?}", id, path);

        Ok(())
    }

    async fn list(&self, filter: &SessionFilter) -> StorageResult<Vec<SessionMetadata>> {
        self.ensure_dir().await?;

        let mut entries = fs::read_dir(&self.base_path).await?;
        let mut sessions = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            // Only process .json files
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            match self.read_metadata(&path).await {
                Ok(metadata) => {
                    if filter.matches(&metadata) {
                        sessions.push(metadata);
                    }
                }
                Err(e) => {
                    warn!("Failed to read session metadata from {:?}: {}", path, e);
                }
            }
        }

        // Sort by modified time (newest first)
        sessions.sort_by(|a, b| b.modified.cmp(&a.modified));

        // Apply limit
        if let Some(limit) = filter.limit {
            sessions.truncate(limit);
        }

        Ok(sessions)
    }

    async fn exists(&self, id: &str) -> StorageResult<bool> {
        let path = self.session_path(id);
        Ok(path.exists())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_storage() -> (LocalSessionStorage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalSessionStorage::with_path(temp_dir.path().to_path_buf());
        (storage, temp_dir)
    }

    #[tokio::test]
    async fn test_save_and_load() {
        let (storage, _temp) = create_test_storage().await;

        let mut session = Session::new("Test Session");
        session.add_user_message("Hello");
        session.add_assistant_message("Hi there!");

        // Save
        storage.save(&session).await.unwrap();

        // Load
        let loaded = storage.load(session.id()).await.unwrap();
        assert_eq!(loaded.title(), "Test Session");
        assert_eq!(loaded.len(), 2);
    }

    #[tokio::test]
    async fn test_delete() {
        let (storage, _temp) = create_test_storage().await;

        let session = Session::new("To Delete");
        storage.save(&session).await.unwrap();

        assert!(storage.exists(session.id()).await.unwrap());

        storage.delete(session.id()).await.unwrap();

        assert!(!storage.exists(session.id()).await.unwrap());
    }

    #[tokio::test]
    async fn test_list_with_filter() {
        let (storage, _temp) = create_test_storage().await;

        // Create sessions with different branches
        let mut session1 = Session::new("Feature A");
        session1.metadata.git_branch = Some("main".to_string());
        storage.save(&session1).await.unwrap();

        let mut session2 = Session::new("Feature B");
        session2.metadata.git_branch = Some("develop".to_string());
        storage.save(&session2).await.unwrap();

        let mut session3 = Session::new("Feature C");
        session3.metadata.git_branch = Some("main".to_string());
        storage.save(&session3).await.unwrap();

        // Filter by branch
        let filter = SessionFilter::new().with_branch("main");
        let results = storage.list(&filter).await.unwrap();
        assert_eq!(results.len(), 2);

        // Filter with limit
        let filter = SessionFilter::new().with_limit(1);
        let results = storage.list(&filter).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_not_found() {
        let (storage, _temp) = create_test_storage().await;

        let result = storage.load("nonexistent").await;
        assert!(matches!(result, Err(StorageError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_title_search() {
        let (storage, _temp) = create_test_storage().await;

        storage.save(&Session::new("Add user auth")).await.unwrap();
        storage.save(&Session::new("Fix login bug")).await.unwrap();
        storage.save(&Session::new("Update auth flow")).await.unwrap();

        let filter = SessionFilter::new().with_title("auth");
        let results = storage.list(&filter).await.unwrap();
        assert_eq!(results.len(), 2);
    }
}
