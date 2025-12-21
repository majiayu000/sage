//! JSONL storage for enhanced messages
//!
//! This module provides JSONL-based storage for enhanced messages,
//! following the Claude Code pattern of storing one JSON object per line.
//!
//! # File Format
//!
//! Each session is stored as a directory containing:
//! - `messages.jsonl` - One enhanced message per line
//! - `snapshots.jsonl` - File history snapshots
//! - `metadata.json` - Session metadata
//!
//! # Example
//!
//! ```jsonl
//! {"type":"user","uuid":"...","parentUuid":null,...}
//! {"type":"assistant","uuid":"...","parentUuid":"...",...}
//! {"type":"tool_result","uuid":"...","parentUuid":"...",...}
//! ```

use crate::error::{SageError, SageResult};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs::{self, File, OpenOptions};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{debug, error, info, warn};

use super::types::{
    EnhancedMessage, FileHistorySnapshot, SessionContext, SessionId, ThinkingMetadata, TodoItem,
};

/// Session metadata stored in metadata.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    /// Session ID
    pub id: SessionId,

    /// Session name (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Creation timestamp
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,

    /// Working directory
    #[serde(rename = "workingDirectory")]
    pub working_directory: PathBuf,

    /// Git branch at session start
    #[serde(rename = "gitBranch")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,

    /// Model used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Sage Agent version
    pub version: String,

    /// Total message count
    #[serde(rename = "messageCount")]
    pub message_count: usize,

    /// Session state (active, completed, failed)
    pub state: String,

    /// Additional metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl SessionMetadata {
    /// Create new session metadata
    pub fn new(id: impl Into<String>, working_directory: PathBuf) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            name: None,
            created_at: now,
            updated_at: now,
            working_directory,
            git_branch: None,
            model: None,
            version: env!("CARGO_PKG_VERSION").to_string(),
            message_count: 0,
            state: "active".to_string(),
            metadata: HashMap::new(),
        }
    }

    /// Set session name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set git branch
    pub fn with_git_branch(mut self, branch: impl Into<String>) -> Self {
        self.git_branch = Some(branch.into());
        self
    }

    /// Set model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Update message count
    pub fn update_message_count(&mut self, count: usize) {
        self.message_count = count;
        self.updated_at = Utc::now();
    }

    /// Set state
    pub fn set_state(&mut self, state: impl Into<String>) {
        self.state = state.into();
        self.updated_at = Utc::now();
    }
}

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
            .ok_or_else(|| SageError::Config("Could not determine home directory".to_string()))?;
        let base_path = home.join(".sage").join("sessions");
        Ok(Self::new(base_path))
    }

    /// Get the directory path for a session
    fn session_dir(&self, id: &SessionId) -> PathBuf {
        self.base_path.join(id)
    }

    /// Get the messages file path
    fn messages_path(&self, id: &SessionId) -> PathBuf {
        self.session_dir(id).join("messages.jsonl")
    }

    /// Get the snapshots file path
    fn snapshots_path(&self, id: &SessionId) -> PathBuf {
        self.session_dir(id).join("snapshots.jsonl")
    }

    /// Get the metadata file path
    fn metadata_path(&self, id: &SessionId) -> PathBuf {
        self.session_dir(id).join("metadata.json")
    }

    /// Ensure the session directory exists
    async fn ensure_session_dir(&self, id: &SessionId) -> SageResult<()> {
        let dir = self.session_dir(id);
        if !dir.exists() {
            fs::create_dir_all(&dir).await.map_err(|e| {
                SageError::Io(format!("Failed to create session directory: {}", e))
            })?;
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

    /// Save session metadata
    pub async fn save_metadata(
        &self,
        id: &SessionId,
        metadata: &SessionMetadata,
    ) -> SageResult<()> {
        let path = self.metadata_path(id);
        let json = serde_json::to_string_pretty(metadata)
            .map_err(|e| SageError::Json(format!("Failed to serialize metadata: {}", e)))?;

        fs::write(&path, json)
            .await
            .map_err(|e| SageError::Io(format!("Failed to write metadata file: {}", e)))?;

        debug!("Saved metadata for session {}", id);
        Ok(())
    }

    /// Load session metadata
    pub async fn load_metadata(&self, id: &SessionId) -> SageResult<Option<SessionMetadata>> {
        let path = self.metadata_path(id);

        if !path.exists() {
            return Ok(None);
        }

        let json = fs::read_to_string(&path)
            .await
            .map_err(|e| SageError::Io(format!("Failed to read metadata file: {}", e)))?;

        let metadata: SessionMetadata = serde_json::from_str(&json)
            .map_err(|e| SageError::Json(format!("Failed to deserialize metadata: {}", e)))?;

        Ok(Some(metadata))
    }

    /// Append a message to the session
    pub async fn append_message(
        &self,
        id: &SessionId,
        message: &EnhancedMessage,
    ) -> SageResult<()> {
        self.ensure_session_dir(id).await?;

        let path = self.messages_path(id);
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await
            .map_err(|e| SageError::Io(format!("Failed to open messages file: {}", e)))?;

        let json = serde_json::to_string(message)
            .map_err(|e| SageError::Json(format!("Failed to serialize message: {}", e)))?;

        file.write_all(json.as_bytes())
            .await
            .map_err(|e| SageError::Io(format!("Failed to write message: {}", e)))?;
        file.write_all(b"\n")
            .await
            .map_err(|e| SageError::Io(format!("Failed to write newline: {}", e)))?;

        debug!("Appended message {} to session {}", message.uuid, id);
        Ok(())
    }

    /// Append a file history snapshot
    pub async fn append_snapshot(
        &self,
        id: &SessionId,
        snapshot: &FileHistorySnapshot,
    ) -> SageResult<()> {
        self.ensure_session_dir(id).await?;

        let path = self.snapshots_path(id);
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await
            .map_err(|e| SageError::Io(format!("Failed to open snapshots file: {}", e)))?;

        let json = serde_json::to_string(snapshot)
            .map_err(|e| SageError::Json(format!("Failed to serialize snapshot: {}", e)))?;

        file.write_all(json.as_bytes())
            .await
            .map_err(|e| SageError::Io(format!("Failed to write snapshot: {}", e)))?;
        file.write_all(b"\n")
            .await
            .map_err(|e| SageError::Io(format!("Failed to write newline: {}", e)))?;

        debug!("Appended snapshot for message {} to session {}", snapshot.message_id, id);
        Ok(())
    }

    /// Load all messages from a session
    pub async fn load_messages(&self, id: &SessionId) -> SageResult<Vec<EnhancedMessage>> {
        let path = self.messages_path(id);

        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&path)
            .await
            .map_err(|e| SageError::Io(format!("Failed to open messages file: {}", e)))?;

        let reader = BufReader::new(file);
        let mut lines = reader.lines();
        let mut messages = Vec::new();

        while let Some(line) = lines
            .next_line()
            .await
            .map_err(|e| SageError::Io(format!("Failed to read line: {}", e)))?
        {
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<EnhancedMessage>(&line) {
                Ok(msg) => messages.push(msg),
                Err(e) => {
                    warn!("Failed to parse message: {} - line: {}", e, &line[..50.min(line.len())]);
                }
            }
        }

        debug!("Loaded {} messages from session {}", messages.len(), id);
        Ok(messages)
    }

    /// Load all snapshots from a session
    pub async fn load_snapshots(&self, id: &SessionId) -> SageResult<Vec<FileHistorySnapshot>> {
        let path = self.snapshots_path(id);

        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&path)
            .await
            .map_err(|e| SageError::Io(format!("Failed to open snapshots file: {}", e)))?;

        let reader = BufReader::new(file);
        let mut lines = reader.lines();
        let mut snapshots = Vec::new();

        while let Some(line) = lines
            .next_line()
            .await
            .map_err(|e| SageError::Io(format!("Failed to read line: {}", e)))?
        {
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<FileHistorySnapshot>(&line) {
                Ok(snapshot) => snapshots.push(snapshot),
                Err(e) => {
                    warn!("Failed to parse snapshot: {} - line: {}", e, &line[..50.min(line.len())]);
                }
            }
        }

        debug!("Loaded {} snapshots from session {}", snapshots.len(), id);
        Ok(snapshots)
    }

    /// Delete a session
    pub async fn delete_session(&self, id: &SessionId) -> SageResult<()> {
        let dir = self.session_dir(id);

        if dir.exists() {
            fs::remove_dir_all(&dir).await.map_err(|e| {
                SageError::Io(format!("Failed to delete session directory: {}", e))
            })?;
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
        let mut entries = fs::read_dir(&self.base_path).await.map_err(|e| {
            SageError::Io(format!("Failed to read sessions directory: {}", e))
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            SageError::Io(format!("Failed to read directory entry: {}", e))
        })? {
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

    /// Get message by UUID
    pub async fn get_message(
        &self,
        session_id: &SessionId,
        message_uuid: &str,
    ) -> SageResult<Option<EnhancedMessage>> {
        let messages = self.load_messages(session_id).await?;
        Ok(messages.into_iter().find(|m| m.uuid == message_uuid))
    }

    /// Get messages up to a specific UUID (for undo)
    pub async fn get_messages_until(
        &self,
        session_id: &SessionId,
        message_uuid: &str,
    ) -> SageResult<Vec<EnhancedMessage>> {
        let messages = self.load_messages(session_id).await?;
        let mut result = Vec::new();

        for msg in messages {
            result.push(msg.clone());
            if msg.uuid == message_uuid {
                break;
            }
        }

        Ok(result)
    }

    /// Get the message chain (following parentUuid links)
    pub async fn get_message_chain(
        &self,
        session_id: &SessionId,
        start_uuid: &str,
    ) -> SageResult<Vec<EnhancedMessage>> {
        let messages = self.load_messages(session_id).await?;

        // Build a map for quick lookup
        let msg_map: HashMap<&str, &EnhancedMessage> =
            messages.iter().map(|m| (m.uuid.as_str(), m)).collect();

        // Follow the chain from start
        let mut chain = Vec::new();
        let mut current_uuid = Some(start_uuid);

        while let Some(uuid) = current_uuid {
            if let Some(msg) = msg_map.get(uuid) {
                chain.push((*msg).clone());
                current_uuid = msg.parent_uuid.as_deref();
            } else {
                break;
            }
        }

        // Reverse to get chronological order
        chain.reverse();
        Ok(chain)
    }
}

/// Message chain tracker for building parent-child relationships
#[derive(Debug, Default)]
pub struct MessageChainTracker {
    /// Last message UUID
    last_uuid: Option<String>,

    /// Current session ID
    session_id: Option<String>,

    /// Current context
    context: Option<SessionContext>,

    /// Current todos
    todos: Vec<TodoItem>,

    /// Current thinking metadata
    thinking: Option<ThinkingMetadata>,
}

impl MessageChainTracker {
    /// Create a new tracker
    pub fn new() -> Self {
        Self::default()
    }

    /// Set session ID
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Set context
    pub fn with_context(mut self, context: SessionContext) -> Self {
        self.context = Some(context);
        self
    }

    /// Update last message UUID
    pub fn set_last_uuid(&mut self, uuid: impl Into<String>) {
        self.last_uuid = Some(uuid.into());
    }

    /// Get parent UUID for next message
    pub fn parent_uuid(&self) -> Option<String> {
        self.last_uuid.clone()
    }

    /// Update todos
    pub fn set_todos(&mut self, todos: Vec<TodoItem>) {
        self.todos = todos;
    }

    /// Update thinking metadata
    pub fn set_thinking(&mut self, thinking: ThinkingMetadata) {
        self.thinking = Some(thinking);
    }

    /// Create a user message
    pub fn create_user_message(&mut self, content: impl Into<String>) -> EnhancedMessage {
        let session_id = self.session_id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let context = self.context.clone().unwrap_or_else(|| {
            SessionContext::new(std::env::current_dir().unwrap_or_default())
        });

        let mut msg = EnhancedMessage::user(content, &session_id, context)
            .with_todos(self.todos.clone());

        if let Some(parent) = &self.last_uuid {
            msg = msg.with_parent(parent);
        }

        if let Some(thinking) = &self.thinking {
            msg = msg.with_thinking(thinking.clone());
        }

        self.last_uuid = Some(msg.uuid.clone());
        msg
    }

    /// Create an assistant message
    pub fn create_assistant_message(&mut self, content: impl Into<String>) -> EnhancedMessage {
        let session_id = self.session_id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let context = self.context.clone().unwrap_or_else(|| {
            SessionContext::new(std::env::current_dir().unwrap_or_default())
        });

        let mut msg = EnhancedMessage::assistant(content, &session_id, context, self.last_uuid.clone())
            .with_todos(self.todos.clone());

        if let Some(thinking) = &self.thinking {
            msg = msg.with_thinking(thinking.clone());
        }

        self.last_uuid = Some(msg.uuid.clone());
        msg
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_create_session() {
        let tmp = TempDir::new().unwrap();
        let storage = JsonlSessionStorage::new(tmp.path());

        let metadata = storage
            .create_session("test-session", PathBuf::from("/tmp"))
            .await
            .unwrap();

        assert_eq!(metadata.id, "test-session");
        assert!(storage.session_exists(&"test-session".to_string()).await);
    }

    #[tokio::test]
    async fn test_append_and_load_messages() {
        let tmp = TempDir::new().unwrap();
        let storage = JsonlSessionStorage::new(tmp.path());
        let session_id = "test-session".to_string();

        storage
            .create_session(&session_id, PathBuf::from("/tmp"))
            .await
            .unwrap();

        let context = SessionContext::new(PathBuf::from("/tmp"));
        let msg1 = EnhancedMessage::user("Hello", &session_id, context.clone());
        let msg2 = EnhancedMessage::assistant("Hi!", &session_id, context, Some(msg1.uuid.clone()));

        storage.append_message(&session_id, &msg1).await.unwrap();
        storage.append_message(&session_id, &msg2).await.unwrap();

        let messages = storage.load_messages(&session_id).await.unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].uuid, msg1.uuid);
        assert_eq!(messages[1].uuid, msg2.uuid);
    }

    #[tokio::test]
    async fn test_message_chain() {
        let tmp = TempDir::new().unwrap();
        let storage = JsonlSessionStorage::new(tmp.path());
        let session_id = "test-session".to_string();

        storage
            .create_session(&session_id, PathBuf::from("/tmp"))
            .await
            .unwrap();

        let context = SessionContext::new(PathBuf::from("/tmp"));
        let msg1 = EnhancedMessage::user("First", &session_id, context.clone());
        let msg2 = EnhancedMessage::assistant("Second", &session_id, context.clone(), Some(msg1.uuid.clone()));
        let msg3 = EnhancedMessage::user("Third", &session_id, context)
            .with_parent(&msg2.uuid);

        storage.append_message(&session_id, &msg1).await.unwrap();
        storage.append_message(&session_id, &msg2).await.unwrap();
        storage.append_message(&session_id, &msg3).await.unwrap();

        let chain = storage.get_message_chain(&session_id, &msg3.uuid).await.unwrap();
        assert_eq!(chain.len(), 3);
        assert_eq!(chain[0].uuid, msg1.uuid);
        assert_eq!(chain[1].uuid, msg2.uuid);
        assert_eq!(chain[2].uuid, msg3.uuid);
    }

    #[tokio::test]
    async fn test_message_chain_tracker() {
        let context = SessionContext::new(PathBuf::from("/tmp"));
        let mut tracker = MessageChainTracker::new()
            .with_session("test")
            .with_context(context);

        let msg1 = tracker.create_user_message("Hello");
        assert!(msg1.parent_uuid.is_none());

        let msg2 = tracker.create_assistant_message("Hi!");
        assert_eq!(msg2.parent_uuid, Some(msg1.uuid.clone()));

        let msg3 = tracker.create_user_message("How are you?");
        assert_eq!(msg3.parent_uuid, Some(msg2.uuid.clone()));
    }

    #[tokio::test]
    async fn test_delete_session() {
        let tmp = TempDir::new().unwrap();
        let storage = JsonlSessionStorage::new(tmp.path());
        let session_id = "test-session".to_string();

        storage
            .create_session(&session_id, PathBuf::from("/tmp"))
            .await
            .unwrap();

        assert!(storage.session_exists(&session_id).await);

        storage.delete_session(&session_id).await.unwrap();
        assert!(!storage.session_exists(&session_id).await);
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let tmp = TempDir::new().unwrap();
        let storage = JsonlSessionStorage::new(tmp.path());

        storage
            .create_session("session-1", PathBuf::from("/tmp/1"))
            .await
            .unwrap();
        storage
            .create_session("session-2", PathBuf::from("/tmp/2"))
            .await
            .unwrap();

        let sessions = storage.list_sessions().await.unwrap();
        assert_eq!(sessions.len(), 2);
    }
}
