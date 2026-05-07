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
        ProviderEnvConfig::new("zai", "ZAI_API_KEY"),
        ProviderEnvConfig::new("moonshot", "MOONSHOT_API_KEY"),
        ProviderEnvConfig::new("kimi", "KIMI_API_KEY"),
        ProviderEnvConfig::new("ollama", "OLLAMA_API_KEY"),
    ]
}

/// Paths to check for auto-import from other tools.
///
/// Returns an empty Vec if the user\'s home directory cannot be
/// determined. Previously this fell back to `dirs::home_dir()
/// .unwrap_or_default()`, which silently produced relative paths like
/// `./.claude/credentials.json` — so on a host without `HOME` the
/// resolver would either find a stray local directory or, worse,
/// import a sibling file that happens to exist in cwd. Returning an
/// empty list with a warn-level log makes the failure mode explicit.
pub fn auto_import_paths() -> Vec<(String, PathBuf)> {
    let Some(home) = dirs::home_dir() else {
        tracing::warn!(
            "Could not determine home directory; auto-import from Claude Code / \
             Cursor / Aider is disabled. Set HOME (Unix) or USERPROFILE (Windows) \
             to enable."
        );
        return Vec::new();
    };
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
