//! API key helper functions for model parameters
//!
//! This module provides helper functions for API key resolution and formatting.

use crate::config::provider::{ApiKeyInfo, ApiKeySource};

/// Get standard environment variable names for a provider
pub fn get_standard_env_vars_for_provider(provider: &str) -> Vec<String> {
    match provider {
        "openai" => vec!["OPENAI_API_KEY".to_string()],
        "anthropic" => vec![
            "ANTHROPIC_API_KEY".to_string(),
            "CLAUDE_API_KEY".to_string(),
        ],
        "google" => vec!["GOOGLE_API_KEY".to_string(), "GEMINI_API_KEY".to_string()],
        "azure" => vec![
            "AZURE_OPENAI_API_KEY".to_string(),
            "AZURE_API_KEY".to_string(),
        ],
        "openrouter" => vec!["OPENROUTER_API_KEY".to_string()],
        "doubao" => vec!["DOUBAO_API_KEY".to_string(), "ARK_API_KEY".to_string()],
        "glm" | "zhipu" => vec!["GLM_API_KEY".to_string(), "ZHIPU_API_KEY".to_string()],
        _ => {
            // For custom or default providers, try <PROVIDER>_API_KEY
            vec![format!("{}_API_KEY", provider.to_uppercase())]
        }
    }
}

/// Format API key status for display
pub fn format_api_key_status_for_provider(provider: &str, info: &ApiKeyInfo) -> String {
    match &info.source {
        ApiKeySource::ConfigFile => {
            format!(
                "✓ {} API key (from config): {}",
                provider,
                info.masked_key().unwrap_or_default()
            )
        }
        ApiKeySource::SageEnvVar | ApiKeySource::StandardEnvVar => {
            format!(
                "✓ {} API key (from {}): {}",
                provider,
                info.env_var_name.as_deref().unwrap_or("env"),
                info.masked_key().unwrap_or_default()
            )
        }
        ApiKeySource::NotFound => {
            let env_hints = get_standard_env_vars_for_provider(provider);
            format!(
                "✗ {} API key missing. Set {} or add to config",
                provider,
                env_hints.first().cloned().unwrap_or_default()
            )
        }
    }
}
