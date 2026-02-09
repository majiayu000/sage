//! Provider options for onboarding
//!
//! This module defines the available LLM providers and their configuration.

/// Available providers for onboarding
#[derive(Debug, Clone)]
pub struct ProviderOption {
    /// Provider identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
    /// Whether this provider is recommended
    pub recommended: bool,
    /// URL to get an API key
    pub api_key_url: String,
}

impl ProviderOption {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        api_key_url: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            recommended: false,
            api_key_url: api_key_url.into(),
        }
    }

    pub fn recommended(mut self) -> Self {
        self.recommended = true;
        self
    }
}

/// Get the default provider options
pub fn default_provider_options() -> Vec<ProviderOption> {
    vec![
        ProviderOption::new(
            "anthropic",
            "Anthropic (Claude)",
            "Claude models - excellent for code generation and analysis",
            "https://console.anthropic.com/account/keys",
        )
        .recommended(),
        ProviderOption::new(
            "openai",
            "OpenAI (GPT)",
            "GPT-4 and GPT-3.5 models - widely used and well-documented",
            "https://platform.openai.com/api-keys",
        ),
        ProviderOption::new(
            "google",
            "Google (Gemini)",
            "Gemini models - multimodal capabilities",
            "https://makersuite.google.com/app/apikey",
        ),
        ProviderOption::new(
            "glm",
            "智谱AI (GLM)",
            "GLM-4 models - powerful Chinese and English capabilities",
            "https://open.bigmodel.cn/",
        ),
        ProviderOption::new(
            "ollama",
            "Ollama (Local)",
            "Run models locally - no API key required",
            "https://ollama.ai/",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_option_new() {
        let opt = ProviderOption::new(
            "test",
            "Test Provider",
            "A test provider",
            "https://test.com",
        );
        assert_eq!(opt.id, "test");
        assert_eq!(opt.name, "Test Provider");
        assert!(!opt.recommended);
    }

    #[test]
    fn test_provider_option_recommended() {
        let opt = ProviderOption::new("test", "Test", "Desc", "url").recommended();
        assert!(opt.recommended);
    }

    #[test]
    fn test_default_provider_options() {
        let providers = default_provider_options();
        assert!(providers.len() >= 5);

        let anthropic = providers.iter().find(|p| p.id == "anthropic");
        assert!(anthropic.is_some());
        assert!(anthropic.unwrap().recommended);

        // Check GLM provider exists
        let glm = providers.iter().find(|p| p.id == "glm");
        assert!(glm.is_some());
        assert_eq!(glm.unwrap().name, "智谱AI (GLM)");
    }
}
