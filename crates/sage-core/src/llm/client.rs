//! LLM client implementation

use crate::error::{SageError, SageResult};
use crate::llm::messages::{LLMMessage, LLMResponse};
use crate::llm::providers::{LLMProvider, ModelParameters};
use crate::config::provider::ProviderConfig;
use crate::tools::types::ToolSchema;
use crate::types::LLMUsage;
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;

/// LLM client for making requests to various providers
pub struct LLMClient {
    provider: LLMProvider,
    config: ProviderConfig,
    model_params: ModelParameters,
    http_client: Client,
}

impl LLMClient {
    /// Create a new LLM client
    pub fn new(
        provider: LLMProvider,
        config: ProviderConfig,
        model_params: ModelParameters,
    ) -> SageResult<Self> {
        // Validate configuration
        config.validate()
            .map_err(|e| SageError::config(format!("Invalid provider config: {}", e)))?;

        // Create HTTP client
        let mut client_builder = Client::builder()
            .timeout(Duration::from_secs(config.timeout.unwrap_or(60)));

        // Add custom headers
        let mut headers = reqwest::header::HeaderMap::new();
        for (key, value) in &config.headers {
            if let (Ok(name), Ok(val)) = (
                reqwest::header::HeaderName::from_bytes(key.as_bytes()),
                reqwest::header::HeaderValue::from_str(value),
            ) {
                headers.insert(name, val);
            }
        }

        if !headers.is_empty() {
            client_builder = client_builder.default_headers(headers);
        }

        let http_client = client_builder
            .build()
            .map_err(|e| SageError::llm(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            provider,
            config,
            model_params,
            http_client,
        })
    }

    /// Send a chat completion request
    pub async fn chat(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMResponse> {
        // TODO: Add streaming response support
        // - Implement streaming for all providers
        // - Add Server-Sent Events (SSE) support
        // - Support token-by-token processing

        // TODO: Add response caching
        // - Cache responses based on message hash
        // - Implement cache invalidation strategies
        // - Support distributed caching for multi-instance deployments

        // TODO: Add request/response middleware
        // - Request preprocessing and validation
        // - Response post-processing and filtering
        // - Metrics collection and monitoring

        match &self.provider {
            LLMProvider::OpenAI => self.openai_chat(messages, tools).await,
            LLMProvider::Anthropic => self.anthropic_chat(messages, tools).await,
            LLMProvider::Google => self.google_chat(messages, tools).await,
            LLMProvider::Azure => self.azure_chat(messages, tools).await,
            LLMProvider::OpenRouter => self.openrouter_chat(messages, tools).await,
            LLMProvider::Doubao => self.doubao_chat(messages, tools).await,
            LLMProvider::Ollama => self.ollama_chat(messages, tools).await,
            LLMProvider::Custom(name) => {
                // TODO: Implement plugin system for custom providers
                // - Add provider plugin API
                // - Support dynamic provider loading
                // - Implement provider validation and security
                Err(SageError::llm(format!("Custom provider '{name}' not implemented")))
            }
        }
    }

    /// OpenAI chat completion
    async fn openai_chat(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMResponse> {
        let url = format!("{}/chat/completions", self.config.get_base_url());
        
        let mut request_body = json!({
            "model": self.model_params.model,
            "messages": self.convert_messages_for_openai(messages)?,
        });

        // Add optional parameters
        if let Some(max_tokens) = self.model_params.max_tokens {
            request_body["max_tokens"] = json!(max_tokens);
        }
        if let Some(temperature) = self.model_params.temperature {
            request_body["temperature"] = json!(temperature);
        }
        if let Some(top_p) = self.model_params.top_p {
            request_body["top_p"] = json!(top_p);
        }
        if let Some(stop) = &self.model_params.stop {
            request_body["stop"] = json!(stop);
        }

        // Add tools if provided
        if let Some(tools) = tools {
            if !tools.is_empty() {
                request_body["tools"] = json!(self.convert_tools_for_openai(tools)?);
                if let Some(parallel) = self.model_params.parallel_tool_calls {
                    request_body["parallel_tool_calls"] = json!(parallel);
                }
            }
        }

        let mut request = self.http_client
            .post(&url)
            .json(&request_body);

        // Add authentication
        if let Some(api_key) = self.config.get_api_key() {
            request = request.bearer_auth(api_key);
        }

        // Add organization header if provided
        if let Some(org) = &self.config.organization {
            request = request.header("OpenAI-Organization", org);
        }

        let response = request
            .send()
            .await
            .map_err(|e| SageError::llm(format!("OpenAI request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SageError::llm(format!("OpenAI API error: {}", error_text)));
        }

        let response_json: Value = response
            .json()
            .await
            .map_err(|e| SageError::llm(format!("Failed to parse OpenAI response: {}", e)))?;

        self.parse_openai_response(response_json)
    }

    /// Anthropic chat completion
    async fn anthropic_chat(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMResponse> {
        let url = format!("{}/v1/messages", self.config.get_base_url());
        
        let (system_message, user_messages) = self.extract_system_message(messages);
        
        let mut request_body = json!({
            "model": self.model_params.model,
            "messages": self.convert_messages_for_anthropic(&user_messages)?,
        });

        if let Some(system) = system_message {
            request_body["system"] = json!(system);
        }

        // Add optional parameters
        if let Some(max_tokens) = self.model_params.max_tokens {
            request_body["max_tokens"] = json!(max_tokens);
        }
        if let Some(temperature) = self.model_params.temperature {
            request_body["temperature"] = json!(temperature);
        }
        if let Some(top_p) = self.model_params.top_p {
            request_body["top_p"] = json!(top_p);
        }
        if let Some(stop) = &self.model_params.stop {
            request_body["stop_sequences"] = json!(stop);
        }

        // Add tools if provided
        if let Some(tools) = tools {
            if !tools.is_empty() {
                request_body["tools"] = json!(self.convert_tools_for_anthropic(tools)?);
            }
        }

        let mut request = self.http_client
            .post(&url)
            .json(&request_body);

        // Add authentication
        if let Some(api_key) = self.config.get_api_key() {
            request = request.header("x-api-key", api_key);
        }

        // Add API version
        if let Some(version) = &self.config.api_version {
            request = request.header("anthropic-version", version);
        }

        let response = request
            .send()
            .await
            .map_err(|e| SageError::llm(format!("Anthropic request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SageError::llm(format!("Anthropic API error: {}", error_text)));
        }

        let response_json: Value = response
            .json()
            .await
            .map_err(|e| SageError::llm(format!("Failed to parse Anthropic response: {}", e)))?;

        self.parse_anthropic_response(response_json)
    }

    /// Azure OpenAI chat completion
    async fn azure_chat(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMResponse> {
        let url = format!("{}/openai/deployments/{}/chat/completions?api-version={}",
            self.config.get_base_url(),
            self.model_params.model,
            self.config.api_version.as_deref().unwrap_or("2024-02-15-preview")
        );

        let mut request_body = json!({
            "messages": self.convert_messages_for_openai(messages)?,
        });

        // Add optional parameters
        if let Some(max_tokens) = self.model_params.max_tokens {
            request_body["max_tokens"] = json!(max_tokens);
        }
        if let Some(temperature) = self.model_params.temperature {
            request_body["temperature"] = json!(temperature);
        }
        if let Some(top_p) = self.model_params.top_p {
            request_body["top_p"] = json!(top_p);
        }

        // Add tools if provided
        if let Some(tools) = tools {
            let tool_schemas: Vec<Value> = tools.iter()
                .map(|tool| json!({
                    "type": "function",
                    "function": {
                        "name": tool.name,
                        "description": tool.description,
                        "parameters": tool.parameters
                    }
                }))
                .collect();
            request_body["tools"] = json!(tool_schemas);
        }

        let request = self.http_client
            .post(&url)
            .header("api-key", self.config.get_api_key().unwrap_or_default())
            .header("Content-Type", "application/json")
            .json(&request_body);

        tracing::debug!("Azure API request: {}", serde_json::to_string_pretty(&request_body).unwrap_or_default());

        let response = request.send().await
            .map_err(|e| SageError::llm(format!("Azure API request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SageError::llm(format!("Azure API error: {}", error_text)));
        }

        let response_json: Value = response.json().await
            .map_err(|e| SageError::llm(format!("Failed to parse Azure response: {}", e)))?;

        tracing::debug!("Azure API response: {}", serde_json::to_string_pretty(&response_json).unwrap_or_default());

        self.parse_openai_response(response_json)
    }

    /// OpenRouter chat completion
    async fn openrouter_chat(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMResponse> {
        let url = format!("{}/api/v1/chat/completions", self.config.get_base_url());

        let mut request_body = json!({
            "model": self.model_params.model,
            "messages": self.convert_messages_for_openai(messages)?,
        });

        // Add optional parameters
        if let Some(max_tokens) = self.model_params.max_tokens {
            request_body["max_tokens"] = json!(max_tokens);
        }
        if let Some(temperature) = self.model_params.temperature {
            request_body["temperature"] = json!(temperature);
        }
        if let Some(top_p) = self.model_params.top_p {
            request_body["top_p"] = json!(top_p);
        }

        // Add tools if provided
        if let Some(tools) = tools {
            let tool_schemas: Vec<Value> = tools.iter()
                .map(|tool| json!({
                    "type": "function",
                    "function": {
                        "name": tool.name,
                        "description": tool.description,
                        "parameters": tool.parameters
                    }
                }))
                .collect();
            request_body["tools"] = json!(tool_schemas);
        }

        let request = self.http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.get_api_key().unwrap_or_default()))
            .header("Content-Type", "application/json")
            .json(&request_body);

        tracing::debug!("OpenRouter API request: {}", serde_json::to_string_pretty(&request_body).unwrap_or_default());

        let response = request.send().await
            .map_err(|e| SageError::llm(format!("OpenRouter API request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SageError::llm(format!("OpenRouter API error: {}", error_text)));
        }

        let response_json: Value = response.json().await
            .map_err(|e| SageError::llm(format!("Failed to parse OpenRouter response: {}", e)))?;

        tracing::debug!("OpenRouter API response: {}", serde_json::to_string_pretty(&response_json).unwrap_or_default());

        self.parse_openai_response(response_json)
    }

    /// Doubao chat completion
    async fn doubao_chat(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMResponse> {
        let url = format!("{}/api/v3/chat/completions", self.config.get_base_url());

        let mut request_body = json!({
            "model": self.model_params.model,
            "messages": self.convert_messages_for_openai(messages)?,
        });

        // Add optional parameters
        if let Some(max_tokens) = self.model_params.max_tokens {
            request_body["max_tokens"] = json!(max_tokens);
        }
        if let Some(temperature) = self.model_params.temperature {
            request_body["temperature"] = json!(temperature);
        }
        if let Some(top_p) = self.model_params.top_p {
            request_body["top_p"] = json!(top_p);
        }

        // Add tools if provided
        if let Some(tools) = tools {
            let tool_schemas: Vec<Value> = tools.iter()
                .map(|tool| json!({
                    "type": "function",
                    "function": {
                        "name": tool.name,
                        "description": tool.description,
                        "parameters": tool.parameters
                    }
                }))
                .collect();
            request_body["tools"] = json!(tool_schemas);
        }

        let request = self.http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.get_api_key().unwrap_or_default()))
            .header("Content-Type", "application/json")
            .json(&request_body);

        tracing::debug!("Doubao API request: {}", serde_json::to_string_pretty(&request_body).unwrap_or_default());

        let response = request.send().await
            .map_err(|e| SageError::llm(format!("Doubao API request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SageError::llm(format!("Doubao API error: {}", error_text)));
        }

        let response_json: Value = response.json().await
            .map_err(|e| SageError::llm(format!("Failed to parse Doubao response: {}", e)))?;

        tracing::debug!("Doubao API response: {}", serde_json::to_string_pretty(&response_json).unwrap_or_default());

        self.parse_openai_response(response_json)
    }

    /// Google (Gemini) chat completion
    async fn google_chat(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMResponse> {
        let api_key = self.config.get_api_key()
            .ok_or_else(|| SageError::llm("Google API key not provided"))?;

        let url = format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            self.config.get_base_url(),
            self.model_params.model,
            api_key
        );

        let converted_messages = self.convert_messages_for_google(messages)?;
        tracing::debug!("Google API converted messages: {:?}", converted_messages);

        let mut request_body = json!({
            "contents": converted_messages,
        });

        // Add generation config
        let mut generation_config = json!({});
        if let Some(max_tokens) = self.model_params.max_tokens {
            generation_config["maxOutputTokens"] = json!(max_tokens);
        }
        if let Some(temperature) = self.model_params.temperature {
            generation_config["temperature"] = json!(temperature);
        }
        if let Some(top_p) = self.model_params.top_p {
            generation_config["topP"] = json!(top_p);
        }
        if let Some(top_k) = self.model_params.top_k {
            generation_config["topK"] = json!(top_k);
        }
        if let Some(stop) = &self.model_params.stop {
            generation_config["stopSequences"] = json!(stop);
        }

        if !generation_config.as_object().unwrap().is_empty() {
            request_body["generationConfig"] = generation_config;
        }

        // Add tools if provided
        if let Some(tools) = tools {
            if !tools.is_empty() {
                request_body["tools"] = json!([{
                    "functionDeclarations": self.convert_tools_for_google(tools)?
                }]);
            }
        }

        let response = self.http_client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| SageError::llm(format!("Google request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SageError::llm(format!("Google API error: {}", error_text)));
        }

        let response_json: Value = response
            .json()
            .await
            .map_err(|e| SageError::llm(format!("Failed to parse Google response: {}", e)))?;

        tracing::debug!("Google API response: {}", serde_json::to_string_pretty(&response_json).unwrap_or_else(|_| "Failed to serialize".to_string()));

        self.parse_google_response(response_json)
    }

    /// Ollama chat completion
    async fn ollama_chat(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMResponse> {
        let url = format!("{}/v1/chat/completions", self.config.get_base_url());

        let mut request_body = json!({
            "model": self.model_params.model,
            "messages": self.convert_messages_for_openai(messages)?,
        });

        // Add optional parameters (Ollama supports limited parameters)
        if let Some(temperature) = self.model_params.temperature {
            request_body["temperature"] = json!(temperature);
        }
        if let Some(top_p) = self.model_params.top_p {
            request_body["top_p"] = json!(top_p);
        }

        // Add tools if provided (Ollama has limited tool support)
        if let Some(tools) = tools {
            let tool_schemas: Vec<Value> = tools.iter()
                .map(|tool| json!({
                    "type": "function",
                    "function": {
                        "name": tool.name,
                        "description": tool.description,
                        "parameters": tool.parameters
                    }
                }))
                .collect();
            request_body["tools"] = json!(tool_schemas);
        }

        let request = self.http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.get_api_key().unwrap_or_else(|| "ollama".to_string())))
            .header("Content-Type", "application/json")
            .json(&request_body);

        tracing::debug!("Ollama API request: {}", serde_json::to_string_pretty(&request_body).unwrap_or_default());

        let response = request.send().await
            .map_err(|e| SageError::llm(format!("Ollama API request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SageError::llm(format!("Ollama API error: {}", error_text)));
        }

        let response_json: Value = response.json().await
            .map_err(|e| SageError::llm(format!("Failed to parse Ollama response: {}", e)))?;

        tracing::debug!("Ollama API response: {}", serde_json::to_string_pretty(&response_json).unwrap_or_default());

        self.parse_openai_response(response_json)
    }

    /// Convert messages for OpenAI format
    fn convert_messages_for_openai(&self, messages: &[LLMMessage]) -> SageResult<Vec<Value>> {
        let mut converted = Vec::new();
        
        for message in messages {
            let mut msg = json!({
                "role": message.role.to_string(),
                "content": message.content
            });

            if let Some(tool_calls) = &message.tool_calls {
                msg["tool_calls"] = json!(tool_calls);
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
    fn convert_messages_for_anthropic(&self, messages: &[LLMMessage]) -> SageResult<Vec<Value>> {
        let mut converted = Vec::new();
        
        for message in messages {
            // Skip system messages (handled separately)
            if message.role == crate::llm::messages::MessageRole::System {
                continue;
            }

            let msg = json!({
                "role": message.role.to_string(),
                "content": message.content
            });

            converted.push(msg);
        }
        
        Ok(converted)
    }

    /// Extract system message from messages
    fn extract_system_message(&self, messages: &[LLMMessage]) -> (Option<String>, Vec<LLMMessage>) {
        let mut system_content = None;
        let mut other_messages = Vec::new();

        for message in messages {
            if message.role == crate::llm::messages::MessageRole::System {
                system_content = Some(message.content.clone());
            } else {
                other_messages.push(message.clone());
            }
        }

        (system_content, other_messages)
    }

    /// Convert tools for OpenAI format
    fn convert_tools_for_openai(&self, tools: &[ToolSchema]) -> SageResult<Vec<Value>> {
        let mut converted = Vec::new();
        
        for tool in tools {
            let tool_def = json!({
                "type": "function",
                "function": {
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.parameters
                }
            });
            converted.push(tool_def);
        }
        
        Ok(converted)
    }

    /// Convert tools for Anthropic format
    fn convert_tools_for_anthropic(&self, tools: &[ToolSchema]) -> SageResult<Vec<Value>> {
        let mut converted = Vec::new();
        
        for tool in tools {
            let tool_def = json!({
                "name": tool.name,
                "description": tool.description,
                "input_schema": tool.parameters
            });
            converted.push(tool_def);
        }
        
        Ok(converted)
    }

    /// Parse OpenAI response
    fn parse_openai_response(&self, response: Value) -> SageResult<LLMResponse> {
        let choice = response["choices"][0].clone();
        let message = &choice["message"];
        
        let content = message["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let mut tool_calls = Vec::new();
        if let Some(calls) = message["tool_calls"].as_array() {
            for call in calls {
                if let Some(function) = call["function"].as_object() {
                    let tool_call = crate::tools::types::ToolCall {
                        id: call["id"].as_str().unwrap_or("").to_string(),
                        name: function["name"].as_str().unwrap_or("").to_string(),
                        arguments: serde_json::from_str(
                            function["arguments"].as_str().unwrap_or("{}")
                        ).unwrap_or_default(),
                        call_id: None,
                    };
                    tool_calls.push(tool_call);
                }
            }
        }

        let usage = if let Some(usage_data) = response["usage"].as_object() {
            Some(LLMUsage {
                prompt_tokens: usage_data["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: usage_data["completion_tokens"].as_u64().unwrap_or(0) as u32,
                total_tokens: usage_data["total_tokens"].as_u64().unwrap_or(0) as u32,
                cost_usd: None,
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
    fn parse_anthropic_response(&self, response: Value) -> SageResult<LLMResponse> {
        let content = response["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let usage = if let Some(usage_data) = response["usage"].as_object() {
            Some(LLMUsage {
                prompt_tokens: usage_data["input_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: usage_data["output_tokens"].as_u64().unwrap_or(0) as u32,
                total_tokens: (usage_data["input_tokens"].as_u64().unwrap_or(0) + 
                              usage_data["output_tokens"].as_u64().unwrap_or(0)) as u32,
                cost_usd: None,
            })
        } else {
            None
        };

        Ok(LLMResponse {
            content,
            tool_calls: Vec::new(), // TODO: Parse Anthropic tool calls
            usage,
            model: response["model"].as_str().map(|s| s.to_string()),
            finish_reason: response["stop_reason"].as_str().map(|s| s.to_string()),
            id: response["id"].as_str().map(|s| s.to_string()),
            metadata: HashMap::new(),
        })
    }

    /// Convert messages for Google format
    fn convert_messages_for_google(&self, messages: &[LLMMessage]) -> SageResult<Vec<Value>> {
        tracing::debug!("Converting {} messages for Google", messages.len());
        for (i, msg) in messages.iter().enumerate() {
            tracing::debug!("Message {}: role={:?}, content_len={}", i, msg.role, msg.content.len());
        }

        let mut converted = Vec::new();
        let mut system_message = String::new();

        for message in messages {
            match message.role {
                crate::llm::messages::MessageRole::System => {
                    // Collect system messages to prepend to first user message
                    if !system_message.is_empty() {
                        system_message.push_str("\n\n");
                    }
                    system_message.push_str(&message.content);
                },
                crate::llm::messages::MessageRole::User => {
                    let mut content = message.content.clone();
                    if !system_message.is_empty() {
                        content = format!("{}\n\n{}", system_message, content);
                        system_message.clear(); // Only add system message to first user message
                    }

                    converted.push(json!({
                        "role": "user",
                        "parts": [{"text": content}]
                    }));
                },
                crate::llm::messages::MessageRole::Assistant => {
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
                },
                crate::llm::messages::MessageRole::Tool => {
                    // Convert tool messages to user messages for Google
                    // Google doesn't support tool role, so we treat tool results as user input
                    converted.push(json!({
                        "role": "user",
                        "parts": [{"text": message.content}]
                    }));
                },
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

    /// Convert tools for Google format
    fn convert_tools_for_google(&self, tools: &[ToolSchema]) -> SageResult<Vec<Value>> {
        let mut converted = Vec::new();

        for tool in tools {
            let tool_def = json!({
                "name": tool.name,
                "description": tool.description,
                "parameters": tool.parameters
            });
            converted.push(tool_def);
        }

        Ok(converted)
    }

    /// Parse Google response
    fn parse_google_response(&self, response: Value) -> SageResult<LLMResponse> {
        let candidates = response["candidates"].as_array()
            .ok_or_else(|| SageError::llm("No candidates in Google response"))?;

        if candidates.is_empty() {
            return Err(SageError::llm("Empty candidates array in Google response"));
        }

        let candidate = &candidates[0];
        let content_parts = candidate["content"]["parts"].as_array()
            .ok_or_else(|| SageError::llm("No content parts in Google response"))?;

        let mut content = String::new();
        let mut tool_calls = Vec::new();

        for part in content_parts {
            if let Some(text) = part["text"].as_str() {
                content.push_str(text);
            } else if let Some(function_call) = part["functionCall"].as_object() {
                let tool_name = function_call["name"].as_str().unwrap_or("").to_string();
                let tool_call = crate::tools::types::ToolCall {
                    id: format!("call_{}", uuid::Uuid::new_v4()),
                    name: tool_name.clone(),
                    arguments: function_call["args"].as_object()
                        .map(|args| {
                            let mut map = std::collections::HashMap::new();
                            for (k, v) in args {
                                map.insert(k.clone(), v.clone());
                            }
                            map
                        })
                        .unwrap_or_else(|| std::collections::HashMap::new()),
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
            let prompt_tokens = usage_metadata.get("promptTokenCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32;
            let completion_tokens = usage_metadata.get("candidatesTokenCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32;
            let total_tokens = usage_metadata.get("totalTokenCount")
                .and_then(|v| v.as_u64())
                .unwrap_or((prompt_tokens + completion_tokens) as u64) as u32;

            Some(LLMUsage {
                prompt_tokens,
                completion_tokens,
                total_tokens,
                cost_usd: None,
            })
        } else {
            None
        };

        Ok(LLMResponse {
            content,
            tool_calls,
            usage,
            model: Some(self.model_params.model.clone()),
            finish_reason: candidate["finishReason"].as_str().map(|s| s.to_string()),
            id: None, // Google doesn't provide request ID in the same way
            metadata: HashMap::new(),
        })
    }
}
