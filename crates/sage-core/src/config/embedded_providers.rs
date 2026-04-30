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
            description: "Claude frontier models (Opus, Sonnet, Haiku)".to_string(),
            api_base_url: "https://api.anthropic.com".to_string(),
            env_var: "ANTHROPIC_API_KEY".to_string(),
            help_url: Some("https://console.anthropic.com/settings/keys".to_string()),
            requires_api_key: true,
            models: vec![
                ModelInfo {
                    id: "claude-opus-4-7".to_string(),
                    name: "Claude Opus 4.7".to_string(),
                    default: true,
                    context_window: Some(1_000_000),
                    max_output_tokens: Some(128_000),
                },
                ModelInfo {
                    id: "claude-sonnet-4-6".to_string(),
                    name: "Claude Sonnet 4.6".to_string(),
                    default: false,
                    context_window: Some(1_000_000),
                    max_output_tokens: Some(64_000),
                },
                ModelInfo {
                    id: "claude-haiku-4-5".to_string(),
                    name: "Claude 4.5 Haiku".to_string(),
                    default: false,
                    context_window: Some(200_000),
                    max_output_tokens: Some(64_000),
                },
            ],
        },
        ProviderInfo {
            id: "openai".to_string(),
            name: "OpenAI".to_string(),
            description: "GPT-5.4 frontier models".to_string(),
            api_base_url: "https://api.openai.com/v1".to_string(),
            env_var: "OPENAI_API_KEY".to_string(),
            help_url: Some("https://platform.openai.com/api-keys".to_string()),
            requires_api_key: true,
            models: vec![
                ModelInfo {
                    id: "gpt-5.4".to_string(),
                    name: "GPT-5.4".to_string(),
                    default: true,
                    context_window: Some(1_050_000),
                    max_output_tokens: Some(128_000),
                },
                ModelInfo {
                    id: "gpt-5.4-mini".to_string(),
                    name: "GPT-5.4 Mini".to_string(),
                    default: false,
                    context_window: Some(400_000),
                    max_output_tokens: Some(128_000),
                },
                ModelInfo {
                    id: "gpt-5.4-nano".to_string(),
                    name: "GPT-5.4 Nano".to_string(),
                    default: false,
                    context_window: Some(400_000),
                    max_output_tokens: Some(128_000),
                },
            ],
        },
        ProviderInfo {
            id: "google".to_string(),
            name: "Google".to_string(),
            description: "Gemini 2.5 production models".to_string(),
            api_base_url: "https://generativelanguage.googleapis.com".to_string(),
            env_var: "GOOGLE_API_KEY".to_string(),
            help_url: Some("https://aistudio.google.com/apikey".to_string()),
            requires_api_key: true,
            models: vec![
                ModelInfo {
                    id: "gemini-2.5-pro".to_string(),
                    name: "Gemini 2.5 Pro".to_string(),
                    default: true,
                    context_window: Some(1_048_576),
                    max_output_tokens: Some(65_536),
                },
                ModelInfo {
                    id: "gemini-2.5-flash".to_string(),
                    name: "Gemini 2.5 Flash".to_string(),
                    default: false,
                    context_window: Some(1_048_576),
                    max_output_tokens: Some(65_536),
                },
            ],
        },
        ProviderInfo {
            id: "glm".to_string(),
            name: "GLM (智谱)".to_string(),
            description: "Zhipu AI GLM models via Anthropic-compatible endpoint".to_string(),
            api_base_url: "https://open.bigmodel.cn/api/anthropic".to_string(),
            env_var: "GLM_API_KEY".to_string(),
            help_url: Some("https://open.bigmodel.cn/usercenter/apikeys".to_string()),
            requires_api_key: true,
            models: vec![
                ModelInfo {
                    id: "glm-4.7".to_string(),
                    name: "GLM-4.7".to_string(),
                    default: true,
                    context_window: Some(204_800),
                    max_output_tokens: Some(131_072),
                },
                ModelInfo {
                    id: "glm-4.7-flash".to_string(),
                    name: "GLM-4.7 Flash".to_string(),
                    default: false,
                    context_window: Some(204_800),
                    max_output_tokens: Some(131_072),
                },
            ],
        },
        ProviderInfo {
            id: "zai".to_string(),
            name: "Z.AI".to_string(),
            description: "GLM-5.x coding models via the Z.AI OpenAI-compatible API".to_string(),
            api_base_url: "https://api.z.ai/api/paas/v4".to_string(),
            env_var: "ZAI_API_KEY".to_string(),
            help_url: Some("https://docs.z.ai/api-reference/introduction".to_string()),
            requires_api_key: true,
            models: vec![
                ModelInfo {
                    id: "glm-5.1".to_string(),
                    name: "GLM-5.1".to_string(),
                    default: true,
                    context_window: Some(204_800),
                    max_output_tokens: Some(131_072),
                },
                ModelInfo {
                    id: "glm-5".to_string(),
                    name: "GLM-5".to_string(),
                    default: false,
                    context_window: Some(204_800),
                    max_output_tokens: Some(131_072),
                },
            ],
        },
        ProviderInfo {
            id: "moonshot".to_string(),
            name: "Moonshot AI (Kimi)".to_string(),
            description: "Latest Kimi coding and multimodal models via the OpenAI-compatible API"
                .to_string(),
            api_base_url: "https://api.moonshot.ai/v1".to_string(),
            env_var: "MOONSHOT_API_KEY".to_string(),
            help_url: Some("https://platform.kimi.ai/docs/models".to_string()),
            requires_api_key: true,
            models: vec![
                ModelInfo {
                    id: "kimi-k2.6".to_string(),
                    name: "Kimi K2.6".to_string(),
                    default: true,
                    context_window: Some(256_000),
                    max_output_tokens: Some(32_768),
                },
                ModelInfo {
                    id: "kimi-k2.5".to_string(),
                    name: "Kimi K2.5".to_string(),
                    default: false,
                    context_window: Some(256_000),
                    max_output_tokens: Some(32_768),
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
