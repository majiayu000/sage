//! Message persister for unified session storage.
//!
//! Provides a thread-safe API for real-time message persistence using the
//! unified session data model, including message chaining, token usage
//! accumulation, and session state updates.

use crate::error::{SageError, SageResult};
use crate::session::MessageChainTracker;
use crate::session::UnifiedSessionStorage;
use crate::session::types::unified::{
    FileHistorySnapshot, MessageRole, SessionContext, SessionHeader, SessionMessage,
    SessionMessageType, SessionMetadataPatch, SessionState, ThinkingMetadata, TodoItem,
    TokenUsage, ToolCall, ToolResult,
};
use chrono::Utc;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Thread-safe message persister for unified session storage.
#[derive(Clone)]
pub struct MessagePersister {
    storage: Arc<UnifiedSessionStorage>,
    state: Arc<RwLock<PersisterState>>,
}

#[derive(Debug)]
struct PersisterState {
    session_id: String,
    tracker: MessageChainTracker,
    token_usage: TokenUsage,
    message_count: usize,
    session_state: SessionState,
    first_prompt_set: bool,
}

impl MessagePersister {
    /// Create a new session and initialize the persister.
    pub async fn create_new(
        storage: Arc<UnifiedSessionStorage>,
        working_dir: PathBuf,
        model: Option<String>,
    ) -> SageResult<Self> {
        let session_id = uuid::Uuid::new_v4().to_string();
        Self::create_with_id(storage, session_id, working_dir, model, None).await
    }

    /// Create a new sidechain session and initialize the persister.
    pub async fn create_sidechain(
        storage: Arc<UnifiedSessionStorage>,
        parent_session_id: impl Into<String>,
        root_message_id: impl Into<String>,
        working_dir: PathBuf,
        model: Option<String>,
    ) -> SageResult<Self> {
        let parent_session_id = parent_session_id.into();
        let root_message_id = root_message_id.into();
        let session_id = uuid::Uuid::new_v4().to_string();

        Self::create_with_id(
            storage,
            session_id,
            working_dir,
            model,
            Some((parent_session_id, root_message_id)),
        )
        .await
    }

    /// Create a persister from an existing session header.
    pub fn from_existing(
        storage: Arc<UnifiedSessionStorage>,
        header: SessionHeader,
        last_message_uuid: Option<String>,
    ) -> Self {
        let mut context = SessionContext::new(header.working_directory.clone());
        if let Some(branch) = header.git_branch.clone() {
            context = context.with_git_branch(branch);
        } else {
            context.detect_git_branch();
        }

        let mut tracker = MessageChainTracker::new()
            .with_session(header.id.clone())
            .with_context(context);

        if let Some(uuid) = last_message_uuid {
            tracker.set_last_uuid(uuid);
        }

        let state = PersisterState {
            session_id: header.id.clone(),
            tracker,
            token_usage: header.token_usage.clone(),
            message_count: header.message_count,
            session_state: header.state,
            first_prompt_set: header.first_prompt.is_some(),
        };

        Self {
            storage,
            state: Arc::new(RwLock::new(state)),
        }
    }

    /// Get the session ID.
    pub async fn session_id(&self) -> String {
        self.state.read().await.session_id.clone()
    }

    /// Access the underlying storage.
    pub fn storage(&self) -> Arc<UnifiedSessionStorage> {
        Arc::clone(&self.storage)
    }

    /// Get last message UUID.
    pub async fn last_message_uuid(&self) -> Option<String> {
        self.state
            .read()
            .await
            .tracker
            .last_message_uuid()
            .map(|s| s.to_string())
    }

    /// Set last message UUID (used for restore/branching).
    pub async fn set_last_message_uuid(&self, uuid: impl Into<String>) {
        self.state
            .write()
            .await
            .tracker
            .set_last_uuid(uuid.into());
    }

    /// Update todos on the tracker.
    pub async fn set_todos(&self, todos: Vec<TodoItem>) {
        self.state.write().await.tracker.set_todos(todos);
    }

    /// Update thinking metadata on the tracker.
    pub async fn set_thinking(&self, thinking: ThinkingMetadata) {
        self.state.write().await.tracker.set_thinking(thinking);
    }

    /// Record a user message and persist it.
    pub async fn record_user_message(&self, content: &str) -> SageResult<SessionMessage> {
        let (session_id, message, message_count, needs_first_prompt) = {
            let mut state = self.state.write().await;
            let message = state.tracker.create_user_message(content);
            state.message_count += 1;
            let needs_first_prompt = !state.first_prompt_set;
            (
                state.session_id.clone(),
                message,
                state.message_count,
                needs_first_prompt,
            )
        };

        self.storage.append_message(&session_id, &message).await?;

        let patch = SessionMetadataPatch {
            updated_at: Utc::now(),
            message_count: Some(message_count),
            last_prompt: Some(content.to_string()),
            summary: None,
            custom_title: None,
            state: None,
            token_usage: None,
        };

        self.storage.update_header(&session_id, &patch).await?;

        if needs_first_prompt {
            if let Ok(Some(mut header)) = self.storage.load_header(&session_id).await {
                header.set_first_prompt_if_empty(content);
                if let Err(e) = self.storage.save_header(&session_id, &header).await {
                    tracing::warn!(error = %e, session_id = %session_id, "Failed to save first prompt (non-fatal)");
                } else {
                    self.state.write().await.first_prompt_set = true;
                }
            }
        }

        Ok(message)
    }

    /// Record an assistant message and persist it.
    pub async fn record_assistant_message(
        &self,
        content: &str,
        tool_calls: Option<Vec<ToolCall>>,
        usage: Option<TokenUsage>,
    ) -> SageResult<SessionMessage> {
        let (session_id, message, message_count, token_usage) = {
            let mut state = self.state.write().await;
            let mut message = state.tracker.create_assistant_message(content);

            if let Some(calls) = tool_calls {
                message.message.tool_calls = Some(calls);
            }
            if let Some(ref u) = usage {
                message.usage = Some(u.clone());
                state.token_usage.add(u);
            }

            state.message_count += 1;
            (
                state.session_id.clone(),
                message,
                state.message_count,
                state.token_usage.clone(),
            )
        };

        self.storage.append_message(&session_id, &message).await?;

        let patch = SessionMetadataPatch {
            updated_at: Utc::now(),
            message_count: Some(message_count),
            last_prompt: None,
            summary: None,
            custom_title: None,
            state: None,
            token_usage: Some(token_usage),
        };

        self.storage.update_header(&session_id, &patch).await?;

        Ok(message)
    }

    /// Record a tool result message and persist it.
    pub async fn record_tool_result_message(
        &self,
        results: Vec<ToolResult>,
    ) -> SageResult<SessionMessage> {
        let (session_id, message, message_count) = {
            let mut state = self.state.write().await;
            let parent_uuid = state.tracker.parent_uuid();
            let context = state.tracker.context();
            let message =
                SessionMessage::tool_result(results, state.session_id.clone(), context, parent_uuid);
            state.tracker.set_last_uuid(message.uuid.clone());
            state.message_count += 1;
            (state.session_id.clone(), message, state.message_count)
        };

        self.storage.append_message(&session_id, &message).await?;

        let patch = SessionMetadataPatch {
            updated_at: Utc::now(),
            message_count: Some(message_count),
            last_prompt: None,
            summary: None,
            custom_title: None,
            state: None,
            token_usage: None,
        };

        self.storage.update_header(&session_id, &patch).await?;

        Ok(message)
    }

    /// Record an error message and persist it. Also marks session as failed.
    pub async fn record_error_message(
        &self,
        error_type: &str,
        error_message: &str,
    ) -> SageResult<SessionMessage> {
        let (session_id, message, message_count) = {
            let mut state = self.state.write().await;
            let parent_uuid = state.tracker.parent_uuid();
            let context = state.tracker.context();

            let mut metadata = HashMap::new();
            metadata.insert(
                "error_type".to_string(),
                serde_json::Value::String(error_type.to_string()),
            );

            let message = SessionMessage {
                message_type: SessionMessageType::Error,
                uuid: uuid::Uuid::new_v4().to_string(),
                parent_uuid,
                branch_id: None,
                branch_parent_uuid: None,
                timestamp: Utc::now(),
                session_id: state.session_id.clone(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                context,
                message: crate::session::types::unified::MessageContent {
                    role: MessageRole::Error,
                    content: format!("[{}] {}", error_type, error_message),
                    tool_calls: None,
                    tool_results: None,
                },
                usage: None,
                thinking_metadata: None,
                todos: Vec::new(),
                is_sidechain: false,
                metadata,
            };

            state.tracker.set_last_uuid(message.uuid.clone());
            state.message_count += 1;
            state.session_state = SessionState::Failed;

            (state.session_id.clone(), message, state.message_count)
        };

        self.storage.append_message(&session_id, &message).await?;

        let patch = SessionMetadataPatch {
            updated_at: Utc::now(),
            message_count: Some(message_count),
            last_prompt: None,
            summary: None,
            custom_title: None,
            state: Some(SessionState::Failed),
            token_usage: None,
        };

        self.storage.update_header(&session_id, &patch).await?;

        Ok(message)
    }

    /// Append a file history snapshot.
    pub async fn append_snapshot(&self, snapshot: &FileHistorySnapshot) -> SageResult<()> {
        let session_id = self.session_id().await;
        self.storage.append_snapshot(&session_id, snapshot).await
    }

    /// Update session state in metadata.
    pub async fn update_session_state(&self, state: SessionState) -> SageResult<()> {
        let session_id = {
            let mut guard = self.state.write().await;
            guard.session_state = state;
            guard.session_id.clone()
        };

        let patch = SessionMetadataPatch {
            updated_at: Utc::now(),
            message_count: None,
            last_prompt: None,
            summary: None,
            custom_title: None,
            state: Some(state),
            token_usage: None,
        };

        self.storage.update_header(&session_id, &patch).await
    }

    /// Update cached token usage (without adding a message).
    pub async fn add_token_usage(&self, usage: &TokenUsage) -> SageResult<()> {
        let (session_id, token_usage) = {
            let mut state = self.state.write().await;
            state.token_usage.add(usage);
            (state.session_id.clone(), state.token_usage.clone())
        };

        let patch = SessionMetadataPatch {
            updated_at: Utc::now(),
            message_count: None,
            last_prompt: None,
            summary: None,
            custom_title: None,
            state: None,
            token_usage: Some(token_usage),
        };

        self.storage.update_header(&session_id, &patch).await
    }

    async fn create_with_id(
        storage: Arc<UnifiedSessionStorage>,
        session_id: String,
        working_dir: PathBuf,
        model: Option<String>,
        sidechain: Option<(String, String)>,
    ) -> SageResult<Self> {
        let mut header = if let Some((parent_session_id, root_message_id)) = sidechain {
            storage
                .create_sidechain(
                    session_id.clone(),
                    parent_session_id,
                    root_message_id,
                    working_dir.clone(),
                )
                .await?
        } else {
            storage
                .create_session(session_id.clone(), working_dir.clone())
                .await?
        };

        if let Some(model) = model {
            header = header.with_model(model);
            storage.save_header(&session_id, &header).await?;
        }

        let mut context = SessionContext::new(working_dir);
        context.detect_git_branch();

        let tracker = MessageChainTracker::new()
            .with_session(&session_id)
            .with_context(context);

        let state = PersisterState {
            session_id: session_id.clone(),
            tracker,
            token_usage: header.token_usage.clone(),
            message_count: header.message_count,
            session_state: header.state,
            first_prompt_set: header.first_prompt.is_some(),
        };

        Ok(Self {
            storage,
            state: Arc::new(RwLock::new(state)),
        })
    }
}

impl std::fmt::Debug for MessagePersister {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MessagePersister")
            .field("session_id", &"<redacted>")
            .finish()
    }
}

impl std::fmt::Display for MessagePersister {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MessagePersister")
    }
}

impl MessagePersister {
    /// Ensure the persister is initialized for a session.
    pub async fn ensure_initialized(&self) -> SageResult<()> {
        let session_id = self.session_id().await;
        if !self.storage.session_exists(&session_id).await {
            return Err(SageError::not_found(format!(
                "Session not found: {}",
                session_id
            )));
        }
        Ok(())
    }
}
