//! Default configuration loading functions

use crate::config::loader::ConfigLoader;
use crate::config::model::Config;
use crate::error::SageResult;
use std::collections::HashMap;
use std::path::Path;

/// Convenience function to load configuration with default sources
///
/// Loads configuration in this order:
/// 1. Default configuration
/// 2. sage_config.json (if exists)
/// 3. sage_config.toml (if exists)
/// 4. Environment variables
pub fn load_config() -> SageResult<Config> {
    ConfigLoader::new()
        .with_defaults()
        .with_file("sage_config.json")
        .with_file("sage_config.toml")
        .with_env()
        .load()
}

/// Load configuration with custom file path
///
/// Loads configuration in this order:
/// 1. Default configuration
/// 2. Custom config file
/// 3. Environment variables
pub fn load_config_from_file<P: AsRef<Path>>(path: P) -> SageResult<Config> {
    ConfigLoader::new()
        .with_defaults()
        .with_file(path)
        .with_env()
        .load()
}

/// Load configuration with command line overrides
///
/// Loads configuration in this order:
/// 1. Default configuration
/// 2. Environment variables
/// 3. Config file (if specified, or default files)
/// 4. Command line overrides
pub fn load_config_with_overrides(
    config_file: Option<&str>,
    overrides: HashMap<String, String>,
) -> SageResult<Config> {
    let mut loader = ConfigLoader::new().with_defaults().with_env();

    if let Some(file) = config_file {
        loader = loader.with_file(file);
    } else {
        loader = loader
            .with_file("sage_config.json")
            .with_file("sage_config.toml");
    }

    loader.with_args(overrides).load()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_load_config_from_file() {
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
                "enabled_tools": ["task_done"],
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
    fn test_load_config_with_overrides_no_file() {
        let overrides = HashMap::from([
            ("provider".to_string(), "anthropic".to_string()),
            ("max_steps".to_string(), "50".to_string()),
        ]);

        let config = load_config_with_overrides(None, overrides).unwrap();
        assert_eq!(config.default_provider, "anthropic");
        assert_eq!(config.max_steps, Some(50));
    }

    #[test]
    fn test_load_config_with_overrides_with_file() {
        // Clear any interfering environment variables from other tests
        // SAFETY: This is a single-threaded test environment
        unsafe {
            std::env::remove_var("GOOGLE_TEMPERATURE");
        }

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test.json");
        let config_json = r#"{
            "default_provider": "openai",
            "max_steps": 100,
            "enable_lakeview": false,
            "model_providers": {},
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
