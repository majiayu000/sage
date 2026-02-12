//! Recording utilities for session management
//!
//! This module provides helper functions for recording LLM and tool events
//! to the session recorder.

use crate::llm::messages::{LlmMessage, LlmResponse};
use crate::tools::types::ToolSchema;
use crate::trajectory::{SessionRecorder, TokenUsage};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Record a tool call before execution
pub async fn record_tool_call(
    recorder: &Arc<Mutex<SessionRecorder>>,
    tool_name: &str,
    arguments: &serde_json::Value,
) {
    let tool_input = arguments.clone();
    if let Err(e) = recorder
        .lock()
        .await
        .record_tool_call(tool_name, tool_input)
        .await
    {
        tracing::warn!(error = %e, tool_name = %tool_name, "Failed to record tool call");
    }
}

/// Record a tool result after execution
pub async fn record_tool_result(
    recorder: &Arc<Mutex<SessionRecorder>>,
    tool_name: &str,
    success: bool,
    output: Option<String>,
    error: Option<String>,
    execution_time_ms: u64,
) {
    if let Err(e) = recorder
        .lock()
        .await
        .record_tool_result(tool_name, success, output, error, execution_time_ms)
        .await
    {
        tracing::warn!(error = %e, tool_name = %tool_name, "Failed to record tool result");
    }
}

/// Record LLM request before sending
pub async fn record_llm_request(
    recorder: &Arc<Mutex<SessionRecorder>>,
    messages: &[LlmMessage],
    tool_schemas: &[ToolSchema],
) {
    let input_messages: Vec<serde_json::Value> = messages
        .iter()
        .map(|msg| serde_json::to_value(msg).unwrap_or_default())
        .collect();
    let tools_available: Vec<String> = tool_schemas.iter().map(|t| t.name.clone()).collect();
    if let Err(e) = recorder
        .lock()
        .await
        .record_llm_request(input_messages, Some(tools_available))
        .await
    {
        tracing::warn!(error = %e, "Failed to record LLM request");
    }
}

/// Record LLM response after receiving
pub async fn record_llm_response(
    recorder: &Arc<Mutex<SessionRecorder>>,
    llm_response: &LlmResponse,
    model: &str,
) {
    let usage = llm_response.usage.as_ref().map(|u| TokenUsage {
        input_tokens: u.input_tokens,
        output_tokens: u.output_tokens,
        cache_read_tokens: u.cache_read_tokens,
        cache_write_tokens: u.cache_write_tokens,
        cost_estimate: None,
    });
    let tool_calls = if llm_response.tool_calls.is_empty() {
        None
    } else {
        Some(
            llm_response
                .tool_calls
                .iter()
                .map(|tc| serde_json::to_value(tc).unwrap_or_default())
                .collect(),
        )
    };
    if let Err(e) = recorder
        .lock()
        .await
        .record_llm_response(&llm_response.content, model, usage, tool_calls)
        .await
    {
        tracing::warn!(error = %e, "Failed to record LLM response");
    }
}
