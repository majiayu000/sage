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

/// Default provider configurations.
///
/// The list must stay aligned with `embedded_providers()` in
/// `crate::config::embedded_providers`: every provider that the
/// onboarding flow can register also needs its environment variable
/// declared here, otherwise the resolver will silently never look
/// up the user's exported key.
pub fn default_providers() -> Vec<ProviderEnvConfig> {
    vec![
        ProviderEnvConfig::new("anthropic", "ANTHROPIC_API_KEY"),
        ProviderEnvConfig::new("openai", "OPENAI_API_KEY"),
        ProviderEnvConfig::new("google", "GOOGLE_API_KEY"),
        ProviderEnvConfig::new("glm", "GLM_API_KEY"),
        ProviderEnvConfig::new("zhipu", "ZHIPU_API_KEY"),
        ProviderEnvConfig::new("zai", "ZAI_API_KEY"),
        ProviderEnvConfig::new("moonshot", "MOONSHOT_API_KEY"),
        ProviderEnvConfig::new("kimi", "KIMI_API_KEY"),
        ProviderEnvConfig::new("openrouter", "OPENROUTER_API_KEY"),
        ProviderEnvConfig::new("azure", "AZURE_OPENAI_API_KEY"),
        ProviderEnvConfig::new("ollama", "OLLAMA_API_KEY"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::embedded_providers::embedded_providers;
    use std::collections::HashMap;

    #[test]
    fn openrouter_and_azure_are_resolvable() {
        let by_name: HashMap<_, _> = default_providers()
            .into_iter()
            .map(|p| (p.name, p.env_var))
            .collect();
        assert_eq!(
            by_name.get("openrouter").map(String::as_str),
            Some("OPENROUTER_API_KEY"),
            "openrouter must be resolvable via OPENROUTER_API_KEY"
        );
        assert_eq!(
            by_name.get("azure").map(String::as_str),
            Some("AZURE_OPENAI_API_KEY"),
            "azure must be resolvable via AZURE_OPENAI_API_KEY"
        );
    }

    #[test]
    fn every_required_registry_provider_has_a_resolver_entry() {
        // Guards against future drift: every provider that
        // `embedded_providers()` says requires an API key must have a
        // matching env-var declaration in `default_providers()`,
        // otherwise the resolver silently never looks up the user's key.
        let resolver: HashMap<_, _> = default_providers()
            .into_iter()
            .map(|p| (p.name, p.env_var))
            .collect();

        let mut missing: Vec<String> = Vec::new();
        let mut mismatched: Vec<(String, String, String)> = Vec::new();
        for info in embedded_providers() {
            if !info.requires_api_key {
                continue;
            }
            match resolver.get(&info.id) {
                None => missing.push(info.id),
                Some(env_var) if env_var != &info.env_var => {
                    mismatched.push((info.id, info.env_var, env_var.clone()))
                }
                Some(_) => {}
            }
        }

        assert!(
            missing.is_empty(),
            "providers in embedded_providers() but not default_providers(): {missing:?}"
        );
        assert!(
            mismatched.is_empty(),
            "env_var mismatch between embedded_providers() and default_providers(): {mismatched:?}"
        );
    }
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
