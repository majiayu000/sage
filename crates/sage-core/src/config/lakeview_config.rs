//! Configuration for Lakeview integration

use serde::{Deserialize, Serialize};

/// Configuration for Lakeview integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LakeviewConfig {
    /// Model provider for Lakeview
    pub model_provider: String,
    /// Model name for Lakeview
    pub model_name: String,
    /// Lakeview API endpoint
    pub endpoint: Option<String>,
    /// Lakeview API key
    pub api_key: Option<String>,
    /// Whether to enable Lakeview
    pub enabled: bool,
}

impl Default for LakeviewConfig {
    fn default() -> Self {
        Self {
            model_provider: "openai".to_string(),
            model_name: "gpt-4".to_string(),
            endpoint: None,
            api_key: None,
            enabled: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lakeview_config_default() {
        let config = LakeviewConfig::default();
        assert_eq!(config.model_provider, "openai");
        assert_eq!(config.model_name, "gpt-4");
        assert!(!config.enabled);
    }
}
