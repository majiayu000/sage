//! CLI-provided configuration overrides
//!
//! This module provides the CliOverrides type for command-line configuration.

/// CLI-provided configuration overrides
#[derive(Debug, Clone, Default)]
pub struct CliOverrides {
    /// Provider specified via CLI
    pub provider: Option<String>,
    /// Model specified via CLI
    pub model: Option<String>,
    /// API key specified via CLI
    pub api_key: Option<String>,
    /// Max steps specified via CLI
    pub max_steps: Option<u32>,
}

impl CliOverrides {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = Some(provider.into());
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    pub fn with_max_steps(mut self, max_steps: u32) -> Self {
        self.max_steps = Some(max_steps);
        self
    }

    /// Check if any overrides are set
    pub fn has_overrides(&self) -> bool {
        self.provider.is_some()
            || self.model.is_some()
            || self.api_key.is_some()
            || self.max_steps.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_overrides_default() {
        let overrides = CliOverrides::default();
        assert!(!overrides.has_overrides());
    }

    #[test]
    fn test_cli_overrides_builder() {
        let overrides = CliOverrides::new()
            .with_provider("openai")
            .with_model("gpt-4")
            .with_api_key("test-key")
            .with_max_steps(50);

        assert!(overrides.has_overrides());
        assert_eq!(overrides.provider, Some("openai".to_string()));
        assert_eq!(overrides.model, Some("gpt-4".to_string()));
        assert_eq!(overrides.api_key, Some("test-key".to_string()));
        assert_eq!(overrides.max_steps, Some(50));
    }
}
