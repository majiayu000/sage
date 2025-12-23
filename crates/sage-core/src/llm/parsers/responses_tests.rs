//! Unit tests for response parsers

#[cfg(test)]
mod tests {
    use crate::llm::parsers::responses::ResponseParser;
    use serde_json::json;

    #[test]
    fn test_parse_openai_basic_text() {
        let response = json!({
            "id": "chatcmpl-123",
            "model": "gpt-4",
            "choices": [{
                "message": {
                    "content": "Hello, world!",
                    "role": "assistant"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }
        });

        let result = ResponseParser::parse_openai(response);
        assert!(result.is_ok());

        let llm_response = result.unwrap();
        assert_eq!(llm_response.content, "Hello, world!");
        assert_eq!(llm_response.model, Some("gpt-4".to_string()));
        assert_eq!(llm_response.id, Some("chatcmpl-123".to_string()));
        assert_eq!(llm_response.finish_reason, Some("stop".to_string()));
        assert!(llm_response.tool_calls.is_empty());

        let usage = llm_response.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 5);
        assert_eq!(usage.total_tokens, 15);
    }

    #[test]
    fn test_parse_openai_with_tool_calls() {
        let response = json!({
            "id": "chatcmpl-456",
            "model": "gpt-4",
            "choices": [{
                "message": {
                    "content": null,
                    "role": "assistant",
                    "tool_calls": [{
                        "id": "call_abc123",
                        "type": "function",
                        "function": {
                            "name": "get_weather",
                            "arguments": "{\"location\": \"San Francisco\", \"unit\": \"celsius\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": {
                "prompt_tokens": 20,
                "completion_tokens": 10,
                "total_tokens": 30
            }
        });

        let result = ResponseParser::parse_openai(response);
        assert!(result.is_ok());

        let llm_response = result.unwrap();
        assert_eq!(llm_response.content, "");
        assert_eq!(llm_response.tool_calls.len(), 1);

        let tool_call = &llm_response.tool_calls[0];
        assert_eq!(tool_call.id, "call_abc123");
        assert_eq!(tool_call.name, "get_weather");
        assert_eq!(
            tool_call
                .arguments
                .get("location")
                .unwrap()
                .as_str()
                .unwrap(),
            "San Francisco"
        );
        assert_eq!(
            tool_call.arguments.get("unit").unwrap().as_str().unwrap(),
            "celsius"
        );
    }

    #[test]
    fn test_parse_openai_multiple_tool_calls() {
        let response = json!({
            "id": "chatcmpl-789",
            "model": "gpt-4",
            "choices": [{
                "message": {
                    "content": "",
                    "role": "assistant",
                    "tool_calls": [
                        {
                            "id": "call_1",
                            "type": "function",
                            "function": {
                                "name": "tool_one",
                                "arguments": "{\"param\": \"value1\"}"
                            }
                        },
                        {
                            "id": "call_2",
                            "type": "function",
                            "function": {
                                "name": "tool_two",
                                "arguments": "{\"param\": \"value2\"}"
                            }
                        }
                    ]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": {
                "prompt_tokens": 30,
                "completion_tokens": 20,
                "total_tokens": 50
            }
        });

        let result = ResponseParser::parse_openai(response);
        assert!(result.is_ok());

        let llm_response = result.unwrap();
        assert_eq!(llm_response.tool_calls.len(), 2);
        assert_eq!(llm_response.tool_calls[0].name, "tool_one");
        assert_eq!(llm_response.tool_calls[1].name, "tool_two");
    }

    #[test]
    fn test_parse_openai_without_usage() {
        let response = json!({
            "id": "chatcmpl-999",
            "model": "gpt-4",
            "choices": [{
                "message": {
                    "content": "Test",
                    "role": "assistant"
                },
                "finish_reason": "stop"
            }]
        });

        let result = ResponseParser::parse_openai(response);
        assert!(result.is_ok());

        let llm_response = result.unwrap();
        assert!(llm_response.usage.is_none());
    }

    #[test]
    fn test_parse_anthropic_basic_text() {
        let response = json!({
            "id": "msg_123",
            "model": "claude-3-5-sonnet-20241022",
            "content": [
                {
                    "type": "text",
                    "text": "Hello from Claude!"
                }
            ],
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 12,
                "output_tokens": 8
            }
        });

        let result = ResponseParser::parse_anthropic(response);
        assert!(result.is_ok());

        let llm_response = result.unwrap();
        assert_eq!(llm_response.content, "Hello from Claude!");
        assert_eq!(
            llm_response.model,
            Some("claude-3-5-sonnet-20241022".to_string())
        );
        assert_eq!(llm_response.id, Some("msg_123".to_string()));
        assert_eq!(llm_response.finish_reason, Some("end_turn".to_string()));
        assert!(llm_response.tool_calls.is_empty());

        let usage = llm_response.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 12);
        assert_eq!(usage.completion_tokens, 8);
        assert_eq!(usage.total_tokens, 20);
    }

    #[test]
    fn test_parse_anthropic_with_tool_use() {
        let response = json!({
            "id": "msg_456",
            "model": "claude-3-opus-20240229",
            "content": [
                {
                    "type": "text",
                    "text": "I'll check the weather for you."
                },
                {
                    "type": "tool_use",
                    "id": "toolu_123",
                    "name": "get_weather",
                    "input": {
                        "location": "Paris",
                        "unit": "fahrenheit"
                    }
                }
            ],
            "stop_reason": "tool_use",
            "usage": {
                "input_tokens": 25,
                "output_tokens": 15
            }
        });

        let result = ResponseParser::parse_anthropic(response);
        assert!(result.is_ok());

        let llm_response = result.unwrap();
        assert_eq!(llm_response.content, "I'll check the weather for you.");
        assert_eq!(llm_response.tool_calls.len(), 1);

        let tool_call = &llm_response.tool_calls[0];
        assert_eq!(tool_call.id, "toolu_123");
        assert_eq!(tool_call.name, "get_weather");
        assert_eq!(
            tool_call
                .arguments
                .get("location")
                .unwrap()
                .as_str()
                .unwrap(),
            "Paris"
        );
        assert_eq!(
            tool_call.arguments.get("unit").unwrap().as_str().unwrap(),
            "fahrenheit"
        );
    }

    #[test]
    fn test_parse_anthropic_multiple_text_blocks() {
        let response = json!({
            "id": "msg_789",
            "model": "claude-3-sonnet-20240229",
            "content": [
                {
                    "type": "text",
                    "text": "First paragraph."
                },
                {
                    "type": "text",
                    "text": "Second paragraph."
                }
            ],
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 10,
                "output_tokens": 10
            }
        });

        let result = ResponseParser::parse_anthropic(response);
        assert!(result.is_ok());

        let llm_response = result.unwrap();
        assert_eq!(llm_response.content, "First paragraph.\nSecond paragraph.");
    }

    #[test]
    fn test_parse_anthropic_with_cache_metrics() {
        let response = json!({
            "id": "msg_cache",
            "model": "claude-3-5-sonnet-20241022",
            "content": [
                {
                    "type": "text",
                    "text": "Response with caching"
                }
            ],
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 100,
                "output_tokens": 50,
                "cache_creation_input_tokens": 80,
                "cache_read_input_tokens": 20
            }
        });

        let result = ResponseParser::parse_anthropic(response);
        assert!(result.is_ok());

        let llm_response = result.unwrap();
        let usage = llm_response.usage.unwrap();

        assert_eq!(usage.cache_creation_input_tokens, Some(80));
        assert_eq!(usage.cache_read_input_tokens, Some(20));
        // Total should include base input + cache tokens
        assert_eq!(usage.prompt_tokens, 200); // 100 + 80 + 20
        assert_eq!(usage.completion_tokens, 50);
        assert_eq!(usage.total_tokens, 250); // 200 + 50
    }

    #[test]
    fn test_parse_anthropic_empty_tool_input() {
        let response = json!({
            "id": "msg_empty",
            "model": "claude-3-opus-20240229",
            "content": [
                {
                    "type": "tool_use",
                    "id": "toolu_empty",
                    "name": "test_tool",
                    "input": {}
                }
            ],
            "stop_reason": "tool_use",
            "usage": {
                "input_tokens": 10,
                "output_tokens": 5
            }
        });

        let result = ResponseParser::parse_anthropic(response);
        assert!(result.is_ok());

        let llm_response = result.unwrap();
        assert_eq!(llm_response.tool_calls.len(), 1);
        assert!(llm_response.tool_calls[0].arguments.is_empty());
    }

    #[test]
    fn test_parse_google_basic_text() {
        let response = json!({
            "candidates": [{
                "content": {
                    "parts": [{
                        "text": "Hello from Gemini!"
                    }],
                    "role": "model"
                },
                "finishReason": "STOP"
            }],
            "usageMetadata": {
                "promptTokenCount": 15,
                "candidatesTokenCount": 10,
                "totalTokenCount": 25
            }
        });

        let result = ResponseParser::parse_google(response, "gemini-pro");
        assert!(result.is_ok());

        let llm_response = result.unwrap();
        assert_eq!(llm_response.content, "Hello from Gemini!");
        assert_eq!(llm_response.model, Some("gemini-pro".to_string()));
        assert_eq!(llm_response.finish_reason, Some("STOP".to_string()));
        assert!(llm_response.tool_calls.is_empty());

        let usage = llm_response.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 15);
        assert_eq!(usage.completion_tokens, 10);
        assert_eq!(usage.total_tokens, 25);
    }

    #[test]
    fn test_parse_google_with_function_call() {
        let response = json!({
            "candidates": [{
                "content": {
                    "parts": [{
                        "functionCall": {
                            "name": "search_web",
                            "args": {
                                "query": "Rust programming",
                                "limit": 5.0
                            }
                        }
                    }],
                    "role": "model"
                },
                "finishReason": "STOP"
            }],
            "usageMetadata": {
                "promptTokenCount": 20,
                "candidatesTokenCount": 12,
                "totalTokenCount": 32
            }
        });

        let result = ResponseParser::parse_google(response, "gemini-pro");
        assert!(result.is_ok());

        let llm_response = result.unwrap();
        assert_eq!(llm_response.tool_calls.len(), 1);

        let tool_call = &llm_response.tool_calls[0];
        assert_eq!(tool_call.name, "search_web");
        assert_eq!(
            tool_call.arguments.get("query").unwrap().as_str().unwrap(),
            "Rust programming"
        );
        assert_eq!(
            tool_call.arguments.get("limit").unwrap().as_f64().unwrap(),
            5.0
        );

        // Should have default text when tool is called
        assert!(llm_response.content.contains("search_web"));
    }

    #[test]
    fn test_parse_google_empty_candidates() {
        let response = json!({
            "candidates": []
        });

        let result = ResponseParser::parse_google(response, "gemini-pro");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Empty candidates array")
        );
    }

    #[test]
    fn test_parse_google_no_candidates() {
        let response = json!({
            "error": "Invalid request"
        });

        let result = ResponseParser::parse_google(response, "gemini-pro");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No candidates"));
    }

    #[test]
    fn test_parse_google_without_usage_metadata() {
        let response = json!({
            "candidates": [{
                "content": {
                    "parts": [{
                        "text": "Test"
                    }],
                    "role": "model"
                },
                "finishReason": "STOP"
            }]
        });

        let result = ResponseParser::parse_google(response, "gemini-pro");
        assert!(result.is_ok());

        let llm_response = result.unwrap();
        assert!(llm_response.usage.is_none());
    }

    #[test]
    fn test_parse_google_multiple_text_parts() {
        let response = json!({
            "candidates": [{
                "content": {
                    "parts": [
                        {"text": "Part 1"},
                        {"text": " Part 2"}
                    ],
                    "role": "model"
                },
                "finishReason": "STOP"
            }]
        });

        let result = ResponseParser::parse_google(response, "gemini-pro");
        assert!(result.is_ok());

        let llm_response = result.unwrap();
        assert_eq!(llm_response.content, "Part 1 Part 2");
    }

    #[test]
    fn test_parse_openai_invalid_tool_arguments() {
        // Test with malformed JSON in arguments
        let response = json!({
            "id": "chatcmpl-err",
            "model": "gpt-4",
            "choices": [{
                "message": {
                    "content": "",
                    "role": "assistant",
                    "tool_calls": [{
                        "id": "call_err",
                        "type": "function",
                        "function": {
                            "name": "test_tool",
                            "arguments": "not valid json"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        });

        let result = ResponseParser::parse_openai(response);
        assert!(result.is_ok());

        let llm_response = result.unwrap();
        // Should fall back to empty map on parse error
        assert_eq!(llm_response.tool_calls.len(), 1);
        assert!(llm_response.tool_calls[0].arguments.is_empty());
    }

    #[test]
    fn test_parse_anthropic_without_usage() {
        let response = json!({
            "id": "msg_no_usage",
            "model": "claude-3-opus-20240229",
            "content": [
                {
                    "type": "text",
                    "text": "Test"
                }
            ],
            "stop_reason": "end_turn"
        });

        let result = ResponseParser::parse_anthropic(response);
        assert!(result.is_ok());

        let llm_response = result.unwrap();
        assert!(llm_response.usage.is_none());
    }

    #[test]
    fn test_parse_anthropic_unknown_content_type() {
        let response = json!({
            "id": "msg_unknown",
            "model": "claude-3-opus-20240229",
            "content": [
                {
                    "type": "text",
                    "text": "Known type"
                },
                {
                    "type": "unknown_type",
                    "data": "should be ignored"
                }
            ],
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 10,
                "output_tokens": 5
            }
        });

        let result = ResponseParser::parse_anthropic(response);
        assert!(result.is_ok());

        let llm_response = result.unwrap();
        // Should only have the text content, unknown type ignored
        assert_eq!(llm_response.content, "Known type");
        assert!(llm_response.tool_calls.is_empty());
    }

    #[test]
    fn test_parse_google_with_text_and_function() {
        let response = json!({
            "candidates": [{
                "content": {
                    "parts": [
                        {
                            "text": "Let me search for that."
                        },
                        {
                            "functionCall": {
                                "name": "search",
                                "args": {"q": "test"}
                            }
                        }
                    ],
                    "role": "model"
                },
                "finishReason": "STOP"
            }]
        });

        let result = ResponseParser::parse_google(response, "gemini-pro");
        assert!(result.is_ok());

        let llm_response = result.unwrap();
        assert_eq!(llm_response.content, "Let me search for that.");
        assert_eq!(llm_response.tool_calls.len(), 1);
    }

    #[test]
    fn test_parse_google_function_without_args() {
        let response = json!({
            "candidates": [{
                "content": {
                    "parts": [{
                        "functionCall": {
                            "name": "no_args_function"
                        }
                    }],
                    "role": "model"
                },
                "finishReason": "STOP"
            }]
        });

        let result = ResponseParser::parse_google(response, "gemini-pro");
        assert!(result.is_ok());

        let llm_response = result.unwrap();
        assert_eq!(llm_response.tool_calls.len(), 1);
        assert!(llm_response.tool_calls[0].arguments.is_empty());
    }

    #[test]
    fn test_parse_anthropic_cache_read_only() {
        let response = json!({
            "id": "msg_cache_read",
            "model": "claude-3-5-sonnet-20241022",
            "content": [{"type": "text", "text": "Cached response"}],
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 50,
                "output_tokens": 25,
                "cache_read_input_tokens": 100
            }
        });

        let result = ResponseParser::parse_anthropic(response);
        assert!(result.is_ok());

        let llm_response = result.unwrap();
        let usage = llm_response.usage.unwrap();

        assert_eq!(usage.cache_read_input_tokens, Some(100));
        assert_eq!(usage.cache_creation_input_tokens, None);
        assert_eq!(usage.prompt_tokens, 150); // 50 + 100
        assert_eq!(usage.total_tokens, 175); // 150 + 25
    }

    #[test]
    fn test_parse_anthropic_cache_creation_only() {
        let response = json!({
            "id": "msg_cache_create",
            "model": "claude-3-5-sonnet-20241022",
            "content": [{"type": "text", "text": "New cached response"}],
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 60,
                "output_tokens": 30,
                "cache_creation_input_tokens": 150
            }
        });

        let result = ResponseParser::parse_anthropic(response);
        assert!(result.is_ok());

        let llm_response = result.unwrap();
        let usage = llm_response.usage.unwrap();

        assert_eq!(usage.cache_creation_input_tokens, Some(150));
        assert_eq!(usage.cache_read_input_tokens, None);
        assert_eq!(usage.prompt_tokens, 210); // 60 + 150
        assert_eq!(usage.total_tokens, 240); // 210 + 30
    }
}
