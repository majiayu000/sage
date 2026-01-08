//! Server-Sent Events (SSE) decoder for parsing LLM streaming responses
//!
//! This module provides a buffered SSE parser that handles:
//! - Multi-line data fields
//! - Event type prefixes
//! - Incomplete chunks across network boundaries
//! - Incomplete UTF-8 sequences across chunk boundaries
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
///
/// This decoder properly handles:
/// - Incomplete SSE events split across network chunks
/// - Incomplete UTF-8 sequences split across chunk boundaries
#[derive(Debug, Default)]
pub struct SseDecoder {
    /// Buffer for incomplete SSE event data (valid UTF-8 text)
    buffer: String,
    /// Buffer for incomplete UTF-8 byte sequences at chunk boundaries
    /// UTF-8 characters can be 1-4 bytes, and may be split across network chunks
    incomplete_utf8: Vec<u8>,
}

impl SseDecoder {
    /// Create a new SSE decoder
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            incomplete_utf8: Vec::new(),
        }
    }

    /// Feed raw bytes into the decoder and extract complete events
    ///
    /// Returns a vector of complete SSE events parsed from the input.
    /// Incomplete events are buffered for the next call.
    ///
    /// This method properly handles UTF-8 characters split across chunk boundaries
    /// by buffering incomplete byte sequences until the next chunk arrives.
    pub fn feed(&mut self, chunk: &[u8]) -> Vec<SseEvent> {
        // Combine any incomplete UTF-8 bytes from previous chunk with new chunk
        let bytes_to_decode = if self.incomplete_utf8.is_empty() {
            chunk.to_vec()
        } else {
            let mut combined = std::mem::take(&mut self.incomplete_utf8);
            combined.extend_from_slice(chunk);
            combined
        };

        // Find the valid UTF-8 boundary (may not be at the end of bytes_to_decode)
        let (valid_str, remaining_bytes) = Self::decode_utf8_with_remainder(&bytes_to_decode);

        // Store any incomplete UTF-8 bytes for the next chunk
        self.incomplete_utf8 = remaining_bytes;

        // Append valid UTF-8 string to buffer
        self.buffer.push_str(&valid_str);

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

    /// Decode bytes as UTF-8, returning the valid string and any trailing incomplete bytes
    ///
    /// UTF-8 encoding:
    /// - 1 byte:  0xxxxxxx (0x00-0x7F)
    /// - 2 bytes: 110xxxxx 10xxxxxx (first byte 0xC0-0xDF)
    /// - 3 bytes: 1110xxxx 10xxxxxx 10xxxxxx (first byte 0xE0-0xEF)
    /// - 4 bytes: 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx (first byte 0xF0-0xF7)
    ///
    /// Continuation bytes always start with 10xxxxxx (0x80-0xBF)
    fn decode_utf8_with_remainder(bytes: &[u8]) -> (String, Vec<u8>) {
        // Try to decode the entire buffer first (fast path for complete UTF-8)
        if let Ok(s) = std::str::from_utf8(bytes) {
            return (s.to_string(), Vec::new());
        }

        // Find the last valid UTF-8 boundary by scanning backwards
        // We need to find where the last complete UTF-8 character ends
        let mut valid_end = bytes.len();

        // Scan backwards to find potential incomplete sequence at the end
        // Maximum UTF-8 sequence is 4 bytes, so check last 4 bytes
        for i in 1..=4.min(bytes.len()) {
            let pos = bytes.len() - i;
            let byte = bytes[pos];

            // Check if this byte is a UTF-8 start byte (not a continuation byte)
            if !Self::is_continuation_byte(byte) {
                // Found a start byte, check if sequence is complete
                let expected_len = Self::utf8_char_len(byte);
                let actual_remaining = bytes.len() - pos;

                if actual_remaining < expected_len {
                    // Incomplete sequence - split here
                    valid_end = pos;
                }
                break;
            }
        }

        // Decode the valid portion
        let valid_bytes = &bytes[..valid_end];
        let remaining_bytes = bytes[valid_end..].to_vec();

        // The valid portion should now be valid UTF-8
        let valid_str = match std::str::from_utf8(valid_bytes) {
            Ok(s) => s.to_string(),
            Err(e) => {
                // If there's still an error, use the valid_up_to position
                let valid_up_to = e.valid_up_to();
                let s = std::str::from_utf8(&valid_bytes[..valid_up_to])
                    .unwrap_or_default()
                    .to_string();
                // Log a warning for unexpected UTF-8 issues
                tracing::warn!(
                    "Unexpected UTF-8 decoding issue at position {}, recovered {} bytes",
                    valid_up_to,
                    valid_bytes.len() - valid_up_to
                );
                return (s, bytes[valid_up_to..].to_vec());
            }
        };

        (valid_str, remaining_bytes)
    }

    /// Check if a byte is a UTF-8 continuation byte (10xxxxxx)
    #[inline]
    fn is_continuation_byte(byte: u8) -> bool {
        (byte & 0b1100_0000) == 0b1000_0000
    }

    /// Get the expected length of a UTF-8 character from its first byte
    #[inline]
    fn utf8_char_len(first_byte: u8) -> usize {
        if first_byte & 0b1000_0000 == 0 {
            1 // ASCII: 0xxxxxxx
        } else if first_byte & 0b1110_0000 == 0b1100_0000 {
            2 // 110xxxxx
        } else if first_byte & 0b1111_0000 == 0b1110_0000 {
            3 // 1110xxxx
        } else if first_byte & 0b1111_1000 == 0b1111_0000 {
            4 // 11110xxx
        } else {
            1 // Invalid start byte, treat as single byte
        }
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
        self.incomplete_utf8.clear();
    }

    /// Check if there's remaining data in the buffer
    pub fn has_remaining(&self) -> bool {
        !self.buffer.is_empty() || !self.incomplete_utf8.is_empty()
    }

    /// Get remaining buffered data (for debugging)
    pub fn remaining(&self) -> &str {
        &self.buffer
    }

    /// Check if there are incomplete UTF-8 bytes buffered
    pub fn has_incomplete_utf8(&self) -> bool {
        !self.incomplete_utf8.is_empty()
    }

    /// Get the number of incomplete UTF-8 bytes buffered
    pub fn incomplete_utf8_len(&self) -> usize {
        self.incomplete_utf8.len()
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

    // ==================== UTF-8 Boundary Tests ====================

    #[test]
    fn test_utf8_2byte_split() {
        // Chinese character "中" is 3 bytes: E4 B8 AD
        // Let's use a 2-byte character: "é" is C3 A9
        let mut decoder = SseDecoder::new();

        // First chunk ends with incomplete 2-byte UTF-8 (only first byte)
        // "data: caf" + first byte of "é"
        let chunk1 = b"data: caf\xC3";
        let events1 = decoder.feed(chunk1);
        assert_eq!(events1.len(), 0);
        assert!(decoder.has_incomplete_utf8());
        assert_eq!(decoder.incomplete_utf8_len(), 1);

        // Second chunk completes the UTF-8 character
        // Second byte of "é" + "\n\n"
        let chunk2 = b"\xA9\n\n";
        let events2 = decoder.feed(chunk2);
        assert_eq!(events2.len(), 1);
        assert_eq!(events2[0].data, "café");
        assert!(!decoder.has_incomplete_utf8());
    }

    #[test]
    fn test_utf8_3byte_split_at_1() {
        // Chinese character "中" is 3 bytes: E4 B8 AD
        let mut decoder = SseDecoder::new();

        // First chunk: "data: " + first byte of "中"
        let chunk1 = b"data: \xE4";
        let events1 = decoder.feed(chunk1);
        assert_eq!(events1.len(), 0);
        assert!(decoder.has_incomplete_utf8());
        assert_eq!(decoder.incomplete_utf8_len(), 1);

        // Second chunk: remaining 2 bytes of "中" + "文" (E6 96 87) + "\n\n"
        let chunk2 = b"\xB8\xAD\xE6\x96\x87\n\n";
        let events2 = decoder.feed(chunk2);
        assert_eq!(events2.len(), 1);
        assert_eq!(events2[0].data, "中文");
    }

    #[test]
    fn test_utf8_3byte_split_at_2() {
        // Chinese character "中" is 3 bytes: E4 B8 AD
        let mut decoder = SseDecoder::new();

        // First chunk: "data: " + first 2 bytes of "中"
        let chunk1 = b"data: \xE4\xB8";
        let events1 = decoder.feed(chunk1);
        assert_eq!(events1.len(), 0);
        assert!(decoder.has_incomplete_utf8());
        assert_eq!(decoder.incomplete_utf8_len(), 2);

        // Second chunk: last byte of "中" + "\n\n"
        let chunk2 = b"\xAD\n\n";
        let events2 = decoder.feed(chunk2);
        assert_eq!(events2.len(), 1);
        assert_eq!(events2[0].data, "中");
    }

    #[test]
    fn test_utf8_4byte_split() {
        // Emoji "😀" is 4 bytes: F0 9F 98 80
        let mut decoder = SseDecoder::new();

        // First chunk: "data: hi" + first 2 bytes of emoji
        let chunk1 = b"data: hi\xF0\x9F";
        let events1 = decoder.feed(chunk1);
        assert_eq!(events1.len(), 0);
        assert!(decoder.has_incomplete_utf8());
        assert_eq!(decoder.incomplete_utf8_len(), 2);

        // Second chunk: last 2 bytes of emoji + "\n\n"
        let chunk2 = b"\x98\x80\n\n";
        let events2 = decoder.feed(chunk2);
        assert_eq!(events2.len(), 1);
        assert_eq!(events2[0].data, "hi😀");
    }

    #[test]
    fn test_utf8_multiple_incomplete_chunks() {
        // Split a 4-byte emoji across 4 chunks
        // Emoji "🎉" is F0 9F 8E 89
        let mut decoder = SseDecoder::new();

        // Chunk 1: "data: " + first byte
        decoder.feed(b"data: \xF0");
        assert_eq!(decoder.incomplete_utf8_len(), 1);

        // Chunk 2: second byte
        decoder.feed(b"\x9F");
        assert_eq!(decoder.incomplete_utf8_len(), 2);

        // Chunk 3: third byte
        decoder.feed(b"\x8E");
        assert_eq!(decoder.incomplete_utf8_len(), 3);

        // Chunk 4: fourth byte + newlines
        let events = decoder.feed(b"\x89\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "🎉");
        assert!(!decoder.has_incomplete_utf8());
    }

    #[test]
    fn test_utf8_complete_chars_no_buffering() {
        // Pure ASCII - no buffering needed
        let mut decoder = SseDecoder::new();
        let events = decoder.feed(b"data: hello world\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "hello world");
        assert!(!decoder.has_incomplete_utf8());
    }

    #[test]
    fn test_utf8_mixed_content() {
        // Mix of ASCII, 2-byte, 3-byte, and 4-byte chars
        let mut decoder = SseDecoder::new();

        // "Hello 世界 🌍" split across chunks
        // 世 = E4 B8 96, 界 = E7 95 8C, 🌍 = F0 9F 8C 8D
        let chunk1 = b"data: Hello \xE4\xB8";
        decoder.feed(chunk1);
        assert!(decoder.has_incomplete_utf8());

        let chunk2 = b"\x96\xE7\x95\x8C \xF0\x9F";
        decoder.feed(chunk2);
        assert!(decoder.has_incomplete_utf8());

        let chunk3 = b"\x8C\x8D\n\n";
        let events = decoder.feed(chunk3);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "Hello 世界 🌍");
    }

    #[test]
    fn test_utf8_json_with_chinese() {
        // JSON containing Chinese characters, split at UTF-8 boundary
        let mut decoder = SseDecoder::new();

        // {"text": "你好"} where 你 = E4 BD A0, 好 = E5 A5 BD
        let chunk1 = b"data: {\"text\": \"\xE4\xBD";
        decoder.feed(chunk1);

        let chunk2 = b"\xA0\xE5\xA5\xBD\"}\n\n";
        let events = decoder.feed(chunk2);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "{\"text\": \"你好\"}");
    }

    #[test]
    fn test_clear_also_clears_incomplete_utf8() {
        let mut decoder = SseDecoder::new();

        // Feed incomplete UTF-8
        decoder.feed(b"data: \xE4\xB8");
        assert!(decoder.has_incomplete_utf8());
        assert!(decoder.has_remaining());

        // Clear should reset everything
        decoder.clear();
        assert!(!decoder.has_incomplete_utf8());
        assert!(!decoder.has_remaining());
    }
}
