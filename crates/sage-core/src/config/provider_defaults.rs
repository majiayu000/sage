//! Default provider configurations

use crate::config::ModelParameters;
use std::collections::HashMap;

/// Create default model providers configuration
pub fn create_default_providers() -> HashMap<String, ModelParameters> {
    let mut providers = HashMap::new();

    // Anthropic
    providers.insert(
        "anthropic".to_string(),
        ModelParameters {
            model: "claude-sonnet-4-20250514".to_string(),
            api_key: None,
            base_url: Some("https://api.anthropic.com".to_string()),
            max_tokens: Some(4096),
            temperature: Some(0.5),
            top_p: Some(1.0),
            top_k: Some(0),
            parallel_tool_calls: Some(false),
            max_retries: Some(10),
            api_version: None,
            stop_sequences: None,
        },
    );

    // OpenAI
    providers.insert(
        "openai".to_string(),
        ModelParameters {
            model: "gpt-4".to_string(),
            api_key: None,
            base_url: Some("https://api.openai.com".to_string()),
            max_tokens: Some(4096),
            temperature: Some(0.5),
            top_p: Some(1.0),
            top_k: None,
            parallel_tool_calls: Some(true),
            max_retries: Some(10),
            api_version: None,
            stop_sequences: None,
        },
    );

    // Google
    providers.insert(
        "google".to_string(),
        ModelParameters {
            model: "gemini-1.5-pro".to_string(),
            api_key: None,
            base_url: Some("https://generativelanguage.googleapis.com".to_string()),
            max_tokens: Some(4096),
            temperature: Some(0.5),
            top_p: Some(1.0),
            top_k: Some(0),
            parallel_tool_calls: Some(false),
            max_retries: Some(10),
            api_version: None,
            stop_sequences: None,
        },
    );

    // Azure
    providers.insert(
        "azure".to_string(),
        ModelParameters {
            model: "gpt-4".to_string(),
            api_key: None,
            base_url: Some("https://your-resource.openai.azure.com".to_string()),
            max_tokens: Some(4096),
            temperature: Some(0.5),
            top_p: Some(1.0),
            top_k: None,
            parallel_tool_calls: Some(true),
            max_retries: Some(10),
            api_version: Some("2024-02-15-preview".to_string()),
            stop_sequences: None,
        },
    );

    // OpenRouter
    providers.insert(
        "openrouter".to_string(),
        ModelParameters {
            model: "anthropic/claude-3.5-sonnet".to_string(),
            api_key: None,
            base_url: Some("https://openrouter.ai".to_string()),
            max_tokens: Some(4096),
            temperature: Some(0.5),
            top_p: Some(1.0),
            top_k: None,
            parallel_tool_calls: Some(true),
            max_retries: Some(10),
            api_version: None,
            stop_sequences: None,
        },
    );

    // Doubao
    providers.insert(
        "doubao".to_string(),
        ModelParameters {
            model: "doubao-pro-4k".to_string(),
            api_key: None,
            base_url: Some("https://ark.cn-beijing.volces.com".to_string()),
            max_tokens: Some(4096),
            temperature: Some(0.5),
            top_p: Some(1.0),
            top_k: None,
            parallel_tool_calls: Some(true),
            max_retries: Some(10),
            api_version: None,
            stop_sequences: None,
        },
    );

    // Ollama
    providers.insert(
        "ollama".to_string(),
        ModelParameters {
            model: "llama2".to_string(),
            api_key: None,
            base_url: Some("http://localhost:11434".to_string()),
            max_tokens: Some(4096),
            temperature: Some(0.5),
            top_p: Some(1.0),
            top_k: None,
            parallel_tool_calls: Some(false),
            max_retries: Some(3),
            api_version: None,
            stop_sequences: None,
        },
    );

    // GLM (Zhipu AI) - Anthropic-compatible endpoint
    providers.insert(
        "glm".to_string(),
        ModelParameters {
            model: "glm-4.7".to_string(),
            api_key: None,
            base_url: Some("https://open.bigmodel.cn/api/anthropic".to_string()),
            max_tokens: Some(4096),
            temperature: Some(0.7),
            top_p: Some(1.0),
            top_k: None,
            parallel_tool_calls: Some(false),
            max_retries: Some(3),
            api_version: Some("2023-06-01".to_string()),
            stop_sequences: None,
        },
    );

    // Zhipu alias (same behavior as GLM)
    providers.insert(
        "zhipu".to_string(),
        ModelParameters {
            model: "glm-4.7".to_string(),
            api_key: None,
            base_url: Some("https://open.bigmodel.cn/api/anthropic".to_string()),
            max_tokens: Some(4096),
            temperature: Some(0.7),
            top_p: Some(1.0),
            top_k: None,
            parallel_tool_calls: Some(false),
            max_retries: Some(3),
            api_version: Some("2023-06-01".to_string()),
            stop_sequences: None,
        },
    );

    providers
}
