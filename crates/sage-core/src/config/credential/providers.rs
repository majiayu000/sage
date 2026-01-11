//! Provider environment configurations
//!
//! This module defines known provider configurations for API key environment variables.

use std::path::PathBuf;

/// Known provider configurations for environment variable names
#[derive(Debug, Clone)]
pub struct ProviderEnvConfig {
    /// The provider name
    pub name: String,
    /// Environment variable name for API key
    pub env_var: String,
}

impl ProviderEnvConfig {
    pub fn new(name: impl Into<String>, env_var: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            env_var: env_var.into(),
        }
    }
}

/// Default provider configurations
pub fn default_providers() -> Vec<ProviderEnvConfig> {
    vec![
        ProviderEnvConfig::new("anthropic", "ANTHROPIC_API_KEY"),
        ProviderEnvConfig::new("openai", "OPENAI_API_KEY"),
        ProviderEnvConfig::new("google", "GOOGLE_API_KEY"),
        ProviderEnvConfig::new("glm", "GLM_API_KEY"),
        ProviderEnvConfig::new("zhipu", "ZHIPU_API_KEY"),
        ProviderEnvConfig::new("ollama", "OLLAMA_API_KEY"),
    ]
}

/// Paths to check for auto-import from other tools
pub fn auto_import_paths() -> Vec<(String, PathBuf)> {
    let home = dirs::home_dir().unwrap_or_default();
    vec![
        // Claude Code
        (
            "claude-code".to_string(),
            home.join(".claude").join("credentials.json"),
        ),
        // Cursor
        (
            "cursor".to_string(),
            home.join(".cursor").join("credentials.json"),
        ),
        // Aider
        (
            "aider".to_string(),
            home.join(".aider").join("credentials.json"),
        ),
    ]
}
