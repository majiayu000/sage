//! Server-Sent Events (SSE) decoder for parsing LLM streaming responses
//!
//! This module provides a buffered SSE parser that handles:
//! - Multi-line data fields
//! - Event type prefixes
//! - Incomplete chunks across network boundaries
//! - Both OpenAI and Anthropic SSE formats

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

/// Deprecated: Use `SseEvent` instead
#[deprecated(since = "0.2.0", note = "Use `SseEvent` instead")]
pub type SSEEvent = SseEvent;

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

/// Buffered SSE decoder that handles partial chunks
///
/// SSE format (per spec):
/// ```text
/// event: event_type\n
/// id: optional_id\n
/// data: json_payload\n
/// data: continued_data\n
/// \n
/// ```
///
/// Events are separated by double newlines (\n\n)
#[derive(Debug, Default)]
pub struct SseDecoder {
    /// Buffer for incomplete data
    buffer: String,
}

/// Deprecated: Use `SseDecoder` instead
#[deprecated(since = "0.2.0", note = "Use `SseDecoder` instead")]
pub type SSEDecoder = SseDecoder;

impl SseDecoder {
    /// Create a new SSE decoder
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// Feed raw bytes into the decoder and extract complete events
    ///
    /// Returns a vector of complete SSE events parsed from the input.
    /// Incomplete events are buffered for the next call.
    pub fn feed(&mut self, chunk: &[u8]) -> Vec<SseEvent> {
        // Convert bytes to string (SSE is always UTF-8)
        let chunk_str = match std::str::from_utf8(chunk) {
            Ok(s) => s,
            Err(_) => return Vec::new(), // Skip invalid UTF-8
        };

        // Append to buffer
        self.buffer.push_str(chunk_str);

        // Extract complete events (separated by \n\n or \r\n\r\n)
        let mut events = Vec::new();

        loop {
            // Find the next complete event (ends with double newline)
            let event_end = self.find_event_boundary();

            match event_end {
                Some(end) => {
                    // Extract the event text
                    let event_text: String = self.buffer.drain(..end).collect();
                    // Skip the double newline delimiter
                    self.skip_delimiter();

                    // Parse the event
                    if let Some(event) = self.parse_event(&event_text) {
                        events.push(event);
                    }
                }
                None => break, // No more complete events
            }
        }

        events
    }

    /// Find the boundary of the next complete event
    fn find_event_boundary(&self) -> Option<usize> {
        // Look for \n\n (Unix) or \r\n\r\n (Windows)
        if let Some(pos) = self.buffer.find("\n\n") {
            return Some(pos);
        }
        if let Some(pos) = self.buffer.find("\r\n\r\n") {
            return Some(pos);
        }
        None
    }

    /// Skip the delimiter after extracting an event
    fn skip_delimiter(&mut self) {
        // Remove leading newlines
        while self.buffer.starts_with('\n') || self.buffer.starts_with('\r') {
            self.buffer.remove(0);
        }
    }

    /// Parse a single SSE event from text
    fn parse_event(&self, text: &str) -> Option<SseEvent> {
        let mut event_type: Option<String> = None;
        let mut data_lines: Vec<&str> = Vec::new();
        let mut id: Option<String> = None;

        for line in text.lines() {
            let line = line.trim_start(); // Handle leading whitespace

            if line.is_empty() {
                continue; // Skip empty lines within event
            }

            // Parse field: value format
            if let Some(value) = line.strip_prefix("event:") {
                event_type = Some(value.trim().to_string());
            } else if let Some(value) = line.strip_prefix("data:") {
                data_lines.push(value.trim_start()); // Keep trailing spaces
            } else if let Some(value) = line.strip_prefix("id:") {
                id = Some(value.trim().to_string());
            } else if let Some(_value) = line.strip_prefix("retry:") {
                // Retry field is typically ignored for LLM streams
            }
            // Lines without a colon or unknown fields are ignored per SSE spec
        }

        // Data is required for a valid event
        if data_lines.is_empty() {
            return None;
        }

        // Join multi-line data with newlines (per SSE spec)
        let data = data_lines.join("\n");

        Some(SseEvent {
            event_type,
            data,
            id,
        })
    }

    /// Clear the internal buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Check if there's remaining data in the buffer
    pub fn has_remaining(&self) -> bool {
        !self.buffer.is_empty()
    }

    /// Get remaining buffered data (for debugging)
    pub fn remaining(&self) -> &str {
        &self.buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_event() {
        let mut decoder = SseDecoder::new();
        let events = decoder.feed(b"data: {\"text\": \"hello\"}\n\n");

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "{\"text\": \"hello\"}");
        assert_eq!(events[0].event_type, None);
    }

    #[test]
    fn test_event_with_type() {
        let mut decoder = SseDecoder::new();
        let events = decoder.feed(b"event: message_start\ndata: {\"type\": \"message\"}\n\n");

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, Some("message_start".to_string()));
        assert_eq!(events[0].data, "{\"type\": \"message\"}");
    }

    #[test]
    fn test_multiple_events() {
        let mut decoder = SseDecoder::new();
        let events = decoder.feed(b"data: first\n\ndata: second\n\n");

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].data, "first");
        assert_eq!(events[1].data, "second");
    }

    #[test]
    fn test_partial_chunks() {
        let mut decoder = SseDecoder::new();

        // First chunk - incomplete
        let events1 = decoder.feed(b"event: content_block_delta\ndata: {\"ty");
        assert_eq!(events1.len(), 0);

        // Second chunk - completes the event
        let events2 = decoder.feed(b"pe\": \"delta\"}\n\n");
        assert_eq!(events2.len(), 1);
        assert_eq!(
            events2[0].event_type,
            Some("content_block_delta".to_string())
        );
        assert_eq!(events2[0].data, "{\"type\": \"delta\"}");
    }

    #[test]
    fn test_multi_line_data() {
        let mut decoder = SseDecoder::new();
        let events = decoder.feed(b"data: line1\ndata: line2\ndata: line3\n\n");

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "line1\nline2\nline3");
    }

    #[test]
    fn test_openai_done_marker() {
        let mut decoder = SseDecoder::new();
        let events = decoder.feed(b"data: [DONE]\n\n");

        assert_eq!(events.len(), 1);
        assert!(events[0].is_done());
    }

    #[test]
    fn test_anthropic_event_sequence() {
        let mut decoder = SseDecoder::new();

        let input = b"event: message_start\n\
            data: {\"type\": \"message_start\"}\n\n\
            event: content_block_start\n\
            data: {\"type\": \"content_block_start\", \"index\": 0}\n\n\
            event: content_block_delta\n\
            data: {\"type\": \"content_block_delta\", \"delta\": {\"text\": \"Hello\"}}\n\n";

        let events = decoder.feed(input);

        assert_eq!(events.len(), 3);
        assert_eq!(events[0].event_type, Some("message_start".to_string()));
        assert_eq!(
            events[1].event_type,
            Some("content_block_start".to_string())
        );
        assert_eq!(
            events[2].event_type,
            Some("content_block_delta".to_string())
        );
    }

    #[test]
    fn test_event_with_id() {
        let mut decoder = SseDecoder::new();
        let events = decoder.feed(b"id: msg_123\nevent: test\ndata: payload\n\n");

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, Some("msg_123".to_string()));
        assert_eq!(events[0].event_type, Some("test".to_string()));
        assert_eq!(events[0].data, "payload");
    }

    #[test]
    fn test_windows_line_endings() {
        let mut decoder = SseDecoder::new();
        let events = decoder.feed(b"event: test\r\ndata: value\r\n\r\n");

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, Some("test".to_string()));
    }

    #[test]
    fn test_empty_data() {
        let mut decoder = SseDecoder::new();
        let events = decoder.feed(b"event: ping\n\n");

        // Event without data is not emitted
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn test_clear_buffer() {
        let mut decoder = SseDecoder::new();
        decoder.feed(b"data: incomplete");
        assert!(decoder.has_remaining());

        decoder.clear();
        assert!(!decoder.has_remaining());
    }
}
