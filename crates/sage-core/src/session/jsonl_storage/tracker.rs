//! Message chain tracker for building parent-child relationships

use super::super::types::{EnhancedMessage, SessionContext, ThinkingMetadata, TodoItem};

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
        let session_id = self
            .session_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let context = self
            .context
            .clone()
            .unwrap_or_else(|| SessionContext::new(std::env::current_dir().unwrap_or_default()));

        let mut msg =
            EnhancedMessage::user(content, &session_id, context).with_todos(self.todos.clone());

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
        let session_id = self
            .session_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let context = self
            .context
            .clone()
            .unwrap_or_else(|| SessionContext::new(std::env::current_dir().unwrap_or_default()));

        let mut msg =
            EnhancedMessage::assistant(content, &session_id, context, self.last_uuid.clone())
                .with_todos(self.todos.clone());

        if let Some(thinking) = &self.thinking {
            msg = msg.with_thinking(thinking.clone());
        }

        self.last_uuid = Some(msg.uuid.clone());
        msg
    }
}
