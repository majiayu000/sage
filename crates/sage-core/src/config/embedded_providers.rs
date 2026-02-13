//! Built-in provider definitions
//!
//! Contains the default set of LLM providers that ship with Sage.

use super::provider_registry::{ModelInfo, ProviderInfo};

/// Get the built-in provider list
pub(super) fn embedded_providers() -> Vec<ProviderInfo> {
    vec![
        ProviderInfo {
            id: "anthropic".to_string(),
            name: "Anthropic".to_string(),
            description: "Claude models (Opus, Sonnet, Haiku)".to_string(),
            api_base_url: "https://api.anthropic.com".to_string(),
            env_var: "ANTHROPIC_API_KEY".to_string(),
            help_url: Some("https://console.anthropic.com/settings/keys".to_string()),
            requires_api_key: true,
            models: vec![
                ModelInfo {
                    id: "claude-sonnet-4-5-20250929".to_string(),
                    name: "Claude 4.5 Sonnet".to_string(),
                    default: true,
                    context_window: Some(200_000),
                    max_output_tokens: Some(64_000),
                },
                ModelInfo {
                    id: "claude-opus-4-5-20251101".to_string(),
                    name: "Claude 4.5 Opus".to_string(),
                    default: false,
                    context_window: Some(200_000),
                    max_output_tokens: Some(32_000),
                },
                ModelInfo {
                    id: "claude-haiku-4-5-20251001".to_string(),
                    name: "Claude 4.5 Haiku".to_string(),
                    default: false,
                    context_window: Some(200_000),
                    max_output_tokens: Some(8_192),
                },
            ],
        },
        ProviderInfo {
            id: "openai".to_string(),
            name: "OpenAI".to_string(),
            description: "GPT-4 and GPT-3.5 models".to_string(),
            api_base_url: "https://api.openai.com/v1".to_string(),
            env_var: "OPENAI_API_KEY".to_string(),
            help_url: Some("https://platform.openai.com/api-keys".to_string()),
            requires_api_key: true,
            models: vec![
                ModelInfo {
                    id: "gpt-4o".to_string(),
                    name: "GPT-4o".to_string(),
                    default: true,
                    context_window: Some(128_000),
                    max_output_tokens: Some(16_384),
                },
                ModelInfo {
                    id: "gpt-4o-mini".to_string(),
                    name: "GPT-4o Mini".to_string(),
                    default: false,
                    context_window: Some(128_000),
                    max_output_tokens: Some(16_384),
                },
                ModelInfo {
                    id: "o1".to_string(),
                    name: "o1".to_string(),
                    default: false,
                    context_window: Some(200_000),
                    max_output_tokens: Some(100_000),
                },
            ],
        },
        ProviderInfo {
            id: "google".to_string(),
            name: "Google".to_string(),
            description: "Gemini models".to_string(),
            api_base_url: "https://generativelanguage.googleapis.com".to_string(),
            env_var: "GOOGLE_API_KEY".to_string(),
            help_url: Some("https://aistudio.google.com/apikey".to_string()),
            requires_api_key: true,
            models: vec![
                ModelInfo {
                    id: "gemini-2.0-flash".to_string(),
                    name: "Gemini 2.0 Flash".to_string(),
                    default: true,
                    context_window: Some(1_000_000),
                    max_output_tokens: Some(8_192),
                },
                ModelInfo {
                    id: "gemini-2.0-pro".to_string(),
                    name: "Gemini 2.0 Pro".to_string(),
                    default: false,
                    context_window: Some(2_000_000),
                    max_output_tokens: Some(8_192),
                },
            ],
        },
        ProviderInfo {
            id: "glm".to_string(),
            name: "GLM (智谱)".to_string(),
            description: "Zhipu AI GLM models".to_string(),
            api_base_url: "https://open.bigmodel.cn/api/anthropic".to_string(),
            env_var: "GLM_API_KEY".to_string(),
            help_url: Some("https://open.bigmodel.cn/usercenter/apikeys".to_string()),
            requires_api_key: true,
            models: vec![
                ModelInfo {
                    id: "glm-4-plus".to_string(),
                    name: "GLM-4 Plus".to_string(),
                    default: true,
                    context_window: Some(128_000),
                    max_output_tokens: Some(4_096),
                },
                ModelInfo {
                    id: "glm-4-flash".to_string(),
                    name: "GLM-4 Flash".to_string(),
                    default: false,
                    context_window: Some(128_000),
                    max_output_tokens: Some(4_096),
                },
            ],
        },
        ProviderInfo {
            id: "ollama".to_string(),
            name: "Ollama".to_string(),
            description: "Local models via Ollama".to_string(),
            api_base_url: "http://localhost:11434".to_string(),
            env_var: "OLLAMA_API_KEY".to_string(),
            help_url: Some("https://ollama.ai".to_string()),
            requires_api_key: false,
            models: vec![
                ModelInfo {
                    id: "llama3.1".to_string(),
                    name: "Llama 3.1".to_string(),
                    default: true,
                    context_window: Some(128_000),
                    max_output_tokens: None,
                },
                ModelInfo {
                    id: "qwen2.5".to_string(),
                    name: "Qwen 2.5".to_string(),
                    default: false,
                    context_window: Some(128_000),
                    max_output_tokens: None,
                },
            ],
        },
        ProviderInfo {
            id: "openrouter".to_string(),
            name: "OpenRouter".to_string(),
            description: "Access multiple providers via OpenRouter".to_string(),
            api_base_url: "https://openrouter.ai/api/v1".to_string(),
            env_var: "OPENROUTER_API_KEY".to_string(),
            help_url: Some("https://openrouter.ai/keys".to_string()),
            requires_api_key: true,
            models: vec![
                ModelInfo {
                    id: "anthropic/claude-sonnet-4".to_string(),
                    name: "Claude 4 Sonnet (via OpenRouter)".to_string(),
                    default: true,
                    context_window: Some(200_000),
                    max_output_tokens: Some(64_000),
                },
            ],
        },
        ProviderInfo {
            id: "azure".to_string(),
            name: "Azure OpenAI".to_string(),
            description: "Azure-hosted OpenAI models".to_string(),
            api_base_url: "".to_string(), // User must configure
            env_var: "AZURE_OPENAI_API_KEY".to_string(),
            help_url: Some("https://portal.azure.com/#view/Microsoft_Azure_ProjectOxford/CognitiveServicesHub/~/OpenAI".to_string()),
            requires_api_key: true,
            models: vec![],
        },
    ]
}