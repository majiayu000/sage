//! Message format conversion for different providers

use crate::error::SageResult;
use crate::llm::messages::{LlmMessage, MessageRole};
use serde_json::{Value, json};

/// Message format converter
pub struct MessageConverter;

impl MessageConverter {
    /// Convert messages for OpenAI format
    pub fn to_openai(messages: &[LlmMessage]) -> SageResult<Vec<Value>> {
        let mut converted = Vec::new();

        for message in messages {
            let mut msg = json!({
                "role": message.role.to_string(),
                "content": message.content
            });

            // Convert tool_calls to OpenAI format
            if let Some(tool_calls) = &message.tool_calls {
                let openai_tool_calls: Vec<Value> = tool_calls
                    .iter()
                    .map(|tc| {
                        json!({
                            "id": tc.id,
                            "type": "function",
                            "function": {
                                "name": tc.name,
                                "arguments": serde_json::to_string(&tc.arguments).unwrap_or_default()
                            }
                        })
                    })
                    .collect();
                msg["tool_calls"] = json!(openai_tool_calls);
            }

            if let Some(tool_call_id) = &message.tool_call_id {
                msg["tool_call_id"] = json!(tool_call_id);
            }

            if let Some(name) = &message.name {
                msg["name"] = json!(name);
            }

            converted.push(msg);
        }

        Ok(converted)
    }

    /// Convert messages for Anthropic format
    ///
    /// When caching is enabled, adds `cache_control` to ONLY the last 2 messages
    /// (Claude Code style). Anthropic API allows max 4 cache_control blocks total:
    /// - 1 for system prompt
    /// - 1 for tools (last tool)
    /// - 2 for messages (last 2 messages only)
    ///
    /// This allows efficient caching with 90% cost savings on cache reads.
    pub fn to_anthropic(messages: &[LlmMessage], enable_caching: bool) -> SageResult<Vec<Value>> {
        let mut converted = Vec::new();

        // Filter out system messages first to get accurate count
        let non_system_messages: Vec<_> = messages
            .iter()
            .filter(|m| m.role != MessageRole::System)
            .collect();

        let total_count = non_system_messages.len();

        for (index, message) in non_system_messages.into_iter().enumerate() {
            // Handle tool result messages specially for Anthropic format
            if message.role == MessageRole::Tool {
                if let Some(ref tool_call_id) = message.tool_call_id {
                    // Determine if this is an error result
                    let is_error = message.content.contains("<tool_use_error>");

                    let msg = json!({
                        "role": "user",
                        "content": [{
                            "type": "tool_result",
                            "tool_use_id": tool_call_id,
                            "content": message.content,
                            "is_error": is_error
                        }]
                    });
                    converted.push(msg);
                    continue;
                }
            }

            // Handle assistant messages with tool_use
            if message.role == MessageRole::Assistant {
                if let Some(ref tool_calls) = message.tool_calls {
                    if !tool_calls.is_empty() {
                        // Build content array with text and tool_use blocks
                        let mut content_blocks = Vec::new();

                        // Add text block if there's text content
                        if !message.content.is_empty() {
                            content_blocks.push(json!({
                                "type": "text",
                                "text": message.content
                            }));
                        }

                        // Add tool_use blocks
                        for tool_call in tool_calls {
                            content_blocks.push(json!({
                                "type": "tool_use",
                                "id": tool_call.id,
                                "name": tool_call.name,
                                "input": tool_call.arguments
                            }));
                        }

                        let msg = json!({
                            "role": "assistant",
                            "content": content_blocks
                        });
                        converted.push(msg);
                        continue;
                    }
                }
            }

            // Only add cache_control to the last 2 messages (index >= total_count - 2)
            // This keeps us within Anthropic's 4 cache_control block limit
            // Also: cache_control cannot be set on empty text blocks!
            let should_cache = enable_caching
                && index >= total_count.saturating_sub(2)
                && !message.content.is_empty();

            if should_cache {
                let msg = json!({
                    "role": message.role.to_string(),
                    "content": [{
                        "type": "text",
                        "text": message.content,
                        "cache_control": {"type": "ephemeral"}
                    }]
                });
                converted.push(msg);
            } else {
                let msg = json!({
                    "role": message.role.to_string(),
                    "content": message.content
                });
                converted.push(msg);
            }
        }

        Ok(converted)
    }

    /// Convert messages for GLM (Anthropic-compatible format without cache_control)
    pub fn to_glm(messages: &[LlmMessage]) -> SageResult<Vec<Value>> {
        let mut converted = Vec::new();

        // Filter out system messages
        let non_system_messages: Vec<_> = messages
            .iter()
            .filter(|m| m.role != MessageRole::System)
            .collect();

        for message in non_system_messages.into_iter() {
            // Handle tool result messages specially for Anthropic format
            if message.role == MessageRole::Tool {
                if let Some(ref tool_call_id) = message.tool_call_id {
                    let is_error = message.content.contains("<tool_use_error>");
                    let msg = json!({
                        "role": "user",
                        "content": [{
                            "type": "tool_result",
                            "tool_use_id": tool_call_id,
                            "content": message.content,
                            "is_error": is_error
                        }]
                    });
                    converted.push(msg);
                    continue;
                }
            }

            // Handle assistant messages with tool_use
            if message.role == MessageRole::Assistant {
                if let Some(ref tool_calls) = message.tool_calls {
                    if !tool_calls.is_empty() {
                        let mut content_blocks = Vec::new();

                        if !message.content.is_empty() {
                            content_blocks.push(json!({
                                "type": "text",
                                "text": message.content
                            }));
                        }

                        for tool_call in tool_calls {
                            content_blocks.push(json!({
                                "type": "tool_use",
                                "id": tool_call.id,
                                "name": tool_call.name,
                                "input": tool_call.arguments
                            }));
                        }

                        let msg = json!({
                            "role": "assistant",
                            "content": content_blocks
                        });
                        converted.push(msg);
                        continue;
                    }
                }
            }

            // Simple message without cache_control (GLM doesn't support it)
            let msg = json!({
                "role": message.role.to_string(),
                "content": message.content
            });
            converted.push(msg);
        }

        Ok(converted)
    }

    /// Convert messages for Google format
    pub fn to_google(messages: &[LlmMessage]) -> SageResult<Vec<Value>> {
        tracing::debug!("Converting {} messages for Google", messages.len());
        for (i, msg) in messages.iter().enumerate() {
            tracing::debug!(
                "Message {}: role={:?}, content_len={}",
                i,
                msg.role,
                msg.content.len()
            );
        }

        let mut converted = Vec::new();
        let mut system_message = String::new();

        for message in messages {
            match message.role {
                MessageRole::System => {
                    // Collect system messages to prepend to first user message
                    if !system_message.is_empty() {
                        system_message.push_str("\n\n");
                    }
                    system_message.push_str(&message.content);
                }
                MessageRole::User => {
                    let mut content = message.content.clone();
                    if !system_message.is_empty() {
                        content = format!("{}\n\n{}", system_message, content);
                        system_message.clear(); // Only add system message to first user message
                    }

                    converted.push(json!({
                        "role": "user",
                        "parts": [{"text": content}]
                    }));
                }
                MessageRole::Assistant => {
                    let mut parts = Vec::new();

                    // Add text content if present
                    if !message.content.is_empty() {
                        parts.push(json!({"text": message.content}));
                    }

                    // Add function calls if present
                    if let Some(tool_calls) = &message.tool_calls {
                        for tool_call in tool_calls {
                            parts.push(json!({
                                "functionCall": {
                                    "name": tool_call.name,
                                    "args": tool_call.arguments
                                }
                            }));
                        }
                    }

                    converted.push(json!({
                        "role": "model",
                        "parts": parts
                    }));
                }
                MessageRole::Tool => {
                    // Convert tool messages to user messages for Google
                    // Google doesn't support tool role, so we treat tool results as user input
                    converted.push(json!({
                        "role": "user",
                        "parts": [{"text": message.content}]
                    }));
                }
            }
        }

        // If we only have system messages and no user messages, create a user message with the system content
        if converted.is_empty() && !system_message.is_empty() {
            converted.push(json!({
                "role": "user",
                "parts": [{"text": system_message}]
            }));
        }

        // Google API requires conversations to end with a user message
        // If the last message is from the model, add a continuation prompt
        if let Some(last_msg) = converted.last() {
            if last_msg["role"] == "model" {
                converted.push(json!({
                    "role": "user",
                    "parts": [{"text": "Please continue with the task."}]
                }));
            }
        }

        Ok(converted)
    }

    /// Extract system message from messages
    pub fn extract_system_message(messages: &[LlmMessage]) -> (Option<String>, Vec<LlmMessage>) {
        let mut system_content = None;
        let mut other_messages = Vec::new();

        for message in messages {
            if message.role == MessageRole::System {
                system_content = Some(message.content.clone());
            } else {
                other_messages.push(message.clone());
            }
        }

        (system_content, other_messages)
    }
}
