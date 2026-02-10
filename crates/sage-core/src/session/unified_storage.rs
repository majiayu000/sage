//! Unified JSONL Session Storage
//!
//! This module provides the primary storage backend using the unified data model.
//! It replaces FileSessionStorage and uses SessionHeader/SessionMessage/SessionRecord.

use chrono::Utc;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::fs::{self, File, OpenOptions};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{debug, error, info, warn};

use super::types::unified::{
    FileHistorySnapshot, MessageId, Session, SessionContext, SessionHeader,
    SessionId, SessionMessage, SessionMessageType, SessionMetadataPatch, SessionRecord,
    SessionRecordPayload,
};
use crate::error::{SageError, SageResult};

/// Unified JSONL session storage
///
/// Stores sessions in a directory structure:
/// ```text
/// ~/.sage/projects/{escaped-cwd}/
///   {session-id}/
///     metadata.json    - SessionHeader
///     records.jsonl    - SessionRecord (messages, snapshots, patches)
/// ```
pub struct UnifiedSessionStorage {
    /// Base directory for storing sessions
    base_path: PathBuf,
    /// Sequence counter for records
    seq_counter: AtomicU64,
}

impl UnifiedSessionStorage {
    /// Create a new unified session storage
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
            seq_counter: AtomicU64::new(0),
        }
    }

    /// Create storage with default path (~/.sage/sessions)
    pub fn default_path() -> SageResult<Self> {
        let home = dirs::home_dir()
            .ok_or_else(|| SageError::config("Could not determine home directory".to_string()))?;
        let base_path = home.join(".sage").join("sessions");
        Ok(Self::new(base_path))
    }

    /// Create storage for a specific project directory
    pub fn for_project(project_dir: &PathBuf) -> SageResult<Self> {
        let home = dirs::home_dir()
            .ok_or_else(|| SageError::config("Could not determine home directory".to_string()))?;

        // Escape the project path for use as directory name
        let escaped = project_dir
            .to_string_lossy()
            .replace('/', "-")
            .replace('\\', "-")
            .trim_start_matches('-')
            .to_string();

        let base_path = home.join(".sage").join("projects").join(escaped);
        Ok(Self::new(base_path))
    }

    // =========================================================================
    // Path helpers
    // =========================================================================

    /// Get the directory path for a session
    fn session_dir(&self, id: &SessionId) -> PathBuf {
        self.base_path.join(id)
    }

    /// Get the metadata file path
    fn metadata_path(&self, id: &SessionId) -> PathBuf {
        self.session_dir(id).join("metadata.json")
    }

    /// Get the records file path (JSONL)
    fn records_path(&self, id: &SessionId) -> PathBuf {
        self.session_dir(id).join("records.jsonl")
    }

    /// Ensure the session directory exists
    async fn ensure_session_dir(&self, id: &SessionId) -> SageResult<()> {
        let dir = self.session_dir(id);
        if !dir.exists() {
            fs::create_dir_all(&dir)
                .await
                .map_err(|e| SageError::io(format!("Failed to create session directory: {}", e)))?;
        }
        Ok(())
    }

    /// Ensure the sequence counter is initialized from disk if needed.
    async fn ensure_seq_initialized(&self, id: &SessionId) -> SageResult<()> {
        if self.seq_counter.load(Ordering::SeqCst) == 0 {
            let path = self.records_path(id);
            if path.exists() {
                let _ = self.load_records(id).await?;
            }
        }
        Ok(())
    }

    /// Get next sequence number (monotonic across restarts).
    async fn next_seq_for(&self, id: &SessionId) -> SageResult<u64> {
        self.ensure_seq_initialized(id).await?;
        Ok(self.seq_counter.fetch_add(1, Ordering::SeqCst))
    }

    // =========================================================================
    // Session CRUD
    // =========================================================================

    /// Create a new session
    pub async fn create_session(
        &self,
        id: impl Into<String>,
        working_directory: PathBuf,
    ) -> SageResult<SessionHeader> {
        let id = id.into();
        self.ensure_session_dir(&id).await?;

        let mut header = SessionHeader::new(&id, working_directory.clone());

        // Detect git branch
        let mut context = SessionContext::new(working_directory);
        context.detect_git_branch();
        if let Some(branch) = context.git_branch {
            header = header.with_git_branch(branch);
        }

        // Save initial metadata
        self.save_header(&id, &header).await?;

        info!("Created new session: {}", id);
        Ok(header)
    }

    /// Create a sidechain (branched) session
    pub async fn create_sidechain(
        &self,
        id: impl Into<String>,
        parent_session_id: impl Into<String>,
        root_message_id: impl Into<String>,
        working_directory: PathBuf,
    ) -> SageResult<SessionHeader> {
        let id = id.into();
        self.ensure_session_dir(&id).await?;

        let mut header = SessionHeader::new(&id, working_directory.clone());

        // Detect git branch
        let mut context = SessionContext::new(working_directory);
        context.detect_git_branch();
        if let Some(branch) = context.git_branch {
            header = header.with_git_branch(branch);
        }

        // Mark as sidechain
        header = header.as_sidechain(parent_session_id, root_message_id);

        // Save initial metadata
        self.save_header(&id, &header).await?;

        info!("Created sidechain session: {}", id);
        Ok(header)
    }

    /// Create a sidechain session with optional root message ID (legacy helper).
    pub async fn create_sidechain_session(
        &self,
        id: impl Into<String>,
        parent_session_id: impl Into<String>,
        root_message_id: Option<String>,
        working_directory: PathBuf,
    ) -> SageResult<SessionHeader> {
        let id = id.into();
        let parent_id = parent_session_id.into();
        self.ensure_session_dir(&id).await?;

        let mut header = SessionHeader::new(&id, working_directory.clone());

        // Detect git branch
        let mut context = SessionContext::new(working_directory);
        context.detect_git_branch();
        if let Some(branch) = context.git_branch {
            header = header.with_git_branch(branch);
        }

        // Mark as sidechain (root message is optional for legacy callers)
        if let Some(root_id) = root_message_id {
            header = header.as_sidechain(&parent_id, root_id);
        } else {
            header.is_sidechain = true;
            header.parent_session_id = Some(parent_id);
        }

        self.save_header(&id, &header).await?;

        info!("Created sidechain session: {}", id);
        Ok(header)
    }

    /// Delete a session
    pub async fn delete_session(&self, id: &SessionId) -> SageResult<()> {
        let dir = self.session_dir(id);

        if dir.exists() {
            fs::remove_dir_all(&dir)
                .await
                .map_err(|e| SageError::io(format!("Failed to delete session: {}", e)))?;
            info!("Deleted session: {}", id);
        } else {
            warn!("Session not found: {}", id);
        }

        Ok(())
    }

    /// Check if a session exists
    pub async fn session_exists(&self, id: &SessionId) -> bool {
        self.metadata_path(id).exists()
    }

    /// List all sessions
    pub async fn list_sessions(&self) -> SageResult<Vec<SessionHeader>> {
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
                    match self.load_header(&id).await {
                        Ok(Some(header)) => sessions.push(header),
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

    // =========================================================================
    // Header (metadata.json) operations
    // =========================================================================

    /// Save session header
    pub async fn save_header(&self, id: &SessionId, header: &SessionHeader) -> SageResult<()> {
        self.ensure_session_dir(id).await?;

        let path = self.metadata_path(id);
        let json = serde_json::to_string_pretty(header)
            .map_err(|e| SageError::json(format!("Failed to serialize header: {}", e)))?;

        fs::write(&path, json)
            .await
            .map_err(|e| SageError::io(format!("Failed to write header: {}", e)))?;

        debug!("Saved header for session {}", id);
        Ok(())
    }

    /// Save session metadata (legacy alias for header).
    pub async fn save_metadata(&self, id: &SessionId, header: &SessionHeader) -> SageResult<()> {
        self.save_header(id, header).await
    }

    /// Load session header
    pub async fn load_header(&self, id: &SessionId) -> SageResult<Option<SessionHeader>> {
        let path = self.metadata_path(id);

        if !path.exists() {
            return Ok(None);
        }

        let json = fs::read_to_string(&path)
            .await
            .map_err(|e| SageError::io(format!("Failed to read header: {}", e)))?;

        let header: SessionHeader = serde_json::from_str(&json)
            .map_err(|e| SageError::json(format!("Failed to deserialize header: {}", e)))?;

        Ok(Some(header))
    }

    /// Load session metadata (legacy alias for header).
    pub async fn load_metadata(&self, id: &SessionId) -> SageResult<Option<SessionHeader>> {
        self.load_header(id).await
    }

    /// Update session header with a patch
    pub async fn update_header(
        &self,
        id: &SessionId,
        patch: &SessionMetadataPatch,
    ) -> SageResult<()> {
        let mut header = self
            .load_header(id)
            .await?
            .ok_or_else(|| SageError::not_found(format!("Session not found: {}", id)))?;

        // Apply patch
        header.updated_at = patch.updated_at;
        if let Some(count) = patch.message_count {
            header.message_count = count;
        }
        if let Some(ref prompt) = patch.last_prompt {
            header.last_prompt = Some(prompt.clone());
        }
        if let Some(ref summary) = patch.summary {
            header.summary = Some(summary.clone());
        }
        if let Some(ref title) = patch.custom_title {
            header.custom_title = Some(title.clone());
        }
        if let Some(state) = patch.state {
            header.state = state;
        }
        if let Some(ref usage) = patch.token_usage {
            header.token_usage = usage.clone();
        }

        self.save_header(id, &header).await?;

        // Also append patch to records for audit trail
        self.append_record(
            id,
            SessionRecordPayload::MetadataPatch(patch.clone()),
        )
        .await?;

        Ok(())
    }

    /// Update header fields based on an appended message.
    async fn update_header_from_message(
        &self,
        id: &SessionId,
        message: &SessionMessage,
    ) -> SageResult<()> {
        let mut header = self
            .load_header(id)
            .await?
            .ok_or_else(|| SageError::not_found(format!("Session not found: {}", id)))?;

        let mut patch = SessionMetadataPatch {
            updated_at: Utc::now(),
            message_count: Some(header.message_count + 1),
            last_prompt: None,
            summary: None,
            custom_title: None,
            state: None,
            token_usage: None,
        };

        if message.message_type == SessionMessageType::User {
            if header.first_prompt.is_none() {
                header.set_first_prompt_if_empty(&message.message.content);
            }
            header.set_last_prompt(&message.message.content);
            patch.last_prompt = header.last_prompt.clone();
        }

        if let Some(usage) = &message.usage {
            let mut total = header.token_usage.clone();
            total.add(usage);
            patch.token_usage = Some(total.clone());
        }

        if let Some(count) = patch.message_count {
            header.message_count = count;
        }
        if let Some(ref usage) = patch.token_usage {
            header.token_usage = usage.clone();
        }
        header.updated_at = patch.updated_at;

        self.save_header(id, &header).await?;
        self.append_record(id, SessionRecordPayload::MetadataPatch(patch))
            .await?;

        Ok(())
    }

    // =========================================================================
    // Record (JSONL) operations
    // =========================================================================

    /// Append a record to the session (real-time persistence)
    pub async fn append_record(
        &self,
        id: &SessionId,
        payload: SessionRecordPayload,
    ) -> SageResult<()> {
        self.ensure_session_dir(id).await?;

        let record = SessionRecord {
            seq: self.next_seq_for(id).await?,
            timestamp: Utc::now(),
            session_id: id.clone(),
            payload,
        };

        let path = self.records_path(id);
        let json = serde_json::to_string(&record)
            .map_err(|e| SageError::json(format!("Failed to serialize record: {}", e)))?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await
            .map_err(|e| SageError::io(format!("Failed to open records file: {}", e)))?;

        file.write_all(json.as_bytes()).await.map_err(|e| {
            SageError::io(format!("Failed to write record: {}", e))
        })?;
        file.write_all(b"\n").await.map_err(|e| {
            SageError::io(format!("Failed to write newline: {}", e))
        })?;
        file.flush().await.map_err(|e| {
            SageError::io(format!("Failed to flush records: {}", e))
        })?;

        debug!("Appended record to session {}", id);
        Ok(())
    }

    /// Append a message to the session
    pub async fn append_message(
        &self,
        id: &SessionId,
        message: &SessionMessage,
    ) -> SageResult<MessageId> {
        let uuid = message.uuid.clone();
        self.append_record(id, SessionRecordPayload::Message(message.clone()))
            .await?;
        self.update_header_from_message(id, message).await?;
        Ok(uuid)
    }

    /// Append a file snapshot to the session
    pub async fn append_snapshot(
        &self,
        id: &SessionId,
        snapshot: impl Into<FileHistorySnapshot>,
    ) -> SageResult<()> {
        let snapshot = snapshot.into();
        self.append_record(id, SessionRecordPayload::Snapshot(snapshot))
            .await
    }

    /// Load all records from a session
    pub async fn load_records(&self, id: &SessionId) -> SageResult<Vec<SessionRecord>> {
        let path = self.records_path(id);

        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&path)
            .await
            .map_err(|e| SageError::io(format!("Failed to open records file: {}", e)))?;

        let reader = BufReader::new(file);
        let mut lines = reader.lines();
        let mut records = Vec::new();

        while let Some(line) = lines
            .next_line()
            .await
            .map_err(|e| SageError::io(format!("Failed to read line: {}", e)))?
        {
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<SessionRecord>(&line) {
                Ok(record) => records.push(record),
                Err(e) => {
                    warn!("Failed to parse record: {} - line: {}", e, &line[..100.min(line.len())]);
                }
            }
        }

        // Update sequence counter to max seen
        if let Some(max_seq) = records.iter().map(|r| r.seq).max() {
            self.seq_counter.store(max_seq + 1, Ordering::SeqCst);
        }

        Ok(records)
    }

    /// Load all messages from a session
    pub async fn load_messages(&self, id: &SessionId) -> SageResult<Vec<SessionMessage>> {
        let records = self.load_records(id).await?;
        let messages = records
            .into_iter()
            .filter_map(|r| match r.payload {
                SessionRecordPayload::Message(msg) => Some(msg),
                _ => None,
            })
            .collect();
        Ok(messages)
    }

    /// Get message by UUID.
    pub async fn get_message(
        &self,
        session_id: &SessionId,
        message_uuid: &str,
    ) -> SageResult<Option<SessionMessage>> {
        let records = self.load_records(session_id).await?;
        for record in records {
            if let SessionRecordPayload::Message(msg) = record.payload {
                if msg.uuid == message_uuid {
                    return Ok(Some(msg));
                }
            }
        }
        Ok(None)
    }

    /// Get messages up to a specific UUID (for undo).
    pub async fn get_messages_until(
        &self,
        session_id: &SessionId,
        message_uuid: &str,
    ) -> SageResult<Vec<SessionMessage>> {
        let records = self.load_records(session_id).await?;
        let mut result = Vec::new();
        for record in records {
            if let SessionRecordPayload::Message(msg) = record.payload {
                let is_target = msg.uuid == message_uuid;
                result.push(msg);
                if is_target {
                    break;
                }
            }
        }
        Ok(result)
    }

    /// Get the message chain (following parentUuid links).
    pub async fn get_message_chain(
        &self,
        session_id: &SessionId,
        start_uuid: &str,
    ) -> SageResult<Vec<SessionMessage>> {
        let messages = self.load_messages(session_id).await?;
        let msg_map: HashMap<&str, &SessionMessage> =
            messages.iter().map(|m| (m.uuid.as_str(), m)).collect();

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

        chain.reverse();
        Ok(chain)
    }

    /// Load all snapshots from a session
    pub async fn load_snapshots(&self, id: &SessionId) -> SageResult<Vec<FileHistorySnapshot>> {
        let records = self.load_records(id).await?;
        let snapshots = records
            .into_iter()
            .filter_map(|r| match r.payload {
                SessionRecordPayload::Snapshot(snap) => Some(snap),
                _ => None,
            })
            .collect();
        Ok(snapshots)
    }

    /// Load a complete session (header + messages + snapshots)
    pub async fn load_session(&self, id: &SessionId) -> SageResult<Option<Session>> {
        let header = match self.load_header(id).await? {
            Some(h) => h,
            None => return Ok(None),
        };

        let records = self.load_records(id).await?;

        let mut messages = Vec::new();
        let mut snapshots = Vec::new();

        for record in records {
            match record.payload {
                SessionRecordPayload::Message(msg) => messages.push(msg),
                SessionRecordPayload::Snapshot(snap) => snapshots.push(snap),
                SessionRecordPayload::MetadataPatch(_) => {} // Already applied to header
            }
        }

        Ok(Some(Session {
            header,
            messages,
            snapshots,
        }))
    }

    /// Save a complete session (header + records).
    pub async fn save_session(&self, session: &Session) -> SageResult<()> {
        let id = &session.header.id;
        self.ensure_session_dir(id).await?;
        self.save_header(id, &session.header).await?;

        let path = self.records_path(id);
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)
            .await
            .map_err(|e| SageError::io(format!("Failed to open records file: {}", e)))?;

        let mut seq = 0u64;
        for msg in &session.messages {
            let record = SessionRecord {
                seq,
                timestamp: msg.timestamp,
                session_id: id.clone(),
                payload: SessionRecordPayload::Message(msg.clone()),
            };
            let json = serde_json::to_string(&record)
                .map_err(|e| SageError::json(format!("Failed to serialize record: {}", e)))?;
            file.write_all(json.as_bytes())
                .await
                .map_err(|e| SageError::io(format!("Failed to write record: {}", e)))?;
            file.write_all(b"\n")
                .await
                .map_err(|e| SageError::io(format!("Failed to write newline: {}", e)))?;
            seq += 1;
        }

        for snap in &session.snapshots {
            let record = SessionRecord {
                seq,
                timestamp: snap.timestamp,
                session_id: id.clone(),
                payload: SessionRecordPayload::Snapshot(snap.clone()),
            };
            let json = serde_json::to_string(&record)
                .map_err(|e| SageError::json(format!("Failed to serialize record: {}", e)))?;
            file.write_all(json.as_bytes())
                .await
                .map_err(|e| SageError::io(format!("Failed to write record: {}", e)))?;
            file.write_all(b"\n")
                .await
                .map_err(|e| SageError::io(format!("Failed to write newline: {}", e)))?;
            seq += 1;
        }

        file.flush()
            .await
            .map_err(|e| SageError::io(format!("Failed to flush records: {}", e)))?;
        self.seq_counter.store(seq, Ordering::SeqCst);

        Ok(())
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::session::types::unified::SessionState;

    #[tokio::test]
    async fn test_create_session() {
        let tmp = TempDir::new().unwrap();
        let storage = UnifiedSessionStorage::new(tmp.path());

        let header = storage
            .create_session("test-session", PathBuf::from("/tmp"))
            .await
            .unwrap();

        assert_eq!(header.id, "test-session");
        assert_eq!(header.state, SessionState::Active);
        assert!(!header.is_sidechain);
    }

    #[tokio::test]
    async fn test_append_and_load_messages() {
        let tmp = TempDir::new().unwrap();
        let storage = UnifiedSessionStorage::new(tmp.path());

        let _header = storage
            .create_session("test-session", PathBuf::from("/tmp"))
            .await
            .unwrap();

        let ctx = SessionContext::new(PathBuf::from("/tmp"));
        let msg = SessionMessage::user("Hello", "test-session", ctx);

        storage
            .append_message(&"test-session".to_string(), &msg)
            .await
            .unwrap();

        let messages = storage
            .load_messages(&"test-session".to_string())
            .await
            .unwrap();

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].message.content, "Hello");
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let tmp = TempDir::new().unwrap();
        let storage = UnifiedSessionStorage::new(tmp.path());

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

    #[tokio::test]
    async fn test_delete_session() {
        let tmp = TempDir::new().unwrap();
        let storage = UnifiedSessionStorage::new(tmp.path());

        storage
            .create_session("test-session", PathBuf::from("/tmp"))
            .await
            .unwrap();

        assert!(storage.session_exists(&"test-session".to_string()).await);

        storage
            .delete_session(&"test-session".to_string())
            .await
            .unwrap();

        assert!(!storage.session_exists(&"test-session".to_string()).await);
    }

    #[tokio::test]
    async fn test_load_full_session() {
        let tmp = TempDir::new().unwrap();
        let storage = UnifiedSessionStorage::new(tmp.path());

        storage
            .create_session("test-session", PathBuf::from("/tmp"))
            .await
            .unwrap();

        let ctx = SessionContext::new(PathBuf::from("/tmp"));
        let msg1 = SessionMessage::user("Hello", "test-session", ctx.clone());
        let msg2 = SessionMessage::assistant("Hi!", "test-session", ctx, Some(msg1.uuid.clone()));

        storage
            .append_message(&"test-session".to_string(), &msg1)
            .await
            .unwrap();
        storage
            .append_message(&"test-session".to_string(), &msg2)
            .await
            .unwrap();

        let session = storage
            .load_session(&"test-session".to_string())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(session.header.id, "test-session");
        assert_eq!(session.messages.len(), 2);
    }
}
