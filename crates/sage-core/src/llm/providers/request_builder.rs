//! Shared request body builder for OpenAI-compatible providers

use crate::error::SageResult;
use crate::llm::converters::{MessageConverter, ToolConverter};
use crate::llm::messages::LlmMessage;
use crate::llm::provider_types::ModelParameters;
use crate::tools::types::ToolSchema;
use serde_json::{Value, json};

/// Build an OpenAI-compatible chat completion request body.
///
/// Handles the common fields shared by OpenAI, Azure, Doubao, OpenRouter, and Ollama:
/// - `model` (optional, Azure omits it)
/// - `messages` (converted via `MessageConverter::to_openai`)
/// - `stream` (optional)
/// - `max_tokens`, `temperature`, `top_p`
/// - `tools` (converted via `ToolConverter::to_openai`)
pub fn build_openai_request_body(
    model: &str,
    messages: &[LlmMessage],
    tools: Option<&[ToolSchema]>,
    params: &ModelParameters,
    include_model: bool,
    stream: bool,
) -> SageResult<Value> {
    let mut body = json!({
        "messages": MessageConverter::to_openai(messages)?,
    });

    if include_model {
        body["model"] = json!(model);
    }

    if stream {
        body["stream"] = json!(true);
    }

    if let Some(max_tokens) = params.max_tokens {
        body["max_tokens"] = json!(max_tokens);
    }
    if let Some(temperature) = params.temperature {
        body["temperature"] = json!(temperature);
    }
    if let Some(top_p) = params.top_p {
        body["top_p"] = json!(top_p);
    }

    if let Some(tools) = tools {
        if !tools.is_empty() {
            body["tools"] = json!(ToolConverter::to_openai(tools)?);
        }
    }

    Ok(body)
}
