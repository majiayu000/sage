//! Configuration saving for onboarding
//!
//! This module handles saving credentials and configuration during onboarding.

use crate::config::ModelParameters;
use crate::config::persistence::ConfigPersistence;
use crate::error::{SageError, SageResult};
use std::path::Path;
use tracing::info;

/// Save credentials to the credentials file
pub fn save_credentials(global_dir: &Path, provider: &str, api_key: &str) -> SageResult<()> {
    let persistence = ConfigPersistence::new(global_dir);
    persistence.set_api_key(provider, api_key)?;

    info!(
        "Saved {} credentials to {}",
        provider,
        persistence.credentials_path().display()
    );

    Ok(())
}

/// Save global configuration for a provider
pub fn save_global_config(global_dir: &Path, provider: &str) -> SageResult<()> {
    let persistence = ConfigPersistence::new(global_dir);
    let mut params = create_provider_params(provider);
    if params.api_key.is_none() {
        params.api_key = Some(format!("${{{}_API_KEY}}", provider.to_uppercase()));
    }

    let provider_value = serde_json::to_value(params)
        .map_err(|error| SageError::config(format!("Failed to serialize provider: {}", error)))?;
    persistence.set_default_provider(provider)?;
    persistence.set_field(&format!("model_providers.{}", provider), provider_value)?;

    info!(
        "Saved global config to {}",
        persistence.config_path().display()
    );
    Ok(())
}

/// Create model parameters for a provider
fn create_provider_params(provider: &str) -> ModelParameters {
    let mut params = ModelParameters::default();
    apply_provider_defaults(provider, &mut params);
    params
}

/// Apply provider-specific defaults
fn apply_provider_defaults(provider: &str, params: &mut ModelParameters) {
    if provider == "glm" || provider == "zhipu" {
        params.model = "glm-4.7".to_string();
        params.base_url = Some("https://open.bigmodel.cn/api/anthropic".to_string());
        params.api_version = Some("2023-06-01".to_string());
        params.parallel_tool_calls = Some(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn save_credentials_does_not_overwrite_invalid_credentials_file()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let creds_path = dir.path().join("credentials.json");
        let invalid = "{not valid json";
        std::fs::write(&creds_path, invalid)?;

        let result = save_credentials(dir.path(), "openai", "sk-test");

        assert!(result.is_err());
        assert_eq!(std::fs::read_to_string(&creds_path)?, invalid);
        Ok(())
    }
}
