//! Streaming response support for LLM clients

use crate::error::SageResult;
use crate::llm::messages::{LlmMessage, LlmResponse};
use crate::tools::types::ToolSchema;
use crate::types::LlmUsage;
use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::pin::Pin;
use tokio_stream::StreamExt;

/// A chunk of streaming response data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    /// Incremental content
    pub content: Option<String>,
    /// Tool calls (if any)
    pub tool_calls: Option<Vec<crate::tools::ToolCall>>,
    /// Usage information (usually only in the last chunk)
    pub usage: Option<LlmUsage>,
    /// Whether this is the final chunk
    pub is_final: bool,
    /// Finish reason (if final)
    pub finish_reason: Option<String>,
    /// Chunk metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl StreamChunk {
    /// Create a new content chunk
    pub fn content(content: impl Into<String>) -> Self {
        Self {
            content: Some(content.into()),
            tool_calls: None,
            usage: None,
            is_final: false,
            finish_reason: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a final chunk with usage information
    pub fn final_chunk(usage: Option<LlmUsage>, finish_reason: Option<String>) -> Self {
        Self {
            content: None,
            tool_calls: None,
            usage,
            is_final: true,
            finish_reason,
            metadata: HashMap::new(),
        }
    }

    /// Create a tool call chunk
    pub fn tool_calls(tool_calls: Vec<crate::tools::ToolCall>) -> Self {
        Self {
            content: None,
            tool_calls: Some(tool_calls),
            usage: None,
            is_final: false,
            finish_reason: None,
            metadata: HashMap::new(),
        }
    }
}

/// Stream of LLM response chunks
pub type LlmStream = Pin<Box<dyn Stream<Item = SageResult<StreamChunk>> + Send>>;

/// Trait for streaming LLM clients
#[async_trait]
pub trait StreamingLlmClient {
    /// Send a streaming chat completion request
    async fn chat_stream(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LlmStream>;
}

/// Utility functions for working with streams
pub mod stream_utils {
    use super::*;
    use futures::StreamExt;

    /// Collect a stream into a complete response
    pub async fn collect_stream(mut stream: LlmStream) -> SageResult<LlmResponse> {
        let mut content = String::new();
        let mut tool_calls = Vec::new();
        let mut usage = None;
        let mut finish_reason = None;
        let model = None;
        let mut metadata = HashMap::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;

            // Accumulate content
            if let Some(chunk_content) = chunk.content {
                content.push_str(&chunk_content);
            }

            // Collect tool calls
            if let Some(chunk_tool_calls) = chunk.tool_calls {
                tool_calls.extend(chunk_tool_calls);
            }

            // Update usage and finish reason from final chunk
            if chunk.is_final {
                usage = chunk.usage;
                finish_reason = chunk.finish_reason;
            }

            // Merge metadata
            for (key, value) in chunk.metadata {
                metadata.insert(key, value);
            }
        }

        Ok(LlmResponse {
            content,
            tool_calls,
            usage,
            model,
            finish_reason,
            id: None,
            metadata,
        })
    }

    /// Apply a function to each chunk in the stream
    pub fn map_stream<F>(stream: LlmStream, f: F) -> LlmStream
    where
        F: Fn(StreamChunk) -> StreamChunk + Send + 'static,
    {
        Box::pin(stream.map(move |chunk_result| chunk_result.map(|chunk| f(chunk))))
    }

    /// Filter chunks in the stream
    pub fn filter_stream<F>(stream: LlmStream, f: F) -> LlmStream
    where
        F: Fn(&StreamChunk) -> bool + Send + Sync + 'static,
    {
        use std::sync::Arc;
        let f = Arc::new(f);
        Box::pin(stream.filter_map(move |chunk_result| {
            let f = f.clone();
            async move {
                match chunk_result {
                    Ok(chunk) if f(&chunk) => Some(Ok(chunk)),
                    Ok(_) => None, // Filtered out
                    Err(e) => Some(Err(e)),
                }
            }
        }))
    }

    /// Take only content chunks (filter out tool calls and metadata)
    pub fn content_only(stream: LlmStream) -> LlmStream {
        filter_stream(stream, |chunk| chunk.content.is_some())
    }

    /// Buffer chunks and emit them in batches
    pub fn buffer_chunks(stream: LlmStream, buffer_size: usize) -> LlmStream {
        Box::pin(stream.chunks(buffer_size).map(|chunk_batch| {
            // Combine multiple chunks into one
            let mut combined_content = String::new();
            let mut combined_tool_calls = Vec::new();
            let mut final_usage = None;
            let mut final_finish_reason = None;
            let mut is_final = false;
            let mut combined_metadata = HashMap::new();

            for chunk_result in chunk_batch {
                match chunk_result {
                    Ok(chunk) => {
                        if let Some(content) = chunk.content {
                            combined_content.push_str(&content);
                        }
                        if let Some(tool_calls) = chunk.tool_calls {
                            combined_tool_calls.extend(tool_calls);
                        }
                        if chunk.is_final {
                            final_usage = chunk.usage;
                            final_finish_reason = chunk.finish_reason;
                            is_final = true;
                        }
                        for (key, value) in chunk.metadata {
                            combined_metadata.insert(key, value);
                        }
                    }
                    Err(e) => return Err(e),
                }
            }

            Ok(StreamChunk {
                content: if combined_content.is_empty() {
                    None
                } else {
                    Some(combined_content)
                },
                tool_calls: if combined_tool_calls.is_empty() {
                    None
                } else {
                    Some(combined_tool_calls)
                },
                usage: final_usage,
                is_final,
                finish_reason: final_finish_reason,
                metadata: combined_metadata,
            })
        }))
    }

    /// Add timing information to chunks
    pub fn with_timing(stream: LlmStream) -> LlmStream {
        let start_time = std::time::Instant::now();
        Box::pin(stream.map(move |chunk_result| {
            chunk_result.map(|mut chunk| {
                let elapsed = start_time.elapsed();
                chunk.metadata.insert(
                    "elapsed_ms".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(elapsed.as_millis() as u64)),
                );
                chunk
            })
        }))
    }
}

/// Server-Sent Events (SSE) support for web interfaces
pub mod sse {
    use super::*;
    use std::fmt;

    /// SSE event for streaming responses
    #[derive(Debug, Clone)]
    pub struct SseEvent {
        /// Event type
        pub event: Option<String>,
        /// Event data
        pub data: String,
        /// Event ID
        pub id: Option<String>,
        /// Retry interval
        pub retry: Option<u64>,
    }

    impl SseEvent {
        /// Create a new SSE event
        pub fn new(data: impl Into<String>) -> Self {
            Self {
                event: None,
                data: data.into(),
                id: None,
                retry: None,
            }
        }

        /// Set event type
        pub fn with_event(mut self, event: impl Into<String>) -> Self {
            self.event = Some(event.into());
            self
        }

        /// Set event ID
        pub fn with_id(mut self, id: impl Into<String>) -> Self {
            self.id = Some(id.into());
            self
        }

        /// Set retry interval
        pub fn with_retry(mut self, retry: u64) -> Self {
            self.retry = Some(retry);
            self
        }
    }

    impl fmt::Display for SseEvent {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            if let Some(event) = &self.event {
                writeln!(f, "event: {}", event)?;
            }
            if let Some(id) = &self.id {
                writeln!(f, "id: {}", id)?;
            }
            if let Some(retry) = self.retry {
                writeln!(f, "retry: {}", retry)?;
            }
            writeln!(f, "data: {}", self.data)?;
            writeln!(f)?; // Empty line to end the event
            Ok(())
        }
    }

    /// Convert a stream chunk to SSE event
    pub fn chunk_to_sse(chunk: StreamChunk) -> SageResult<SseEvent> {
        let data = serde_json::to_string(&chunk)?;

        let event_type = if chunk.is_final {
            "complete"
        } else if chunk.tool_calls.is_some() {
            "tool_call"
        } else {
            "chunk"
        };

        Ok(SseEvent::new(data).with_event(event_type))
    }

    /// Convert a stream to SSE events
    pub fn stream_to_sse(
        stream: LlmStream,
    ) -> Pin<Box<dyn Stream<Item = SageResult<SseEvent>> + Send>> {
        Box::pin(stream.map(|chunk_result| chunk_result.and_then(chunk_to_sse)))
    }
}
