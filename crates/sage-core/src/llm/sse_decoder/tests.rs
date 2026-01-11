//! Tests for SSE decoder

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
    let mut decoder = SseDecoder::new();

    // First chunk ends with incomplete 2-byte UTF-8 (only first byte)
    let chunk1 = b"data: caf\xC3";
    let events1 = decoder.feed(chunk1);
    assert_eq!(events1.len(), 0);
    assert!(decoder.has_incomplete_utf8());
    assert_eq!(decoder.incomplete_utf8_len(), 1);

    // Second chunk completes the UTF-8 character
    let chunk2 = b"\xA9\n\n";
    let events2 = decoder.feed(chunk2);
    assert_eq!(events2.len(), 1);
    assert_eq!(events2[0].data, "café");
    assert!(!decoder.has_incomplete_utf8());
}

#[test]
fn test_utf8_3byte_split_at_1() {
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
    let mut decoder = SseDecoder::new();
    let events = decoder.feed(b"data: hello world\n\n");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].data, "hello world");
    assert!(!decoder.has_incomplete_utf8());
}

#[test]
fn test_utf8_mixed_content() {
    let mut decoder = SseDecoder::new();

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
    let mut decoder = SseDecoder::new();

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

    decoder.feed(b"data: \xE4\xB8");
    assert!(decoder.has_incomplete_utf8());
    assert!(decoder.has_remaining());

    decoder.clear();
    assert!(!decoder.has_incomplete_utf8());
    assert!(!decoder.has_remaining());
}
