//! Session storage backends
//!
//! This module provides storage backends for persisting sessions,
//! including file-based and memory-based implementations.

use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use super::types::{Session, SessionId, SessionSummary};
use crate::error::{SageError, SageResult};

/// Session storage trait
#[async_trait]
pub trait SessionStorage: Send + Sync {
    /// Save a session
    async fn save(&self, session: &Session) -> SageResult<()>;

    /// Load a session by ID
    async fn load(&self, id: &SessionId) -> SageResult<Option<Session>>;

    /// Delete a session
    async fn delete(&self, id: &SessionId) -> SageResult<()>;

    /// List all sessions (summaries)
    async fn list(&self) -> SageResult<Vec<SessionSummary>>;

    /// Check if a session exists
    async fn exists(&self, id: &SessionId) -> SageResult<bool>;
}

/// File-based session storage
pub struct FileSessionStorage {
    /// Base directory for storing sessions
    base_path: PathBuf,
}

impl FileSessionStorage {
    /// Create a new file-based session storage
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
        }
    }

    /// Create storage with default path (~/.config/sage/sessions)
    pub fn default_path() -> SageResult<Self> {
        let home = dirs::home_dir().ok_or_else(|| {
            SageError::Config("Could not determine home directory".to_string())
        })?;
        let base_path = home.join(".config").join("sage").join("sessions");
        Ok(Self::new(base_path))
    }

    /// Get the file path for a session
    fn session_path(&self, id: &SessionId) -> PathBuf {
        self.base_path.join(format!("{}.json", id))
    }

    /// Ensure the storage directory exists
    async fn ensure_dir(&self) -> SageResult<()> {
        if !self.base_path.exists() {
            fs::create_dir_all(&self.base_path).await.map_err(|e| {
                SageError::Io(format!("Failed to create session directory: {}", e))
            })?;
        }
        Ok(())
    }
}

#[async_trait]
impl SessionStorage for FileSessionStorage {
    async fn save(&self, session: &Session) -> SageResult<()> {
        self.ensure_dir().await?;

        let path = self.session_path(&session.id);
        let json = serde_json::to_string_pretty(session).map_err(|e| {
            SageError::Json(format!("Failed to serialize session: {}", e))
        })?;

        fs::write(&path, json).await.map_err(|e| {
            SageError::Io(format!("Failed to write session file: {}", e))
        })?;

        debug!("Saved session {} to {:?}", session.id, path);
        Ok(())
    }

    async fn load(&self, id: &SessionId) -> SageResult<Option<Session>> {
        let path = self.session_path(id);

        if !path.exists() {
            return Ok(None);
        }

        let json = fs::read_to_string(&path).await.map_err(|e| {
            SageError::Io(format!("Failed to read session file: {}", e))
        })?;

        let session: Session = serde_json::from_str(&json).map_err(|e| {
            SageError::Json(format!("Failed to deserialize session: {}", e))
        })?;

        debug!("Loaded session {} from {:?}", id, path);
        Ok(Some(session))
    }

    async fn delete(&self, id: &SessionId) -> SageResult<()> {
        let path = self.session_path(id);

        if path.exists() {
            fs::remove_file(&path).await.map_err(|e| {
                SageError::Io(format!("Failed to delete session file: {}", e))
            })?;
            info!("Deleted session {} from {:?}", id, path);
        } else {
            warn!("Session {} not found at {:?}", id, path);
        }

        Ok(())
    }

    async fn list(&self) -> SageResult<Vec<SessionSummary>> {
        self.ensure_dir().await?;

        let mut summaries = Vec::new();
        let mut entries = fs::read_dir(&self.base_path).await.map_err(|e| {
            SageError::Io(format!("Failed to read session directory: {}", e))
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            SageError::Io(format!("Failed to read directory entry: {}", e))
        })? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                if let Some(stem) = path.file_stem() {
                    let id = stem.to_string_lossy().to_string();
                    match self.load(&id).await {
                        Ok(Some(session)) => {
                            summaries.push(SessionSummary::from(&session));
                        }
                        Ok(None) => {
                            warn!("Session file exists but could not be loaded: {:?}", path);
                        }
                        Err(e) => {
                            error!("Failed to load session from {:?}: {}", path, e);
                        }
                    }
                }
            }
        }

        // Sort by updated_at descending (most recent first)
        summaries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        Ok(summaries)
    }

    async fn exists(&self, id: &SessionId) -> SageResult<bool> {
        Ok(self.session_path(id).exists())
    }
}

/// In-memory session storage (for testing or temporary sessions)
#[derive(Debug, Default)]
pub struct MemorySessionStorage {
    sessions: Arc<RwLock<HashMap<SessionId, Session>>>,
}

impl MemorySessionStorage {
    /// Create a new in-memory session storage
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl SessionStorage for MemorySessionStorage {
    async fn save(&self, session: &Session) -> SageResult<()> {
        self.sessions
            .write()
            .await
            .insert(session.id.clone(), session.clone());
        Ok(())
    }

    async fn load(&self, id: &SessionId) -> SageResult<Option<Session>> {
        Ok(self.sessions.read().await.get(id).cloned())
    }

    async fn delete(&self, id: &SessionId) -> SageResult<()> {
        self.sessions.write().await.remove(id);
        Ok(())
    }

    async fn list(&self) -> SageResult<Vec<SessionSummary>> {
        let sessions = self.sessions.read().await;
        let mut summaries: Vec<SessionSummary> = sessions
            .values()
            .map(SessionSummary::from)
            .collect();

        // Sort by updated_at descending
        summaries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        Ok(summaries)
    }

    async fn exists(&self, id: &SessionId) -> SageResult<bool> {
        Ok(self.sessions.read().await.contains_key(id))
    }
}

/// Boxed session storage type
pub type BoxedSessionStorage = Box<dyn SessionStorage>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_memory_storage_save_load() {
        let storage = MemorySessionStorage::new();
        let session = Session::new(PathBuf::from("/tmp"));

        storage.save(&session).await.unwrap();
        let loaded = storage.load(&session.id).await.unwrap();

        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().id, session.id);
    }

    #[tokio::test]
    async fn test_memory_storage_delete() {
        let storage = MemorySessionStorage::new();
        let session = Session::new(PathBuf::from("/tmp"));

        storage.save(&session).await.unwrap();
        assert!(storage.exists(&session.id).await.unwrap());

        storage.delete(&session.id).await.unwrap();
        assert!(!storage.exists(&session.id).await.unwrap());
    }

    #[tokio::test]
    async fn test_memory_storage_list() {
        let storage = MemorySessionStorage::new();

        let session1 = Session::new(PathBuf::from("/tmp/1"));
        let session2 = Session::new(PathBuf::from("/tmp/2"));

        storage.save(&session1).await.unwrap();
        storage.save(&session2).await.unwrap();

        let list = storage.list().await.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_memory_storage_exists() {
        let storage = MemorySessionStorage::new();
        let session = Session::new(PathBuf::from("/tmp"));

        assert!(!storage.exists(&session.id).await.unwrap());

        storage.save(&session).await.unwrap();
        assert!(storage.exists(&session.id).await.unwrap());
    }

    #[tokio::test]
    async fn test_memory_storage_load_nonexistent() {
        let storage = MemorySessionStorage::new();
        let loaded = storage.load(&"nonexistent".to_string()).await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_file_storage_session_path() {
        let storage = FileSessionStorage::new("/tmp/test_sessions");
        let path = storage.session_path(&"test-id".to_string());
        assert_eq!(path.to_string_lossy(), "/tmp/test_sessions/test-id.json");
    }
}
