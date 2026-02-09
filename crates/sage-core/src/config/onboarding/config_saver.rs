//! Configuration saving for onboarding
//!
//! This module handles saving credentials and configuration during onboarding.

use crate::config::Config;
use crate::config::ModelParameters;
use crate::config::credential::CredentialsFile;
use crate::error::{SageError, SageResult};
use std::path::Path;
use tracing::info;

/// Save credentials to the credentials file
pub fn save_credentials(global_dir: &Path, provider: &str, api_key: &str) -> SageResult<()> {
    let creds_path = global_dir.join("credentials.json");
    let mut creds = CredentialsFile::load(&creds_path).unwrap_or_default();
    creds.set_api_key(provider, api_key);

    creds
        .save(&creds_path)
        .map_err(|e| SageError::config(format!("Failed to save credentials: {}", e)))?;

    info!("Saved {} credentials to {}", provider, creds_path.display());

    Ok(())
}

/// Save global configuration for a provider
pub fn save_global_config(global_dir: &Path, provider: &str) -> SageResult<()> {
    let config_path = global_dir.join("config.json");
    let mut config = Config::default();

    if !config.model_providers.contains_key(provider) {
        let params = create_provider_params(provider);
        config.model_providers.insert(provider.to_string(), params);
    }

    config.set_default_provider(provider.to_string())?;

    if let Some(params) = config.model_providers.get_mut(provider) {
        apply_provider_defaults(provider, params);
        if params.api_key.is_none() {
            params.api_key = Some(format!("${{{}_API_KEY}}", provider.to_uppercase()));
        }
    }

    std::fs::create_dir_all(global_dir)
        .map_err(|e| SageError::config(format!("Failed to create config directory: {}", e)))?;

    let config_json = serde_json::to_string_pretty(&config)
        .map_err(|e| SageError::config(format!("Failed to serialize config: {}", e)))?;

    std::fs::write(&config_path, config_json)
        .map_err(|e| SageError::config(format!("Failed to save config: {}", e)))?;

    info!("Saved global config to {}", config_path.display());
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
