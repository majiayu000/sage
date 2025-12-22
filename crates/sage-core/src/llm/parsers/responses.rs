//! Response parsing for different providers

use crate::error::{SageError, SageResult};
use crate::llm::messages::LLMResponse;
use crate::types::LLMUsage;
use serde_json::Value;
use std::collections::HashMap;

/// Response parser for various providers
pub struct ResponseParser;

impl ResponseParser {
    /// Parse OpenAI response
    pub fn parse_openai(response: Value) -> SageResult<LLMResponse> {
        let choice = response["choices"][0].clone();
        let message = &choice["message"];

        let content = message["content"].as_str().unwrap_or("").to_string();

        let mut tool_calls = Vec::new();
        if let Some(calls) = message["tool_calls"].as_array() {
            for call in calls {
                if let Some(function) = call["function"].as_object() {
                    let tool_call = crate::tools::types::ToolCall {
                        id: call["id"].as_str().unwrap_or("").to_string(),
                        name: function
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        arguments: serde_json::from_str(
                            function
                                .get("arguments")
                                .and_then(|v| v.as_str())
                                .unwrap_or("{}"),
                        )
                        .unwrap_or_default(),
                        call_id: None,
                    };
                    tool_calls.push(tool_call);
                }
            }
        }

        let usage = if let Some(usage_data) = response["usage"].as_object() {
            Some(LLMUsage {
                prompt_tokens: usage_data
                    .get("prompt_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32,
                completion_tokens: usage_data
                    .get("completion_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32,
                total_tokens: usage_data
                    .get("total_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32,
                cost_usd: None,
                cache_creation_input_tokens: None,
                cache_read_input_tokens: None,
            })
        } else {
            None
        };

        Ok(LLMResponse {
            content,
            tool_calls,
            usage,
            model: response["model"].as_str().map(|s| s.to_string()),
            finish_reason: choice["finish_reason"].as_str().map(|s| s.to_string()),
            id: response["id"].as_str().map(|s| s.to_string()),
            metadata: HashMap::new(),
        })
    }

    /// Parse Anthropic response
    ///
    /// Anthropic responses have a content array that may contain:
    /// - {"type": "text", "text": "..."} - Text content
    /// - {"type": "tool_use", "id": "...", "name": "...", "input": {...}} - Tool calls
    ///
    /// When prompt caching is enabled, the usage object may also contain:
    /// - cache_creation_input_tokens: Tokens written to cache
    /// - cache_read_input_tokens: Tokens read from cache
    pub fn parse_anthropic(response: Value) -> SageResult<LLMResponse> {
        let mut content = String::new();
        let mut tool_calls = Vec::new();

        // Iterate through content array to extract text and tool_use blocks
        if let Some(content_array) = response["content"].as_array() {
            for block in content_array {
                match block["type"].as_str() {
                    Some("text") => {
                        // Append text content
                        if let Some(text) = block["text"].as_str() {
                            if !content.is_empty() {
                                content.push('\n');
                            }
                            content.push_str(text);
                        }
                    }
                    Some("tool_use") => {
                        // Parse tool_use block
                        let arguments: HashMap<String, Value> = block["input"]
                            .as_object()
                            .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                            .unwrap_or_default();

                        // Warn if input is empty (likely a proxy issue)
                        if arguments.is_empty() {
                            tracing::warn!(
                                "Tool '{}' received empty input - this may indicate a proxy server issue",
                                block["name"].as_str().unwrap_or("")
                            );
                        }

                        let tool_call = crate::tools::types::ToolCall {
                            id: block["id"].as_str().unwrap_or("").to_string(),
                            name: block["name"].as_str().unwrap_or("").to_string(),
                            arguments,
                            call_id: None,
                        };
                        tool_calls.push(tool_call);
                    }
                    _ => {
                        // Unknown content type, ignore
                    }
                }
            }
        }

        let usage = if let Some(usage_data) = response["usage"].as_object() {
            let input_tokens = usage_data
                .get("input_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let output_tokens = usage_data
                .get("output_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            // Parse cache-related tokens (Anthropic prompt caching)
            let cache_creation_input_tokens = usage_data
                .get("cache_creation_input_tokens")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32);
            let cache_read_input_tokens = usage_data
                .get("cache_read_input_tokens")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32);

            // Log cache metrics if present
            if cache_creation_input_tokens.is_some() || cache_read_input_tokens.is_some() {
                tracing::debug!(
                    "Anthropic cache metrics - created: {:?}, read: {:?}",
                    cache_creation_input_tokens,
                    cache_read_input_tokens
                );
            }

            // Total tokens should include ALL cache-related tokens for accurate reporting:
            // - cache_creation_input_tokens: tokens written to cache (first request)
            // - cache_read_input_tokens: tokens read from cache (subsequent requests)
            // Both represent actual tokens processed by the model
            let cache_tokens = cache_creation_input_tokens.unwrap_or(0) as u64
                + cache_read_input_tokens.unwrap_or(0) as u64;
            let total_input = input_tokens + cache_tokens;

            Some(LLMUsage {
                prompt_tokens: total_input as u32, // Include all cache tokens
                completion_tokens: output_tokens as u32,
                total_tokens: (total_input + output_tokens) as u32,
                cost_usd: None,
                cache_creation_input_tokens,
                cache_read_input_tokens,
            })
        } else {
            None
        };

        Ok(LLMResponse {
            content,
            tool_calls,
            usage,
            model: response["model"].as_str().map(|s| s.to_string()),
            finish_reason: response["stop_reason"].as_str().map(|s| s.to_string()),
            id: response["id"].as_str().map(|s| s.to_string()),
            metadata: HashMap::new(),
        })
    }

    /// Parse Google response
    pub fn parse_google(response: Value, model: &str) -> SageResult<LLMResponse> {
        let candidates = response["candidates"]
            .as_array()
            .ok_or_else(|| SageError::llm("No candidates in Google response"))?;

        if candidates.is_empty() {
            return Err(SageError::llm("Empty candidates array in Google response"));
        }

        let candidate = &candidates[0];
        let content_parts = candidate["content"]["parts"]
            .as_array()
            .ok_or_else(|| SageError::llm("No content parts in Google response"))?;

        let mut content = String::new();
        let mut tool_calls = Vec::new();

        for part in content_parts {
            if let Some(text) = part["text"].as_str() {
                content.push_str(text);
            } else if let Some(function_call) = part["functionCall"].as_object() {
                let tool_name = function_call
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let tool_call = crate::tools::types::ToolCall {
                    id: format!("call_{}", uuid::Uuid::new_v4()),
                    name: tool_name.clone(),
                    arguments: function_call
                        .get("args")
                        .and_then(|v| v.as_object())
                        .map(|args| {
                            let mut map = std::collections::HashMap::new();
                            for (k, v) in args {
                                map.insert(k.clone(), v.clone());
                            }
                            map
                        })
                        .unwrap_or_else(std::collections::HashMap::new),
                    call_id: None,
                };
                tool_calls.push(tool_call);

                // Add some text content when there are tool calls but no text
                if content.is_empty() {
                    content = format!("I'll use the {} tool to help with this task.", tool_name);
                }
            }
        }

        let usage = if let Some(usage_metadata) = response["usageMetadata"].as_object() {
            let prompt_tokens = usage_metadata
                .get("promptTokenCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32;
            let completion_tokens = usage_metadata
                .get("candidatesTokenCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32;
            let total_tokens = usage_metadata
                .get("totalTokenCount")
                .and_then(|v| v.as_u64())
                .unwrap_or((prompt_tokens + completion_tokens) as u64)
                as u32;

            Some(LLMUsage {
                prompt_tokens,
                completion_tokens,
                total_tokens,
                cost_usd: None,
                cache_creation_input_tokens: None,
                cache_read_input_tokens: None,
            })
        } else {
            None
        };

        Ok(LLMResponse {
            content,
            tool_calls,
            usage,
            model: Some(model.to_string()),
            finish_reason: candidate["finishReason"].as_str().map(|s| s.to_string()),
            id: None, // Google doesn't provide request ID in the same way
            metadata: HashMap::new(),
        })
    }
}
