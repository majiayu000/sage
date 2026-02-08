//! Shared Anthropic-compatible SSE stream parser
//!
//! Used by: anthropic, glm (both use Anthropic's SSE event format)

use crate::error::{SageError, SageResult};
use crate::llm::sse_decoder::SseDecoder;
use crate::llm::streaming::{LlmStream, StreamChunk};
use crate::tools::types::ToolCall;
use crate::types::LlmUsage;
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
    usage: Option<LlmUsage>,
    input_tokens: u32,
    provider_name: String,
}

/// Parse an Anthropic-compatible SSE byte stream into an LlmStream.
pub fn anthropic_sse_stream(
    byte_stream: impl futures::Stream<
            Item = Result<impl AsRef<[u8]> + Send + 'static, reqwest::Error>,
        > + Send
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
    }));

    let stream = byte_stream.flat_map(move |chunk_result| {
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
                Err(e) => futures::stream::iter(vec![Err(SageError::llm(
                    format!("Stream error: {}", e),
                ))]),
            }
        })
    });

    Box::pin(stream.flatten())
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
                    if let Some(input) =
                        usage_data.get("input_tokens").and_then(|v| v.as_u64())
                    {
                        state.input_tokens = input as u32;
                    }
                }
            }
            Some("content_block_start") => {
                let block_type = data["content_block"]["type"].as_str();
                state.current_block_type = block_type.map(String::from);
                if block_type == Some("tool_use") {
                    state.current_block_id =
                        data["content_block"]["id"].as_str().map(String::from);
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
                    let arguments: HashMap<String, Value> =
                        serde_json::from_str(&state.tool_input_buffer)
                            .unwrap_or_default();

                    if arguments.is_empty() && !state.tool_input_buffer.is_empty() {
                        tracing::warn!(
                            "Failed to parse tool input JSON for '{}': buffer was '{}'",
                            state.current_tool_name.as_deref().unwrap_or("unknown"),
                            &state.tool_input_buffer
                        );
                    } else if arguments.is_empty() {
                        tracing::warn!(
                            "Tool '{}' received empty input",
                            state.current_tool_name.as_deref().unwrap_or("unknown")
                        );
                    }

                    state.pending_tool_calls.push(ToolCall {
                        id: state.current_block_id.take().unwrap_or_default(),
                        name: state.current_tool_name.take().unwrap_or_default(),
                        arguments,
                        call_id: None,
                    });
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
            }
            Some("error") => {
                let error_msg = data["error"]["message"]
                    .as_str()
                    .unwrap_or("Unknown error");
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
            .map(|v| v as u32)
            .unwrap_or(state.input_tokens);

        let cache_creation_input_tokens = usage_data
            .get("cache_creation_input_tokens")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);
        let cache_read_input_tokens = usage_data
            .get("cache_read_input_tokens")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);

        state.usage = Some(LlmUsage {
            prompt_tokens: input_tokens,
            completion_tokens: output_tokens as u32,
            total_tokens: input_tokens + output_tokens as u32,
            cost_usd: None,
            cache_creation_input_tokens,
            cache_read_input_tokens,
        });
    }
}
