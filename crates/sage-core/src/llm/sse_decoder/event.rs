//! SSE event types

/// A parsed SSE event from the stream
#[derive(Debug, Clone, PartialEq)]
pub struct SseEvent {
    /// Event type (e.g., "message_start", "content_block_delta")
    pub event_type: Option<String>,
    /// Event data (the JSON payload)
    pub data: String,
    /// Event ID (optional, rarely used by LLM providers)
    pub id: Option<String>,
}

impl SseEvent {
    /// Create a new SSE event with just data
    pub fn new(data: impl Into<String>) -> Self {
        Self {
            event_type: None,
            data: data.into(),
            id: None,
        }
    }

    /// Create an SSE event with event type and data
    pub fn with_type(event_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self {
            event_type: Some(event_type.into()),
            data: data.into(),
            id: None,
        }
    }

    /// Check if this is a `[DONE]` marker (OpenAI format)
    pub fn is_done(&self) -> bool {
        self.data.trim() == "[DONE]"
    }
}
