//! Session manager for orchestrating session lifecycle
//!
//! This module provides the SessionManager which handles creating,
//! resuming, saving, and listing sessions.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::storage::{BoxedSessionStorage, MemorySessionStorage};
use super::types::{
    ConversationMessage, Session, SessionConfig, SessionId, SessionState, SessionSummary,
};
use crate::error::{SageError, SageResult};

/// Session manager for handling session lifecycle
pub struct SessionManager {
    /// Storage backend
    storage: BoxedSessionStorage,
    /// Currently active sessions (cached)
    active_sessions: Arc<RwLock<HashMap<SessionId, Session>>>,
    /// Default working directory
    default_working_dir: PathBuf,
}

impl SessionManager {
    /// Create a new session manager with the given storage
    pub fn new(storage: BoxedSessionStorage) -> Self {
        Self {
            storage,
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            default_working_dir: std::env::current_dir().unwrap_or_default(),
        }
    }

    /// Create a session manager with in-memory storage (for testing)
    pub fn in_memory() -> Self {
        Self::new(Box::new(MemorySessionStorage::new()))
    }

    /// Set the default working directory
    pub fn with_default_working_dir(mut self, dir: PathBuf) -> Self {
        self.default_working_dir = dir;
        self
    }

    /// Create a new session
    pub async fn create(&self, config: SessionConfig) -> SageResult<SessionId> {
        let working_dir = config
            .working_directory
            .unwrap_or_else(|| self.default_working_dir.clone());

        let mut session = Session::new(working_dir);

        if let Some(name) = config.name {
            session.name = Some(name);
        }

        if let Some(model) = config.model {
            session.model = Some(model);
        }

        // Add system prompt if provided
        if let Some(system_prompt) = config.system_prompt {
            session.add_message(ConversationMessage::system(system_prompt));
        }

        // Add metadata
        for (key, value) in config.metadata {
            session.metadata.insert(key, value);
        }

        let id = session.id.clone();

        // Save to storage
        self.storage.save(&session).await?;

        // Cache the active session
        self.active_sessions.write().await.insert(id.clone(), session);

        info!("Created new session: {}", id);
        Ok(id)
    }

    /// Resume an existing session
    pub async fn resume(&self, id: &SessionId) -> SageResult<Session> {
        // Check cache first
        {
            let cache = self.active_sessions.read().await;
            if let Some(session) = cache.get(id) {
                debug!("Resumed session {} from cache", id);
                let mut session = session.clone();
                session.resume();
                return Ok(session);
            }
        }

        // Load from storage
        let session = self.storage.load(id).await?.ok_or_else(|| {
            SageError::InvalidInput(format!("Session not found: {}", id))
        })?;

        // Update state if paused
        let mut session = session;
        if session.state == SessionState::Paused {
            session.resume();
        }

        // Cache the session
        self.active_sessions
            .write()
            .await
            .insert(id.clone(), session.clone());

        info!("Resumed session {} from storage", id);
        Ok(session)
    }

    /// Save a session
    pub async fn save(&self, session: &Session) -> SageResult<()> {
        // Update cache
        self.active_sessions
            .write()
            .await
            .insert(session.id.clone(), session.clone());

        // Persist to storage
        self.storage.save(session).await?;

        debug!("Saved session {}", session.id);
        Ok(())
    }

    /// Get a session by ID (from cache or storage)
    pub async fn get(&self, id: &SessionId) -> SageResult<Option<Session>> {
        // Check cache first
        {
            let cache = self.active_sessions.read().await;
            if let Some(session) = cache.get(id) {
                return Ok(Some(session.clone()));
            }
        }

        // Load from storage
        self.storage.load(id).await
    }

    /// List all sessions
    pub async fn list(&self) -> SageResult<Vec<SessionSummary>> {
        self.storage.list().await
    }

    /// List only active sessions
    pub async fn list_active(&self) -> SageResult<Vec<SessionSummary>> {
        let all = self.storage.list().await?;
        Ok(all
            .into_iter()
            .filter(|s| s.state == SessionState::Active || s.state == SessionState::Paused)
            .collect())
    }

    /// Delete a session
    pub async fn delete(&self, id: &SessionId) -> SageResult<()> {
        // Remove from cache
        self.active_sessions.write().await.remove(id);

        // Remove from storage
        self.storage.delete(id).await?;

        info!("Deleted session {}", id);
        Ok(())
    }

    /// Mark a session as completed
    pub async fn complete(&self, id: &SessionId) -> SageResult<()> {
        let mut session = self.get(id).await?.ok_or_else(|| {
            SageError::InvalidInput(format!("Session not found: {}", id))
        })?;

        session.complete();
        self.save(&session).await?;

        // Remove from active cache
        self.active_sessions.write().await.remove(id);

        info!("Completed session {}", id);
        Ok(())
    }

    /// Mark a session as failed
    pub async fn fail(&self, id: &SessionId, error: impl Into<String>) -> SageResult<()> {
        let mut session = self.get(id).await?.ok_or_else(|| {
            SageError::InvalidInput(format!("Session not found: {}", id))
        })?;

        session.fail(error);
        self.save(&session).await?;

        // Remove from active cache
        self.active_sessions.write().await.remove(id);

        warn!("Session {} failed", id);
        Ok(())
    }

    /// Pause a session (keep in storage but mark as paused)
    pub async fn pause(&self, id: &SessionId) -> SageResult<()> {
        let mut session = self.get(id).await?.ok_or_else(|| {
            SageError::InvalidInput(format!("Session not found: {}", id))
        })?;

        session.pause();
        self.save(&session).await?;

        // Remove from active cache
        self.active_sessions.write().await.remove(id);

        info!("Paused session {}", id);
        Ok(())
    }

    /// Add a message to a session
    pub async fn add_message(&self, id: &SessionId, message: ConversationMessage) -> SageResult<()> {
        let mut session = self.get(id).await?.ok_or_else(|| {
            SageError::InvalidInput(format!("Session not found: {}", id))
        })?;

        session.add_message(message);
        self.save(&session).await?;

        debug!("Added message to session {}", id);
        Ok(())
    }

    /// Check if a session exists
    pub async fn exists(&self, id: &SessionId) -> SageResult<bool> {
        // Check cache
        if self.active_sessions.read().await.contains_key(id) {
            return Ok(true);
        }

        // Check storage
        self.storage.exists(id).await
    }

    /// Get the count of active sessions
    pub async fn active_count(&self) -> usize {
        self.active_sessions.read().await.len()
    }

    /// Clear all active sessions from cache
    pub async fn clear_cache(&self) {
        self.active_sessions.write().await.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_session() {
        let manager = SessionManager::in_memory();
        let config = SessionConfig::new().with_name("Test Session");

        let id = manager.create(config).await.unwrap();
        assert!(!id.is_empty());

        let session = manager.get(&id).await.unwrap().unwrap();
        assert_eq!(session.name, Some("Test Session".to_string()));
    }

    #[tokio::test]
    async fn test_create_session_with_system_prompt() {
        let manager = SessionManager::in_memory();
        let config = SessionConfig::new()
            .with_system_prompt("You are a helpful assistant");

        let id = manager.create(config).await.unwrap();
        let session = manager.get(&id).await.unwrap().unwrap();

        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.messages[0].role, super::super::types::MessageRole::System);
    }

    #[tokio::test]
    async fn test_resume_session() {
        let manager = SessionManager::in_memory();
        let config = SessionConfig::new();

        let id = manager.create(config).await.unwrap();

        // Pause the session
        manager.pause(&id).await.unwrap();

        // Resume it
        let session = manager.resume(&id).await.unwrap();
        assert_eq!(session.state, SessionState::Active);
    }

    #[tokio::test]
    async fn test_delete_session() {
        let manager = SessionManager::in_memory();
        let config = SessionConfig::new();

        let id = manager.create(config).await.unwrap();
        assert!(manager.exists(&id).await.unwrap());

        manager.delete(&id).await.unwrap();
        assert!(!manager.exists(&id).await.unwrap());
    }

    #[tokio::test]
    async fn test_complete_session() {
        let manager = SessionManager::in_memory();
        let config = SessionConfig::new();

        let id = manager.create(config).await.unwrap();
        manager.complete(&id).await.unwrap();

        let session = manager.get(&id).await.unwrap().unwrap();
        assert_eq!(session.state, SessionState::Completed);
    }

    #[tokio::test]
    async fn test_fail_session() {
        let manager = SessionManager::in_memory();
        let config = SessionConfig::new();

        let id = manager.create(config).await.unwrap();
        manager.fail(&id, "Something went wrong").await.unwrap();

        let session = manager.get(&id).await.unwrap().unwrap();
        assert_eq!(session.state, SessionState::Failed);
        assert_eq!(session.error, Some("Something went wrong".to_string()));
    }

    #[tokio::test]
    async fn test_add_message() {
        let manager = SessionManager::in_memory();
        let config = SessionConfig::new();

        let id = manager.create(config).await.unwrap();
        manager.add_message(&id, ConversationMessage::user("Hello")).await.unwrap();

        let session = manager.get(&id).await.unwrap().unwrap();
        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.messages[0].content, "Hello");
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let manager = SessionManager::in_memory();

        manager.create(SessionConfig::new().with_name("Session 1")).await.unwrap();
        manager.create(SessionConfig::new().with_name("Session 2")).await.unwrap();

        let list = manager.list().await.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_list_active_sessions() {
        let manager = SessionManager::in_memory();

        let id1 = manager.create(SessionConfig::new().with_name("Active")).await.unwrap();
        let id2 = manager.create(SessionConfig::new().with_name("Completed")).await.unwrap();

        manager.complete(&id2).await.unwrap();

        let active = manager.list_active().await.unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, id1);
    }

    #[tokio::test]
    async fn test_active_count() {
        let manager = SessionManager::in_memory();

        manager.create(SessionConfig::new()).await.unwrap();
        manager.create(SessionConfig::new()).await.unwrap();

        assert_eq!(manager.active_count().await, 2);
    }

    #[tokio::test]
    async fn test_clear_cache() {
        let manager = SessionManager::in_memory();

        manager.create(SessionConfig::new()).await.unwrap();
        assert_eq!(manager.active_count().await, 1);

        manager.clear_cache().await;
        assert_eq!(manager.active_count().await, 0);
    }

    #[tokio::test]
    async fn test_get_nonexistent() {
        let manager = SessionManager::in_memory();
        let result = manager.get(&"nonexistent".to_string()).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_resume_nonexistent() {
        let manager = SessionManager::in_memory();
        let result = manager.resume(&"nonexistent".to_string()).await;
        assert!(result.is_err());
    }
}
