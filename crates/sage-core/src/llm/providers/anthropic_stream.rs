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
            Err(e) => {
                // Skipping a malformed event silently was the previous
                // behaviour and that masked real bugs: a swallowed
                // `message_stop` looked like a truncated stream
                // (#21), and a swallowed `error` event hid the
                // upstream's reason for failing. Log enough context
                // for an operator to triage without leaking the full
                // payload, then continue so a single corrupted event
                // does not poison the whole stream.
                let preview: String = event.data.chars().take(120).collect();
                tracing::warn!(
                    provider = %state.provider_name,
                    event_type = ?event.event_type,
                    error = %e,
                    data_preview = %preview,
                    "Anthropic SSE event JSON failed to parse; skipping this event"
                );
                continue;
            }
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

#[cfg(test)]
#[path = "anthropic_stream_tests.rs"]
mod tests;
