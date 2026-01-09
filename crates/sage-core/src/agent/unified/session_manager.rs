//! Session manager component for unified executor
//!
//! Encapsulates all session-related state and provides a clean API
//! for session recording, file tracking, and message chain management.

use crate::error::SageResult;
use crate::session::{
    FileSnapshotTracker, JsonlSessionStorage, MessageChainTracker, SessionContext,
};
use crate::trajectory::SessionRecorder;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Manages session state including recording, file tracking, and message chains
pub struct SessionManager {
    /// Session recorder for trajectory recording
    pub(super) session_recorder: Option<Arc<Mutex<SessionRecorder>>>,
    /// JSONL session storage for enhanced messages
    pub(super) jsonl_storage: Option<Arc<JsonlSessionStorage>>,
    /// Message chain tracker for building message relationships
    pub(super) message_tracker: MessageChainTracker,
    /// Current session ID
    pub(super) current_session_id: Option<String>,
    /// File snapshot tracker for undo capability
    pub(super) file_tracker: FileSnapshotTracker,
    /// Message count at last summary update (for throttling summary generation)
    pub(super) last_summary_msg_count: usize,
}

impl SessionManager {
    /// Create a new session manager with the given working directory
    pub fn new(working_dir: PathBuf) -> Self {
        let jsonl_storage = JsonlSessionStorage::default_path().ok().map(Arc::new);
        let context = SessionContext::new(working_dir);
        let message_tracker = MessageChainTracker::new().with_context(context);

        Self {
            session_recorder: None,
            jsonl_storage,
            message_tracker,
            current_session_id: None,
            file_tracker: FileSnapshotTracker::default_tracker(),
            last_summary_msg_count: 0,
        }
    }

    /// Set session recorder
    pub fn set_session_recorder(&mut self, recorder: Arc<Mutex<SessionRecorder>>) {
        self.session_recorder = Some(recorder);
    }

    /// Get current session ID
    pub fn current_session_id(&self) -> Option<&str> {
        self.current_session_id.as_deref()
    }

    /// Set current session ID
    pub fn set_current_session_id(&mut self, session_id: Option<String>) {
        self.current_session_id = session_id;
    }

    /// Get the file tracker for external file tracking
    pub fn file_tracker_mut(&mut self) -> &mut FileSnapshotTracker {
        &mut self.file_tracker
    }

    /// Get the file tracker reference
    pub fn file_tracker(&self) -> &FileSnapshotTracker {
        &self.file_tracker
    }

    /// Get session recorder reference
    pub fn session_recorder(&self) -> Option<&Arc<Mutex<SessionRecorder>>> {
        self.session_recorder.as_ref()
    }

    /// Get JSONL storage reference
    pub fn jsonl_storage(&self) -> Option<&Arc<JsonlSessionStorage>> {
        self.jsonl_storage.as_ref()
    }

    /// Set JSONL storage
    pub fn set_jsonl_storage(&mut self, storage: Arc<JsonlSessionStorage>) {
        self.jsonl_storage = Some(storage);
    }

    /// Get message tracker reference
    pub fn message_tracker(&self) -> &MessageChainTracker {
        &self.message_tracker
    }

    /// Get mutable message tracker reference
    pub fn message_tracker_mut(&mut self) -> &mut MessageChainTracker {
        &mut self.message_tracker
    }

    /// Get last summary message count
    pub fn last_summary_msg_count(&self) -> usize {
        self.last_summary_msg_count
    }

    /// Set last summary message count
    pub fn set_last_summary_msg_count(&mut self, count: usize) {
        self.last_summary_msg_count = count;
    }

    /// Check if session recording is active
    pub fn is_recording_active(&self) -> bool {
        self.current_session_id.is_some() && self.jsonl_storage.is_some()
    }

    /// Track a file for snapshot capability
    pub async fn track_file(&mut self, path: impl AsRef<std::path::Path>) -> SageResult<()> {
        self.file_tracker.track_file(path).await
    }

    /// Clear the file tracker
    pub fn clear_file_tracker(&mut self) {
        self.file_tracker.clear();
    }

    /// Check if file tracker is empty
    pub fn is_file_tracker_empty(&self) -> bool {
        self.file_tracker.is_empty()
    }

    /// Create a file snapshot
    pub async fn create_file_snapshot(
        &self,
        message_uuid: &str,
    ) -> SageResult<crate::session::FileHistorySnapshot> {
        self.file_tracker.create_snapshot(message_uuid).await
    }

    /// Reset message tracker with new session and context
    pub fn reset_message_tracker(&mut self, session_id: &str, working_dir: PathBuf) {
        let mut context = SessionContext::new(working_dir);
        context.detect_git_branch();
        self.message_tracker = MessageChainTracker::new()
            .with_session(session_id)
            .with_context(context);
    }

    /// Update todos in the message tracker
    pub fn set_todos(&mut self, todos: Vec<crate::session::TodoItem>) {
        self.message_tracker.set_todos(todos);
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new(std::env::current_dir().unwrap_or_default())
    }
}
