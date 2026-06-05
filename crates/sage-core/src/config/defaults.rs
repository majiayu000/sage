//! Default configuration loading functions

use crate::config::ModelParameters;
use crate::config::credential::CredentialsFile;
use crate::config::loader::ConfigLoader;
use crate::config::model::Config;
use crate::config::provider_defaults::create_default_providers;
use crate::error::SageResult;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Convenience function to load configuration with default sources
///
/// Loads configuration in this order:
/// 1. Default configuration
/// 2. sage_config.json (if exists)
/// 3. sage_config.toml (if exists)
/// 4. Environment variables
/// 5. Global credentials file (~/.sage/credentials.json)
pub fn load_config() -> SageResult<Config> {
    load_config_with_overrides(None, HashMap::new())
}

/// Load configuration with custom file path
///
/// Loads configuration in this order:
/// 1. Default configuration
/// 2. Custom config file
/// 3. Environment variables
/// 4. Global credentials file (~/.sage/credentials.json)
pub fn load_config_from_file<P: AsRef<Path>>(path: P) -> SageResult<Config> {
    load_config_with_overrides(path.as_ref().to_str(), HashMap::new())
}

/// Load configuration with command line overrides
///
/// Loads configuration in this order:
/// 1. Default configuration
/// 2. Environment variables
/// 3. Config file (if specified, or default files)
/// 4. Global credentials file (~/.sage/credentials.json)
/// 5. Command line overrides
pub fn load_config_with_overrides(
    config_file: Option<&str>,
    overrides: HashMap<String, String>,
) -> SageResult<Config> {
    let mut loader = ConfigLoader::new().with_defaults().with_env();
    let explicit_default_provider = explicit_default_provider_requested(config_file, &overrides);

    if let Some(file) = config_file {
        loader = loader.with_file(file);
    } else {
        loader = loader
            .with_file("sage_config.json")
            .with_file("sage_config.toml");

        if let Some(global_config) = dirs::home_dir().map(|h| h.join(".sage").join("config.json")) {
            if global_config.exists() {
                loader = loader.with_file(global_config);
            }
        }
    }

    let mut config = loader.with_args(overrides).load()?;

    // Load credentials from ~/.sage/credentials.json
    if let Some(creds_path) = dirs::home_dir().map(|h| h.join(".sage").join("credentials.json")) {
        if let Some(creds) = CredentialsFile::load_or_warn(&creds_path) {
            let default_params = create_default_providers();
            // Merge credentials into config
            for (provider, api_key) in creds.api_keys {
                tracing::debug!(
                    "Processing credential for provider '{}': key_len={}",
                    provider,
                    api_key.len()
                );
                // Only add if not already configured
                if !config.model_providers.contains_key(&provider) {
                    let mut params = default_params
                        .get(&provider)
                        .cloned()
                        .unwrap_or_else(ModelParameters::default);
                    params.api_key = Some(api_key.clone());
                    config.model_providers.insert(provider.clone(), params);
                    tracing::debug!("Added new provider '{}' with API key", provider);
                } else if let Some(params) = config.model_providers.get_mut(&provider) {
                    // Update API key if not set or is an env var placeholder
                    let current_key = params.api_key.as_deref().unwrap_or("");
                    let should_update = match &params.api_key {
                        None => true,
                        Some(key) => key.starts_with("${") || key.is_empty(),
                    };
                    tracing::debug!(
                        "Provider '{}' exists: current_key_preview='{}...', should_update={}",
                        provider,
                        if current_key.len() > 8 {
                            &current_key[..8]
                        } else {
                            current_key
                        },
                        should_update
                    );
                    if should_update {
                        params.api_key = Some(api_key.clone());
                        tracing::debug!("Updated API key for provider '{}'", provider);
                    }
                }
            }
            tracing::debug!("Loaded credentials from {}", creds_path.display());
        }
    }

    select_default_provider_with_credentials(&mut config, explicit_default_provider);

    Ok(config)
}

fn explicit_default_provider_requested(
    config_file: Option<&str>,
    overrides: &HashMap<String, String>,
) -> bool {
    if overrides
        .get("provider")
        .is_some_and(|provider| !provider.is_empty())
        || overrides
            .get("default_provider")
            .is_some_and(|provider| !provider.is_empty())
    {
        return true;
    }

    if matches!(std::env::var("SAGE_DEFAULT_PROVIDER"), Ok(provider) if !provider.is_empty()) {
        return true;
    }

    match config_file {
        Some(file) => file_declares_default_provider(Path::new(file)),
        None => default_config_paths()
            .iter()
            .any(|path| file_declares_default_provider(path)),
    }
}

fn default_config_paths() -> Vec<PathBuf> {
    let mut paths = vec![
        PathBuf::from("sage_config.json"),
        PathBuf::from("sage_config.toml"),
    ];

    if let Some(global_config) = dirs::home_dir().map(|h| h.join(".sage").join("config.json")) {
        paths.push(global_config);
    }

    paths
}

fn file_declares_default_provider(path: &Path) -> bool {
    let Ok(content) = fs::read_to_string(path) else {
        return false;
    };

    match path.extension().and_then(|s| s.to_str()) {
        Some("toml") => toml::from_str::<toml::Value>(&content)
            .ok()
            .and_then(|value| {
                value
                    .get("default_provider")
                    .and_then(|provider| provider.as_str().map(str::to_string))
            })
            .is_some_and(|provider| !provider.is_empty()),
        Some("yaml") | Some("yml") => serde_yaml::from_str::<serde_yaml::Value>(&content)
            .ok()
            .and_then(|value| {
                value
                    .get("default_provider")
                    .and_then(|provider| provider.as_str().map(str::to_string))
            })
            .is_some_and(|provider| !provider.is_empty()),
        _ => serde_json::from_str::<serde_json::Value>(&content)
            .ok()
            .and_then(|value| {
                value
                    .get("default_provider")
                    .and_then(|provider| provider.as_str().map(str::to_string))
            })
            .is_some_and(|provider| !provider.is_empty()),
    }
}

fn select_default_provider_with_credentials(config: &mut Config, explicit_default_provider: bool) {
    if explicit_default_provider {
        return;
    }

    let Some(params) = config.model_providers.get(&config.default_provider) else {
        return;
    };

    if params
        .get_api_key_info_for_provider(&config.default_provider)
        .key
        .is_some()
    {
        return;
    }

    if let Some(provider) = config
        .model_providers
        .iter()
        .filter(|(provider, params)| params.get_api_key_info_for_provider(provider).key.is_some())
        .map(|(provider, _)| provider)
        .min()
        .cloned()
    {
        config.default_provider = provider;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::api_key_helpers::get_standard_env_vars_for_provider;
    use serial_test::serial;
    use std::fs;
    use tempfile::TempDir;

    struct EnvVarGuard {
        values: Vec<(String, Option<String>)>,
    }

    impl EnvVarGuard {
        fn remove_provider_vars() -> Self {
            let mut vars = vec![
                "SAGE_DEFAULT_PROVIDER".to_string(),
                "SAGE_MAX_STEPS".to_string(),
                "SAGE_WORKING_DIR".to_string(),
                "SAGE_ENABLE_LAKEVIEW".to_string(),
            ];
            for provider in [
                "openai",
                "zai",
                "anthropic",
                "google",
                "azure",
                "openrouter",
                "doubao",
                "ollama",
                "glm",
                "zhipu",
                "moonshot",
                "kimi",
            ] {
                let prefix = provider.to_uppercase();
                vars.push(format!("SAGE_{prefix}_API_KEY"));
                for suffix in ["API_KEY", "MODEL", "BASE_URL", "TEMPERATURE", "MAX_TOKENS"] {
                    vars.push(format!("{prefix}_{suffix}"));
                }
                vars.extend(get_standard_env_vars_for_provider(provider));
            }

            vars.sort();
            vars.dedup();

            let values = vars
                .into_iter()
                .map(|var| {
                    let value = std::env::var(&var).ok();
                    unsafe {
                        std::env::remove_var(&var);
                    }
                    (var, value)
                })
                .collect();

            Self { values }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            for (var, value) in &self.values {
                unsafe {
                    match value {
                        Some(value) => std::env::set_var(var, value),
                        None => std::env::remove_var(var),
                    }
                }
            }
        }
    }

    #[test]
    #[serial]
    fn test_load_config_from_file() {
        let _env = EnvVarGuard::remove_provider_vars();

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test.json");
        let config_json = r#"{
            "default_provider": "openai",
            "max_steps": 100,
            "enable_lakeview": false,
            "model_providers": {
                "openai": {
                    "model": "gpt-4",
                    "api_key": "test_key"
                }
            },
            "tools": {
                "enabled_tools": ["TaskDone"],
                "max_execution_time": 300,
                "allow_parallel_execution": true,
                "tool_settings": {}
            },
            "logging": {
                "level": "info",
                "format": "json",
                "log_to_console": true,
                "log_to_file": false
            },
            "trajectory": {
                "directory": "./trajectories",
                "auto_save": true,
                "save_interval_steps": 5,
                "enable_compression": true
            },
            "mcp": {
                "enabled": false,
                "servers": {},
                "default_timeout_secs": 300,
                "auto_connect": true
            }
        }"#;
        fs::write(&config_path, config_json).unwrap();

        let config = load_config_from_file(&config_path).unwrap();
        assert_eq!(config.default_provider, "openai");
    }

    #[test]
    #[serial]
    fn test_load_config_with_overrides_no_file() {
        let _env = EnvVarGuard::remove_provider_vars();

        let overrides = HashMap::from([
            ("provider".to_string(), "anthropic".to_string()),
            ("max_steps".to_string(), "50".to_string()),
        ]);

        let config = load_config_with_overrides(None, overrides).unwrap();
        assert_eq!(config.default_provider, "anthropic");
        assert_eq!(config.max_steps, Some(50));
    }

    #[test]
    #[serial]
    fn test_implicit_default_provider_does_not_fallback_to_ollama_without_credentials() {
        let _env = EnvVarGuard::remove_provider_vars();
        let mut config = Config::default();

        select_default_provider_with_credentials(&mut config, false);

        assert_eq!(config.default_provider, "anthropic");
    }

    #[test]
    #[serial]
    fn test_implicit_default_provider_uses_available_credentialed_provider() {
        let _env = EnvVarGuard::remove_provider_vars();
        unsafe {
            std::env::set_var("OPENAI_API_KEY", "sk-test-key");
        }
        let mut config = Config::default();

        select_default_provider_with_credentials(&mut config, false);

        assert_eq!(config.default_provider, "openai");
    }

    #[test]
    #[serial]
    fn test_explicit_ollama_default_provider_is_preserved() {
        let _env = EnvVarGuard::remove_provider_vars();
        unsafe {
            std::env::set_var("OPENAI_API_KEY", "sk-test-key");
        }
        let mut config = Config::default();
        config.default_provider = "ollama".to_string();

        select_default_provider_with_credentials(&mut config, true);

        assert_eq!(config.default_provider, "ollama");
    }

    #[test]
    #[serial]
    fn test_load_config_with_overrides_with_file() {
        let _env = EnvVarGuard::remove_provider_vars();

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test.json");
        let config_json = r#"{
            "default_provider": "openai",
            "max_steps": 100,
            "enable_lakeview": false,
            "model_providers": {
                "openai": {
                    "model": "gpt-4",
                    "api_key": "test_key"
                }
            },
            "tools": {
                "enabled_tools": [],
                "max_execution_time": 300,
                "allow_parallel_execution": true,
                "tool_settings": {}
            },
            "logging": {
                "level": "info",
                "format": "json",
                "log_to_console": true,
                "log_to_file": false
            },
            "trajectory": {
                "directory": "./trajectories",
                "auto_save": true,
                "save_interval_steps": 5,
                "enable_compression": true
            },
            "mcp": {
                "enabled": false,
                "servers": {}
            }
        }"#;
        fs::write(&config_path, config_json).unwrap();

        let overrides = HashMap::from([("max_steps".to_string(), "200".to_string())]);

        let config =
            load_config_with_overrides(Some(config_path.to_str().unwrap()), overrides).unwrap();

        // Override should take precedence
        assert_eq!(config.max_steps, Some(200));
        // File value should still be present
        assert_eq!(config.default_provider, "openai");
    }
}
