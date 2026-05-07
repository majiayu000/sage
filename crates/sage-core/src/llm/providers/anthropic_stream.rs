//! Shared Anthropic-compatible SSE stream parser
//!
//! Used by: anthropic, glm (both use Anthropic's SSE event format)

use crate::error::{SageError, SageResult};
use crate::llm::sse_decoder::SseDecoder;
use crate::llm::streaming::{LlmStream, StreamChunk};
use crate::tools::types::ToolCall;
use crate::types::TokenUsage;
use futures::StreamExt;
use serde_json::Value;
use std::collections::HashMap;

struct StreamState {
    decoder: SseDecoder,
    current_block_type: Option<String>,
    current_block_id: Option<String>,
    current_tool_name: Option<String>,
    tool_input_buffer: String,
    pending_tool_calls: Vec<ToolCall>,
    stop_reason: Option<String>,
    usage: Option<TokenUsage>,
    input_tokens: u64,
    provider_name: String,
    /// Tracks whether the parser already emitted the terminal
    /// `message_stop` flush. Used by `finalize_stream` to avoid emitting
    /// a duplicate final chunk when the stream ends cleanly.
    saw_message_stop: bool,
}

/// Parse an Anthropic-compatible SSE byte stream into an LlmStream.
pub fn anthropic_sse_stream(
    byte_stream: impl futures::Stream<Item = Result<impl AsRef<[u8]> + Send + 'static, reqwest::Error>>
    + Send
    + 'static,
    provider_name: &str,
) -> LlmStream {
    let state = std::sync::Arc::new(tokio::sync::Mutex::new(StreamState {
        decoder: SseDecoder::new(),
        current_block_type: None,
        current_block_id: None,
        current_tool_name: None,
        tool_input_buffer: String::new(),
        pending_tool_calls: Vec::new(),
        stop_reason: None,
        usage: None,
        input_tokens: 0,
        provider_name: provider_name.to_string(),
        saw_message_stop: false,
    }));

    let stream = byte_stream.flat_map({
        let state = state.clone();
        move |chunk_result| {
            let state = state.clone();
            futures::stream::once(async move {
                match chunk_result {
                    Ok(chunk) => {
                        let mut state = state.lock().await;
                        let events = state.decoder.feed(chunk.as_ref());
                        let mut chunks: Vec<SageResult<StreamChunk>> = Vec::new();
                        process_events(&mut state, events, &mut chunks);
                        futures::stream::iter(chunks)
                    }
                    Err(e) => {
                        let mut state = state.lock().await;
                        let items = flush_on_transport_error(&mut state, e.to_string());
                        futures::stream::iter(items)
                    }
                }
            })
        }
    });

    // After the byte stream ends, run a synthetic terminal flush. If the
    // upstream emitted `message_stop` we already pushed the final chunk
    // and `finalize_stream` is a no-op. If the upstream truncated before
    // `message_stop` (proxy cut, client timeout, network drop) we flush
    // any pending tool calls instead of dropping them silently.
    let flush_state = state.clone();
    let flush = futures::stream::once(async move {
        let mut state = flush_state.lock().await;
        let mut chunks: Vec<SageResult<StreamChunk>> = Vec::new();
        finalize_stream(&mut state, &mut chunks);
        futures::stream::iter(chunks)
    })
    .flatten();

    Box::pin(stream.flatten().chain(flush))
}

/// Emit any state that did not get flushed by an explicit `message_stop`.
///
/// Called once after the upstream byte stream terminates. If
/// `saw_message_stop` is true the parser already emitted everything
/// during the regular event loop, so this is a no-op. Otherwise we
/// flush pending tool calls (if any) and then a final chunk so the
/// caller sees a terminal event instead of an empty turn.
fn finalize_stream(state: &mut StreamState, chunks: &mut Vec<SageResult<StreamChunk>>) {
    if state.saw_message_stop {
        return;
    }

    // The stream ended while a `tool_use` block was still open: we saw
    // `content_block_start` but never `content_block_stop`, so the model
    // had begun emitting tool input but did not finish. Emitting a clean
    // `final_chunk` here would silently drop the partial tool call and
    // leave the orchestrator with no signal at all. Surface it as a
    // typed error and clear the buffer so a subsequent recovery can
    // proceed from a clean slate.
    if state.current_block_type.as_deref() == Some("tool_use") {
        let tool_name = state
            .current_tool_name
            .take()
            .unwrap_or_else(|| "unknown".to_string());
        let block_id = state.current_block_id.take().unwrap_or_default();
        tracing::error!(
            provider = %state.provider_name,
            tool = %tool_name,
            tool_call_id = %block_id,
            buffer_len = state.tool_input_buffer.len(),
            "Stream ended mid-`tool_use` block (no `content_block_stop`); \
             refusing to emit a partial tool call"
        );
        chunks.push(Err(SageError::llm(format!(
            "{} stream ended mid-tool_use block: tool '{}' (id={}) had no \
             `content_block_stop`",
            state.provider_name, tool_name, block_id
        ))));
        state.tool_input_buffer.clear();
        state.current_block_type = None;
        state.saw_message_stop = true;
        return;
    }

    if !state.pending_tool_calls.is_empty() {
        tracing::warn!(
            provider = %state.provider_name,
            pending = state.pending_tool_calls.len(),
            "Stream ended without `message_stop`; flushing pending tool calls so the \
             orchestrator does not see an empty turn"
        );
        let tool_calls = std::mem::take(&mut state.pending_tool_calls);
        chunks.push(Ok(StreamChunk::tool_calls(tool_calls)));
    }
    chunks.push(Ok(StreamChunk::final_chunk(
        state.usage.take(),
        state.stop_reason.take(),
    )));
    state.saw_message_stop = true;
}

/// Drain any pending state and append a terminal error in response to
/// a transport-level failure on the byte stream (proxy cut, client
/// timeout, network drop).
///
/// Downstream consumers short-circuit on the first `Err` chunk via
/// `chunk_result?` / `return Err(e)`, so the post-stream
/// `chain(flush)` in [`anthropic_sse_stream`] is never polled when an
/// `Err` is yielded mid-stream. Without this drain, any
/// `pending_tool_calls` that the parser had already accumulated would
/// be silently dropped — the exact failure mode issue #21 was filed
/// against, in its transport-error variant.
///
/// The returned `Vec` always ends with the `Err` so consumers still
/// observe the terminal failure; tool calls (if any) are emitted
/// first so the orchestrator can record what the model managed to
/// produce before the connection broke. We also flip
/// `saw_message_stop` so that if the chain flush ever does get
/// polled (e.g. because a different consumer pattern continues past
/// the error), it will be a no-op.
fn flush_on_transport_error(
    state: &mut StreamState,
    error_message: String,
) -> Vec<SageResult<StreamChunk>> {
    let mut items: Vec<SageResult<StreamChunk>> = Vec::new();
    if !state.pending_tool_calls.is_empty() {
        tracing::warn!(
            provider = %state.provider_name,
            pending = state.pending_tool_calls.len(),
            "Transport error after complete tool blocks; \
             flushing pending tool calls before surfacing error"
        );
        let tool_calls = std::mem::take(&mut state.pending_tool_calls);
        items.push(Ok(StreamChunk::tool_calls(tool_calls)));
    }
    state.saw_message_stop = true;
    items.push(Err(SageError::llm(format!(
        "{} stream error: {}",
        state.provider_name, error_message
    ))));
    items
}

fn process_events(
    state: &mut StreamState,
    events: Vec<crate::llm::sse_decoder::SseEvent>,
    chunks: &mut Vec<SageResult<StreamChunk>>,
) {
    for event in events {
        let data: Value = match serde_json::from_str(&event.data) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let event_type = event
            .event_type
            .as_deref()
            .or_else(|| data["type"].as_str());

        match event_type {
            Some("message_start") => {
                if let Some(usage_data) = data["message"]["usage"].as_object() {
                    if let Some(input) = usage_data.get("input_tokens").and_then(|v| v.as_u64()) {
                        state.input_tokens = input;
                    }
                }
            }
            Some("content_block_start") => {
                let block_type = data["content_block"]["type"].as_str();
                state.current_block_type = block_type.map(String::from);
                if block_type == Some("tool_use") {
                    state.current_block_id = data["content_block"]["id"].as_str().map(String::from);
                    state.current_tool_name =
                        data["content_block"]["name"].as_str().map(String::from);
                    state.tool_input_buffer.clear();
                }
            }
            Some("content_block_delta") => {
                let delta = &data["delta"];
                match delta["type"].as_str() {
                    Some("text_delta") => {
                        if let Some(text) = delta["text"].as_str() {
                            if !text.is_empty() {
                                chunks.push(Ok(StreamChunk::content(text)));
                            }
                        }
                    }
                    Some("input_json_delta") => {
                        if let Some(partial) = delta["partial_json"].as_str() {
                            state.tool_input_buffer.push_str(partial);
                        }
                    }
                    _ => {}
                }
            }
            Some("content_block_stop") => {
                if state.current_block_type.as_deref() == Some("tool_use") {
                    // The buffer accumulated from `input_json_delta` events
                    // is supposed to be a complete JSON object. If it isn't,
                    // dispatching the tool with empty arguments produces a
                    // wrong result that looks like a normal tool failure to
                    // the caller. Surface a typed error instead so the
                    // orchestrator can decide whether to retry.
                    let parse: Result<HashMap<String, Value>, _> =
                        if state.tool_input_buffer.is_empty() {
                            // Anthropic emits an empty tool_use block when
                            // the model genuinely calls a no-arg tool.
                            // That's a legitimate empty-args ToolCall, not
                            // a parse failure.
                            Ok(HashMap::new())
                        } else {
                            serde_json::from_str(&state.tool_input_buffer)
                        };

                    match parse {
                        Ok(arguments) => {
                            state.pending_tool_calls.push(ToolCall {
                                id: state.current_block_id.take().unwrap_or_default(),
                                name: state.current_tool_name.take().unwrap_or_default(),
                                arguments,
                                call_id: None,
                            });
                        }
                        Err(e) => {
                            let tool = state
                                .current_tool_name
                                .take()
                                .unwrap_or_else(|| "unknown".to_string());
                            let id = state.current_block_id.take().unwrap_or_default();
                            tracing::error!(
                                tool = %tool,
                                tool_call_id = %id,
                                buffer = %state.tool_input_buffer,
                                error = %e,
                                "Anthropic stream: tool input JSON failed to parse; \
                                 refusing to dispatch tool call with empty arguments"
                            );
                            chunks.push(Err(SageError::llm(format!(
                                "{} stream: tool '{}' (id={}) emitted unparseable input JSON: \
                                 {} (buffer: {:?})",
                                state.provider_name, tool, id, e, state.tool_input_buffer
                            ))));
                        }
                    }
                    state.tool_input_buffer.clear();
                }
                state.current_block_type = None;
            }
            Some("message_delta") => {
                handle_message_delta(state, &data);
            }
            Some("message_stop") => {
                if !state.pending_tool_calls.is_empty() {
                    let tool_calls = std::mem::take(&mut state.pending_tool_calls);
                    chunks.push(Ok(StreamChunk::tool_calls(tool_calls)));
                }
                chunks.push(Ok(StreamChunk::final_chunk(
                    state.usage.take(),
                    state.stop_reason.take(),
                )));
                state.saw_message_stop = true;
            }
            Some("error") => {
                let error_msg = data["error"]["message"].as_str().unwrap_or("Unknown error");
                chunks.push(Err(SageError::llm(format!(
                    "{} stream error: {}",
                    state.provider_name, error_msg
                ))));
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
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
            .err()
            .expect("second item must be the typed error");
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
}

fn handle_message_delta(state: &mut StreamState, data: &Value) {
    if let Some(stop_reason) = data["delta"]["stop_reason"].as_str() {
        state.stop_reason = Some(stop_reason.to_string());
    }

    if let Some(usage_data) = data["usage"].as_object() {
        let output_tokens = usage_data
            .get("output_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let input_tokens = usage_data
            .get("input_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(state.input_tokens);

        let cache_write_tokens = usage_data
            .get("cache_creation_input_tokens")
            .and_then(|v| v.as_u64());
        let cache_read_tokens = usage_data
            .get("cache_read_input_tokens")
            .and_then(|v| v.as_u64());

        state.usage = Some(TokenUsage {
            input_tokens,
            output_tokens,
            cache_write_tokens,
            cache_read_tokens,
            cost_estimate: None,
        });
    }
}
