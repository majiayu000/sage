//! Server-Sent Events (SSE) decoder for parsing LLM streaming responses
//!
//! This module provides a buffered SSE parser that handles:
//! - Multi-line data fields
//! - Event type prefixes
//! - Incomplete chunks across network boundaries
//! - Incomplete UTF-8 sequences across chunk boundaries
//! - Both OpenAI and Anthropic SSE formats

mod event;

pub use event::SseEvent;

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
    pub fn feed(&mut self, chunk: &[u8]) -> Vec<SseEvent> {
        // Combine any incomplete UTF-8 bytes from previous chunk with new chunk
        let bytes_to_decode = if self.incomplete_utf8.is_empty() {
            chunk.to_vec()
        } else {
            let mut combined = std::mem::take(&mut self.incomplete_utf8);
            combined.extend_from_slice(chunk);
            combined
        };

        // Find the valid UTF-8 boundary
        let (valid_str, remaining_bytes) = Self::decode_utf8_with_remainder(&bytes_to_decode);

        // Store any incomplete UTF-8 bytes for the next chunk
        self.incomplete_utf8 = remaining_bytes;

        // Append valid UTF-8 string to buffer
        self.buffer.push_str(&valid_str);

        // Extract complete events
        let mut events = Vec::new();

        loop {
            let event_end = self.find_event_boundary();

            match event_end {
                Some(end) => {
                    let event_text: String = self.buffer.drain(..end).collect();
                    self.skip_delimiter();

                    if let Some(event) = self.parse_event(&event_text) {
                        events.push(event);
                    }
                }
                None => break,
            }
        }

        events
    }

    /// Decode bytes as UTF-8, returning the valid string and any trailing incomplete bytes
    fn decode_utf8_with_remainder(bytes: &[u8]) -> (String, Vec<u8>) {
        // Fast path for complete UTF-8
        if let Ok(s) = std::str::from_utf8(bytes) {
            return (s.to_string(), Vec::new());
        }

        // Find the last valid UTF-8 boundary by scanning backwards
        let mut valid_end = bytes.len();

        for i in 1..=4.min(bytes.len()) {
            let pos = bytes.len() - i;
            let byte = bytes[pos];

            if !Self::is_continuation_byte(byte) {
                let expected_len = Self::utf8_char_len(byte);
                let actual_remaining = bytes.len() - pos;

                if actual_remaining < expected_len {
                    valid_end = pos;
                }
                break;
            }
        }

        let valid_bytes = &bytes[..valid_end];
        let remaining_bytes = bytes[valid_end..].to_vec();

        let valid_str = match std::str::from_utf8(valid_bytes) {
            Ok(s) => s.to_string(),
            Err(e) => {
                let valid_up_to = e.valid_up_to();
                let s = std::str::from_utf8(&valid_bytes[..valid_up_to])
                    .unwrap_or_default()
                    .to_string();
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
            1
        } else if first_byte & 0b1110_0000 == 0b1100_0000 {
            2
        } else if first_byte & 0b1111_0000 == 0b1110_0000 {
            3
        } else if first_byte & 0b1111_1000 == 0b1111_0000 {
            4
        } else {
            1
        }
    }

    /// Find the boundary of the next complete event
    fn find_event_boundary(&self) -> Option<usize> {
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
            let line = line.trim_start();

            if line.is_empty() {
                continue;
            }

            if let Some(value) = line.strip_prefix("event:") {
                event_type = Some(value.trim().to_string());
            } else if let Some(value) = line.strip_prefix("data:") {
                data_lines.push(value.trim_start());
            } else if let Some(value) = line.strip_prefix("id:") {
                id = Some(value.trim().to_string());
            }
            // retry: and unknown fields are ignored per SSE spec
        }

        if data_lines.is_empty() {
            return None;
        }

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
mod tests;
