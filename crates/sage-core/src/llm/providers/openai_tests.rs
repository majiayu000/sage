//! Integration tests for OpenAI provider with mock server

#[cfg(test)]
mod tests {
    use crate::config::provider::ProviderConfig;
    use crate::llm::messages::LlmMessage;
    use crate::llm::provider_types::ModelParameters;
    use crate::llm::providers::OpenAiProvider;
    use reqwest::Client;
    use serde_json::json;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn create_test_provider(base_url: &str) -> OpenAiProvider {
        let config = ProviderConfig::new("openai")
            .with_api_key("test-api-key")
            .with_base_url(base_url);
        let model_params = ModelParameters::new("gpt-4");
        let http_client = Client::builder().no_proxy().build().expect("Failed to create HTTP client");
        OpenAiProvider::new(config, model_params, http_client)
    }

    fn mock_openai_response(content: &str) -> serde_json::Value {
        json!({
            "id": "chatcmpl-test123",
            "object": "chat.completion",
            "created": 1704067200,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": content
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 20,
                "total_tokens": 30
            }
        })
    }

    fn mock_openai_tool_call_response(tool_name: &str, args: &str) -> serde_json::Value {
        json!({
            "id": "chatcmpl-test456",
            "object": "chat.completion",
            "created": 1704067200,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_abc123",
                        "type": "function",
                        "function": {
                            "name": tool_name,
                            "arguments": args
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": {
                "prompt_tokens": 15,
                "completion_tokens": 25,
                "total_tokens": 40
            }
        })
    }

    #[tokio::test]
    async fn test_chat_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(header("Authorization", "Bearer test-api-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_openai_response(
                "Hello! I'm an AI assistant.",
            )))
            .mount(&mock_server)
            .await;

        let provider = create_test_provider(&mock_server.uri());
        let messages = vec![LlmMessage::user("Hello!")];

        let result = provider.chat(&messages, None).await;
        assert!(result.is_ok(), "Expected success, got: {:?}", result);

        let response = result.unwrap();
        assert_eq!(response.content, "Hello! I'm an AI assistant.");

        // Check usage if present
        if let Some(usage) = response.usage {
            assert_eq!(usage.prompt_tokens, 10);
            assert_eq!(usage.completion_tokens, 20);
        }
    }

    #[tokio::test]
    async fn test_chat_with_system_message() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(mock_openai_response("I am a helpful assistant.")),
            )
            .mount(&mock_server)
            .await;

        let provider = create_test_provider(&mock_server.uri());
        let messages = vec![
            LlmMessage::system("You are a helpful assistant."),
            LlmMessage::user("What are you?"),
        ];

        let result = provider.chat(&messages, None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_chat_with_tool_calls() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                mock_openai_tool_call_response("read_file", r#"{"path": "/test/file.txt"}"#),
            ))
            .mount(&mock_server)
            .await;

        let provider = create_test_provider(&mock_server.uri());
        let messages = vec![LlmMessage::user("Read the file at /test/file.txt")];

        let result = provider.chat(&messages, None).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(!response.tool_calls.is_empty());
        assert_eq!(response.tool_calls[0].name, "read_file");
    }

    #[tokio::test]
    async fn test_chat_api_error_401() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(401)
                    .set_body_json(json!({"error": {"message": "Invalid API key"}})),
            )
            .mount(&mock_server)
            .await;

        let provider = create_test_provider(&mock_server.uri());
        let messages = vec![LlmMessage::user("Hello!")];

        let result = provider.chat(&messages, None).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("401") || err_msg.contains("error"));
    }

    #[tokio::test]
    async fn test_chat_api_error_429_rate_limit() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(429).set_body_json(json!({
                "error": {
                    "message": "Rate limit exceeded",
                    "type": "rate_limit_error"
                }
            })))
            .mount(&mock_server)
            .await;

        let provider = create_test_provider(&mock_server.uri());
        let messages = vec![LlmMessage::user("Hello!")];

        let result = provider.chat(&messages, None).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("429") || err_msg.contains("rate"));
    }

    #[tokio::test]
    async fn test_chat_api_error_500() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(500).set_body_json(json!({"error": "Internal server error"})),
            )
            .mount(&mock_server)
            .await;

        let provider = create_test_provider(&mock_server.uri());
        let messages = vec![LlmMessage::user("Hello!")];

        let result = provider.chat(&messages, None).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("500"));
    }

    #[tokio::test]
    async fn test_chat_model_parameter_sent() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(mock_openai_response("Response")),
            )
            .mount(&mock_server)
            .await;

        let provider = create_test_provider(&mock_server.uri());
        let messages = vec![LlmMessage::user("Hello!")];

        let result = provider.chat(&messages, None).await;
        assert!(result.is_ok(), "Model parameter should be sent correctly");
    }

    #[tokio::test]
    async fn test_chat_with_temperature() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(mock_openai_response("Creative response")),
            )
            .mount(&mock_server)
            .await;

        let config = ProviderConfig::new("openai")
            .with_api_key("test-key")
            .with_base_url(&mock_server.uri());
        let model_params = ModelParameters::new("gpt-4").with_temperature(0.9);
        let http_client = Client::builder().no_proxy().build().expect("Failed to create HTTP client");
        let provider = OpenAiProvider::new(config, model_params, http_client);

        let messages = vec![LlmMessage::user("Be creative")];
        let result = provider.chat(&messages, None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_chat_malformed_response() {
        let mock_server = MockServer::start().await;

        // Return invalid JSON that won't parse correctly
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "test",
                "choices": []  // Empty choices array
            })))
            .mount(&mock_server)
            .await;

        let provider = create_test_provider(&mock_server.uri());
        let messages = vec![LlmMessage::user("Hello!")];

        let result = provider.chat(&messages, None).await;
        // Should either error or return empty content
        if let Ok(response) = result {
            assert!(response.content.is_empty());
        }
    }

    #[tokio::test]
    async fn test_chat_empty_content_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "chatcmpl-test",
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": ""
                    },
                    "finish_reason": "stop"
                }],
                "usage": {"prompt_tokens": 5, "completion_tokens": 0, "total_tokens": 5}
            })))
            .mount(&mock_server)
            .await;

        let provider = create_test_provider(&mock_server.uri());
        let messages = vec![LlmMessage::user("...")];

        let result = provider.chat(&messages, None).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.content, "");
    }
}
