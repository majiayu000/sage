//! Shared OpenAI-compatible SSE stream parser
//!
//! Used by: openai, azure, openrouter, ollama, doubao

use crate::error::SageError;
use crate::llm::streaming::{LlmStream, StreamChunk};
use futures::StreamExt;
use serde_json::Value;

/// Parse an OpenAI-compatible SSE byte stream into an LlmStream.
///
/// All OpenAI-compatible providers use the same SSE format:
/// - Lines prefixed with `data: `
/// - JSON with `choices[0].delta.content`
/// - `[DONE]` termination marker
pub fn openai_sse_stream(
    byte_stream: impl futures::Stream<Item = Result<impl AsRef<[u8]> + Send + 'static, reqwest::Error>>
    + Send
    + 'static,
) -> LlmStream {
    let stream = byte_stream.filter_map(|chunk_result| async move {
        match chunk_result {
            Ok(chunk) => {
                let chunk_str = String::from_utf8_lossy(chunk.as_ref());
                for line in chunk_str.lines() {
                    if let Some(data) = line.strip_prefix("data: ") {
                        if data == "[DONE]" {
                            return Some(Ok(StreamChunk::final_chunk(
                                None,
                                Some("stop".to_string()),
                            )));
                        }

                        if let Ok(json_data) = serde_json::from_str::<Value>(data) {
                            if let Some(choices) = json_data["choices"].as_array() {
                                if let Some(choice) = choices.first() {
                                    if let Some(delta) = choice["delta"].as_object() {
                                        if let Some(content) =
                                            delta.get("content").and_then(|v| v.as_str())
                                        {
                                            return Some(Ok(StreamChunk::content(content)));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                None
            }
            Err(e) => Some(Err(SageError::llm(format!("Stream error: {}", e)))),
        }
    });

    Box::pin(stream)
}
