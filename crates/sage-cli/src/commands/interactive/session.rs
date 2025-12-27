//! Conversation session management

use sage_core::agent::AgentExecution;
use sage_core::llm::messages::LlmMessage;
use sage_core::types::TaskMetadata;
use std::collections::HashMap;

/// Conversation session manager for interactive mode
pub struct ConversationSession {
    /// Current conversation messages
    pub messages: Vec<LlmMessage>,
    /// Current task metadata
    pub task: Option<TaskMetadata>,
    /// Current agent execution
    pub execution: Option<AgentExecution>,
    /// Session metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Whether this is the first message in the conversation
    is_first_message: bool,
    /// Current JSONL session ID for persistence
    session_id: Option<String>,
}

impl ConversationSession {
    /// Create a new conversation session
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            task: None,
            execution: None,
            metadata: HashMap::new(),
            is_first_message: true,
            session_id: None,
        }
    }

    /// Get the current session ID
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// Set the session ID
    pub fn set_session_id(&mut self, id: impl Into<String>) {
        self.session_id = Some(id.into());
    }

    /// Add a user message to the conversation
    pub fn add_user_message(&mut self, content: &str) {
        self.messages.push(LlmMessage::user(content));
    }

    /// Add an assistant message to the conversation
    pub fn add_assistant_message(&mut self, content: &str) {
        self.messages.push(LlmMessage::assistant(content));
    }

    /// Check if this is a new conversation (no messages yet)
    pub fn is_new_conversation(&self) -> bool {
        self.is_first_message
    }

    /// Mark that the first message has been processed
    pub fn mark_first_message_processed(&mut self) {
        self.is_first_message = false;
    }

    /// Reset the conversation session
    pub fn reset(&mut self) {
        self.messages.clear();
        self.task = None;
        self.execution = None;
        self.metadata.clear();
        self.is_first_message = true;
        self.session_id = None;
    }

    /// Get conversation summary
    pub fn get_summary(&self) -> String {
        format!("Conversation with {} messages", self.messages.len())
    }
}
