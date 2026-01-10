//! API key types and resolution

/// Source of the API key
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiKeySource {
    /// From configuration file
    ConfigFile,
    /// From SAGE_<PROVIDER>_API_KEY environment variable
    SageEnvVar,
    /// From standard environment variable (e.g., ANTHROPIC_API_KEY)
    StandardEnvVar,
    /// No API key found
    NotFound,
}

impl std::fmt::Display for ApiKeySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiKeySource::ConfigFile => write!(f, "config file"),
            ApiKeySource::SageEnvVar => write!(f, "SAGE_*_API_KEY env"),
            ApiKeySource::StandardEnvVar => write!(f, "env variable"),
            ApiKeySource::NotFound => write!(f, "not found"),
        }
    }
}

/// Result of API key resolution with source information
#[derive(Debug, Clone)]
pub struct ApiKeyInfo {
    /// The API key value (if found)
    pub key: Option<String>,
    /// Where the key was found
    pub source: ApiKeySource,
    /// The environment variable name that was used (if from env)
    pub env_var_name: Option<String>,
}

impl ApiKeyInfo {
    /// Check if a valid API key was found
    pub fn is_valid(&self) -> bool {
        self.key.is_some()
    }

    /// Get a display-safe version (masked) of the API key
    pub fn masked_key(&self) -> Option<String> {
        self.key.as_ref().map(|k| mask_api_key(k))
    }
}

/// Get standard environment variable names for a provider
pub fn get_standard_env_vars(provider: &str) -> Vec<String> {
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
            vec![format!("{}_API_KEY", provider.to_uppercase())]
        }
    }
}

/// Mask an API key for safe display
pub fn mask_api_key(key: &str) -> String {
    let len = key.len();
    if len <= 12 {
        return "*".repeat(len);
    }

    let prefix = &key[..8];
    let suffix = &key[len - 4..];
    let mask_len = len - 12;

    format!("{}{}...{}", prefix, "*".repeat(mask_len.min(8)), suffix)
}

/// Display API key status for CLI
pub fn format_api_key_status(provider: &str, info: &ApiKeyInfo) -> String {
    match &info.source {
        ApiKeySource::ConfigFile => {
            format!(
                "✓ {} API key (from config): {}",
                provider,
                info.masked_key().unwrap_or_default()
            )
        }
        ApiKeySource::SageEnvVar => {
            format!(
                "✓ {} API key (from {}): {}",
                provider,
                info.env_var_name.as_deref().unwrap_or("env"),
                info.masked_key().unwrap_or_default()
            )
        }
        ApiKeySource::StandardEnvVar => {
            format!(
                "✓ {} API key (from {}): {}",
                provider,
                info.env_var_name.as_deref().unwrap_or("env"),
                info.masked_key().unwrap_or_default()
            )
        }
        ApiKeySource::NotFound => {
            let env_hints = get_standard_env_vars(provider);
            format!(
                "✗ {} API key missing. Set {} or add to config",
                provider,
                env_hints.first().cloned().unwrap_or_default()
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_api_key() {
        assert_eq!(
            mask_api_key("sk-ant-api03-abc123xyz789"),
            "sk-ant-a********...z789"
        );
        assert_eq!(mask_api_key("short"), "*****");
        assert_eq!(mask_api_key("exactly12ch"), "***********");
    }

    #[test]
    fn test_api_key_source_display() {
        assert_eq!(ApiKeySource::ConfigFile.to_string(), "config file");
        assert_eq!(ApiKeySource::SageEnvVar.to_string(), "SAGE_*_API_KEY env");
        assert_eq!(ApiKeySource::StandardEnvVar.to_string(), "env variable");
        assert_eq!(ApiKeySource::NotFound.to_string(), "not found");
    }

    #[test]
    fn test_get_standard_env_vars() {
        assert!(get_standard_env_vars("anthropic").contains(&"ANTHROPIC_API_KEY".to_string()));
        assert!(get_standard_env_vars("openai").contains(&"OPENAI_API_KEY".to_string()));
        assert!(get_standard_env_vars("google").contains(&"GOOGLE_API_KEY".to_string()));
        assert!(get_standard_env_vars("custom").contains(&"CUSTOM_API_KEY".to_string()));
    }
}
