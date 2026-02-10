//! Session manager component for unified executor
//!
//! Encapsulates all session-related state and provides a clean API
//! for session recording, file tracking, and message chain management.

use crate::error::SageResult;
use crate::llm::messages::{LlmMessage, LlmResponse};
use crate::session::{
    FileSnapshotTracker, JsonlSessionStorage, MessageChainTracker, SessionContext,
};
use crate::tools::types::ToolSchema;
use crate::trajectory::SessionRecorder;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::recording;

/// Manages session state including recording, file tracking, and message chains
pub struct AgentSessionManager {
    pub(super) session_recorder: Option<Arc<Mutex<SessionRecorder>>>,
    pub(super) jsonl_storage: Option<Arc<JsonlSessionStorage>>,
    pub(super) message_tracker: MessageChainTracker,
    pub(super) current_session_id: Option<String>,
    pub(super) file_tracker: FileSnapshotTracker,
    pub(super) last_summary_msg_count: usize,
}

impl AgentSessionManager {
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

    pub fn set_session_recorder(&mut self, recorder: Arc<Mutex<SessionRecorder>>) {
        self.session_recorder = Some(recorder);
    }

    pub fn current_session_id(&self) -> Option<&str> {
        self.current_session_id.as_deref()
    }

    pub fn set_current_session_id(&mut self, session_id: Option<String>) {
        self.current_session_id = session_id;
    }

    pub fn file_tracker_mut(&mut self) -> &mut FileSnapshotTracker {
        &mut self.file_tracker
    }

    pub fn file_tracker(&self) -> &FileSnapshotTracker {
        &self.file_tracker
    }

    pub fn session_recorder(&self) -> Option<&Arc<Mutex<SessionRecorder>>> {
        self.session_recorder.as_ref()
    }

    pub fn jsonl_storage(&self) -> Option<&Arc<JsonlSessionStorage>> {
        self.jsonl_storage.as_ref()
    }

    pub fn set_jsonl_storage(&mut self, storage: Arc<JsonlSessionStorage>) {
        self.jsonl_storage = Some(storage);
    }

    pub fn message_tracker(&self) -> &MessageChainTracker {
        &self.message_tracker
    }

    pub fn message_tracker_mut(&mut self) -> &mut MessageChainTracker {
        &mut self.message_tracker
    }

    pub fn last_summary_msg_count(&self) -> usize {
        self.last_summary_msg_count
    }

    pub fn set_last_summary_msg_count(&mut self, count: usize) {
        self.last_summary_msg_count = count;
    }

    pub fn is_recording_active(&self) -> bool {
        self.current_session_id.is_some() && self.jsonl_storage.is_some()
    }

    pub async fn track_file(&mut self, path: impl AsRef<std::path::Path>) -> SageResult<()> {
        self.file_tracker.track_file(path).await
    }

    pub fn clear_file_tracker(&mut self) {
        self.file_tracker.clear();
    }

    pub fn is_file_tracker_empty(&self) -> bool {
        self.file_tracker.is_empty()
    }

    pub async fn create_file_snapshot(
        &self,
        message_uuid: &str,
    ) -> SageResult<crate::session::FileHistorySnapshot> {
        self.file_tracker.create_snapshot(message_uuid).await
    }

    pub fn reset_message_tracker(&mut self, session_id: &str, working_dir: PathBuf) {
        let mut context = SessionContext::new(working_dir);
        context.detect_git_branch();
        self.message_tracker = MessageChainTracker::new()
            .with_session(session_id)
            .with_context(context);
    }

    pub fn set_todos(&mut self, todos: Vec<crate::session::TodoItem>) {
        self.message_tracker.set_todos(todos);
    }

    // Recording delegation methods

    pub async fn record_tool_call(&self, tool_name: &str, arguments: &serde_json::Value) {
        if let Some(recorder) = &self.session_recorder {
            recording::record_tool_call(recorder, tool_name, arguments).await;
        }
    }

    pub async fn record_tool_result(
        &self,
        tool_name: &str,
        success: bool,
        output: Option<String>,
        error: Option<String>,
        execution_time_ms: u64,
    ) {
        if let Some(recorder) = &self.session_recorder {
            recording::record_tool_result(
                recorder,
                tool_name,
                success,
                output,
                error,
                execution_time_ms,
            )
            .await;
        }
    }

    pub async fn record_llm_request(&self, messages: &[LlmMessage], tool_schemas: &[ToolSchema]) {
        if let Some(recorder) = &self.session_recorder {
            recording::record_llm_request(recorder, messages, tool_schemas).await;
        }
    }

    pub async fn record_llm_response(&self, llm_response: &LlmResponse, model: &str) {
        if let Some(recorder) = &self.session_recorder {
            recording::record_llm_response(recorder, llm_response, model).await;
        }
    }
}

impl Default for AgentSessionManager {
    fn default() -> Self {
        Self::new(std::env::current_dir().unwrap_or_default())
    }
}
