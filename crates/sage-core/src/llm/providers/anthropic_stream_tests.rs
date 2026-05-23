//! Tests for `anthropic_stream`.
//!
//! Split from the parent file to keep it under the RS-SIZE-01 500-line
//! ceiling. Imports use `super::*` to reach internals (`StreamState`,
//! `process_events`, `finalize_stream`, `flush_on_transport_error`).

use super::*;
use crate::llm::sse_decoder::SseEvent;

fn fresh_state() -> StreamState {
    StreamState {
        decoder: SseDecoder::new(),
        current_block_type: None,
        current_block_id: None,
        current_tool_name: None,
        tool_input_buffer: String::new(),
        pending_tool_calls: Vec::new(),
        stop_reason: None,
        usage: None,
        input_tokens: 0,
        provider_name: "anthropic".to_string(),
        saw_message_stop: false,
    }
}

fn run_events(events: Vec<SseEvent>) -> (StreamState, Vec<SageResult<StreamChunk>>) {
    let mut state = fresh_state();
    let mut chunks = Vec::new();
    process_events(&mut state, events, &mut chunks);
    (state, chunks)
}

fn ok_chunks(chunks: &[SageResult<StreamChunk>]) -> usize {
    chunks.iter().filter(|c| c.is_ok()).count()
}

#[test]
fn valid_tool_use_dispatches_with_parsed_arguments() {
    let events = vec![
        SseEvent::with_type(
            "content_block_start",
            r#"{"index":0,"content_block":{"type":"tool_use","id":"toolu_1","name":"shell"}}"#,
        ),
        SseEvent::with_type(
            "content_block_delta",
            r#"{"index":0,"delta":{"type":"input_json_delta","partial_json":"{\"cmd\":\"ls\"}"}}"#,
        ),
        SseEvent::with_type("content_block_stop", r#"{"index":0}"#),
        SseEvent::with_type("message_stop", r#"{}"#),
    ];

    let (_, chunks) = run_events(events);
    // Errors must NOT be emitted on the happy path.
    assert!(
        chunks.iter().all(|c| c.is_ok()),
        "no errors expected; got: {chunks:?}"
    );
    // tool_calls + final_chunk == 2 ok chunks.
    assert_eq!(ok_chunks(&chunks), 2);
}

#[test]
fn unparseable_tool_input_emits_error_and_does_not_dispatch() {
    // Buffer ends mid-object — broken JSON. Before the fix this would
    // silently dispatch the tool with empty arguments. After the fix
    // it must emit an Err and not push anything into pending_tool_calls.
    let events = vec![
        SseEvent::with_type(
            "content_block_start",
            r#"{"index":0,"content_block":{"type":"tool_use","id":"toolu_2","name":"shell"}}"#,
        ),
        SseEvent::with_type(
            "content_block_delta",
            r#"{"index":0,"delta":{"type":"input_json_delta","partial_json":"{\"cmd\":\"ls"}}"#,
        ),
        // Note: NO closing partial — buffer is `{"cmd":"ls`.
        SseEvent::with_type("content_block_stop", r#"{"index":0}"#),
    ];

    let (state, chunks) = run_events(events);
    assert!(
        state.pending_tool_calls.is_empty(),
        "unparseable tool input must not produce a pending ToolCall, got: {:?}",
        state.pending_tool_calls
    );
    let err = chunks
        .iter()
        .find_map(|c| c.as_ref().err())
        .expect("must emit a typed error chunk on parse failure");
    let msg = err.to_string();
    assert!(
        msg.contains("toolu_2"),
        "error must include the tool_use id: {msg}"
    );
    assert!(
        msg.contains("shell"),
        "error must include the tool name: {msg}"
    );
}

#[test]
fn finalize_stream_is_noop_after_message_stop() {
    // After a normal `message_stop`, finalize_stream must NOT
    // double-emit. saw_message_stop is set true by the message_stop
    // arm of process_events, and finalize_stream short-circuits on it.
    let mut state = fresh_state();
    state.saw_message_stop = true;
    state.pending_tool_calls.push(ToolCall {
        id: "x".into(),
        name: "y".into(),
        arguments: HashMap::new(),
        call_id: None,
    }); // intentionally non-empty: the flag must take precedence
    let mut chunks = Vec::new();
    finalize_stream(&mut state, &mut chunks);
    assert!(chunks.is_empty(), "no double-emit after message_stop");
}

#[test]
fn finalize_stream_flushes_pending_tool_calls_on_truncation() {
    // The model emitted complete content_block_stop events for tool
    // blocks but the upstream byte stream ended *before* the parser
    // saw `message_stop` (proxy cut, client timeout, …). Without
    // the EOF flush the orchestrator would observe an empty turn
    // and silently drop the model's tool calls.
    let events = vec![
        SseEvent::with_type(
            "content_block_start",
            r#"{"index":0,"content_block":{"type":"tool_use","id":"toolu_x","name":"shell"}}"#,
        ),
        SseEvent::with_type(
            "content_block_delta",
            r#"{"index":0,"delta":{"type":"input_json_delta","partial_json":"{\"cmd\":\"ls\"}"}}"#,
        ),
        SseEvent::with_type("content_block_stop", r#"{"index":0}"#),
        // Intentionally NO message_stop here — simulating a
        // truncated stream.
    ];
    let (mut state, chunks) = run_events(events);
    // Mid-stream: process_events should NOT have emitted tool_calls
    // yet (that is the message_stop arm's job).
    assert_eq!(
        ok_chunks(&chunks),
        0,
        "no terminal emission expected before message_stop / EOF flush"
    );
    assert_eq!(state.pending_tool_calls.len(), 1);

    // Now simulate the byte stream ending without message_stop.
    let mut tail = Vec::new();
    finalize_stream(&mut state, &mut tail);
    assert!(state.pending_tool_calls.is_empty(), "must drain on flush");
    assert_eq!(
        tail.len(),
        2,
        "expected tool_calls + final_chunk on truncation flush"
    );
    assert!(tail.iter().all(|c| c.is_ok()));
    // First chunk must carry the tool call.
    match &tail[0] {
        Ok(c) if c.tool_calls.as_ref().is_some_and(|tcs| !tcs.is_empty()) => {}
        other => panic!("first flushed chunk must be tool_calls, got {other:?}"),
    }
}

#[test]
fn flush_on_transport_error_drains_pending_tool_calls_before_error() {
    // Codex P2 verification: when the byte stream yields a
    // transport-level error after complete `content_block_stop`
    // events but before `message_stop`, the chained EOF flush is
    // never polled (consumers short-circuit on `?`). The fix
    // funnels the same drain through this helper so pending tool
    // calls are emitted before the terminal `Err`.
    let events = vec![
        SseEvent::with_type(
            "content_block_start",
            r#"{"index":0,"content_block":{"type":"tool_use","id":"toolu_t","name":"shell"}}"#,
        ),
        SseEvent::with_type(
            "content_block_delta",
            r#"{"index":0,"delta":{"type":"input_json_delta","partial_json":"{\"cmd\":\"ls\"}"}}"#,
        ),
        SseEvent::with_type("content_block_stop", r#"{"index":0}"#),
        // Intentionally no message_stop — the transport error
        // arrives next.
    ];
    let (mut state, mid) = run_events(events);
    assert_eq!(
        ok_chunks(&mid),
        0,
        "no terminal emission expected mid-stream"
    );
    assert_eq!(state.pending_tool_calls.len(), 1);

    let items = flush_on_transport_error(&mut state, "connection reset".to_string());
    // First emitted item is the tool_calls chunk (Ok), second is
    // the typed error (Err). Order matters: consumers accumulate
    // tool calls before hitting `?` on the Err.
    assert_eq!(items.len(), 2, "expected tool_calls + Err, got {items:?}");
    match &items[0] {
        Ok(c) if c.tool_calls.as_ref().is_some_and(|tcs| !tcs.is_empty()) => {}
        other => panic!("first item must be the tool_calls flush, got {other:?}"),
    }
    let err = items[1]
        .as_ref()
        .expect_err("second item must be the typed error");
    assert!(err.to_string().contains("connection reset"));

    // saw_message_stop must be set so a later EOF flush no-ops.
    assert!(state.saw_message_stop);
    assert!(state.pending_tool_calls.is_empty());

    // A subsequent finalize_stream call must be a true no-op now.
    let mut tail = Vec::new();
    finalize_stream(&mut state, &mut tail);
    assert!(
        tail.is_empty(),
        "EOF flush must no-op after transport-error flush"
    );
}

#[test]
fn finalize_stream_errors_when_tool_use_block_was_open() {
    // Stream ended after `content_block_start` for a tool_use block
    // but before `content_block_stop`. Previously this emitted a
    // clean `final_chunk`, silently dropping the partial tool call.
    // After the fix it must emit a typed error.
    let events = vec![
        SseEvent::with_type(
            "content_block_start",
            r#"{"index":0,"content_block":{"type":"tool_use","id":"toolu_p","name":"shell"}}"#,
        ),
        SseEvent::with_type(
            "content_block_delta",
            r#"{"index":0,"delta":{"type":"input_json_delta","partial_json":"{\"cmd\":\""}}"#,
        ),
        // Intentionally NO content_block_stop and NO message_stop.
    ];
    let (mut state, chunks) = run_events(events);
    // Mid-stream: nothing terminal yet.
    assert_eq!(ok_chunks(&chunks), 0);
    assert_eq!(state.current_block_type.as_deref(), Some("tool_use"));

    let mut tail = Vec::new();
    finalize_stream(&mut state, &mut tail);
    // We expect exactly one Err chunk and zero Ok chunks: a partial
    // tool_use block must NOT be emitted as a successful tool call.
    let errs: Vec<&SageError> = tail.iter().filter_map(|c| c.as_ref().err()).collect();
    let oks = tail.iter().filter(|c| c.is_ok()).count();
    assert_eq!(
        oks, 0,
        "must not emit a successful chunk for a partial tool block"
    );
    assert_eq!(errs.len(), 1, "must emit exactly one typed error: {tail:?}");
    let msg = errs[0].to_string();
    assert!(
        msg.contains("toolu_p"),
        "error must include the tool_use id: {msg}"
    );
    assert!(
        msg.contains("shell"),
        "error must include the tool name: {msg}"
    );
    assert!(
        state.saw_message_stop,
        "saw_message_stop must be set so subsequent flushes no-op"
    );
    assert_eq!(state.current_block_type, None);
    assert!(state.tool_input_buffer.is_empty());
}

#[test]
fn finalize_stream_emits_only_final_chunk_when_nothing_pending() {
    // A truly empty completion (no message_stop, no pending tool
    // calls) should emit exactly one final chunk so the orchestrator
    // observes a clean termination instead of an empty stream.
    let mut state = fresh_state();
    let mut chunks = Vec::new();
    finalize_stream(&mut state, &mut chunks);
    assert_eq!(chunks.len(), 1);
    assert!(chunks[0].is_ok());
    // Idempotency: a second call after the first is a no-op.
    let mut more = Vec::new();
    finalize_stream(&mut state, &mut more);
    assert!(more.is_empty());
}

#[test]
fn empty_tool_input_buffer_is_a_legitimate_no_arg_call() {
    // Anthropic emits an empty tool_use block when the model genuinely
    // calls a no-arg tool. Empty buffer should be treated as `{}`, not
    // a parse error.
    let events = vec![
        SseEvent::with_type(
            "content_block_start",
            r#"{"index":0,"content_block":{"type":"tool_use","id":"toolu_3","name":"now"}}"#,
        ),
        SseEvent::with_type("content_block_stop", r#"{"index":0}"#),
        SseEvent::with_type("message_stop", r#"{}"#),
    ];

    let (_, chunks) = run_events(events);
    assert!(
        chunks.iter().all(|c| c.is_ok()),
        "empty buffer must NOT error; got: {chunks:?}"
    );
    // tool_calls + final_chunk
    assert_eq!(ok_chunks(&chunks), 2);
}

#[test]
fn malformed_event_is_skipped_and_does_not_poison_following_events() {
    // A single malformed event must be logged-and-skipped; the
    // subsequent valid event must still be processed normally so
    // one corrupted event does not poison the rest of the stream.
    // Before the fix the parse failure was silently `continue`d and
    // there was no signal at all that an event had been dropped.
    let events = vec![
        // Garbage payload; bare `not-json` cannot be parsed by serde_json.
        SseEvent::with_type("content_block_start", "not-json"),
        // A subsequent valid text_delta event must still produce a
        // content chunk — proving the parser recovered.
        SseEvent::with_type(
            "content_block_start",
            r#"{"index":0,"content_block":{"type":"text"}}"#,
        ),
        SseEvent::with_type(
            "content_block_delta",
            r#"{"index":0,"delta":{"type":"text_delta","text":"hello"}}"#,
        ),
        SseEvent::with_type("message_stop", r#"{}"#),
    ];

    let (_, chunks) = run_events(events);
    // The malformed event must NOT produce an Err chunk; parse
    // failures are observability events (warn log) rather than
    // stream errors so a single corrupted event cannot poison the
    // whole stream.
    assert!(
        chunks.iter().all(|c| c.is_ok()),
        "parse failures must not surface as Err chunks: {chunks:?}"
    );
    // We expect: content("hello") + final_chunk = 2 ok chunks. The
    // earlier malformed event contributed 0. If recovery were
    // broken, the second `content_block_start` would also fail to
    // be processed and we'd see only the final_chunk.
    assert_eq!(
        ok_chunks(&chunks),
        2,
        "valid events after a malformed one must still be processed: {chunks:?}"
    );
}

#[test]
fn malformed_event_does_not_eat_subsequent_message_stop() {
    // Specifically guard against the failure mode in #21: a
    // swallowed `message_stop` made the stream look truncated. Now
    // that we log+continue, message_stop on the line after a
    // malformed event must still set saw_message_stop and emit
    // final_chunk during normal processing, so the EOF flush is a
    // no-op afterwards.
    let events = vec![
        SseEvent::with_type("message_start", "garbage{"),
        SseEvent::with_type("message_stop", r#"{}"#),
    ];
    let (state, chunks) = run_events(events);
    assert!(state.saw_message_stop);
    // exactly one final_chunk emitted by the message_stop arm.
    assert_eq!(ok_chunks(&chunks), 1);
}
