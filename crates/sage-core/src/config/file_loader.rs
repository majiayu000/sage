//! File-based configuration loading

use crate::config::model::Config;
use crate::error::{SageError, SageResult};
use std::fs;
use std::path::Path;

/// Load configuration from a file
///
/// Supports JSON, TOML, and YAML formats based on file extension.
/// Returns default config if file doesn't exist.
pub fn load_from_file(path: &Path) -> SageResult<Config> {
    if !path.exists() {
        return Ok(Config::default());
    }

    let content = fs::read_to_string(path).map_err(|e| {
        SageError::config_with_context(
            format!("Failed to read config file: {}", e),
            format!("Reading configuration from '{}'", path.display()),
        )
    })?;

    let config: Config = match path.extension().and_then(|s| s.to_str()) {
        Some("toml") => toml::from_str(&content).map_err(|e| {
            SageError::config_with_context(
                format!("Failed to parse TOML config: {}", e),
                format!("Deserializing TOML configuration from '{}'", path.display()),
            )
        })?,
        Some("yaml") | Some("yml") => serde_yaml::from_str(&content).map_err(|e| {
            SageError::config_with_context(
                format!("Failed to parse YAML config: {}", e),
                format!("Deserializing YAML configuration from '{}'", path.display()),
            )
        })?,
        _ => serde_json::from_str(&content).map_err(|e| {
            SageError::config_with_context(
                format!("Failed to parse JSON config: {}", e),
                format!("Deserializing JSON configuration from '{}'", path.display()),
            )
        })?,
    };

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_load_from_json_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test.json");
        let config_json = r#"{
            "default_provider": "openai",
            "max_steps": 100,
            "enable_lakeview": false,
            "model_providers": {
                "openai": {
                    "model": "gpt-4",
                    "api_key": "test_key",
                    "max_tokens": 4096
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

        let config = load_from_file(&config_path).unwrap();
        assert_eq!(config.default_provider, "openai");
        assert_eq!(config.max_steps, Some(100));
    }

    #[test]
    fn test_load_from_toml_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test.toml");
        let config_toml = r#"
default_provider = "anthropic"
max_steps = 50
enable_lakeview = false

[model_providers.anthropic]
model = "claude-3"
api_key = "test_key"

[tools]
enabled_tools = ["task_done"]
max_execution_time = 300
allow_parallel_execution = true
tool_settings = {}

[logging]
level = "info"
format = "json"
log_to_console = true
log_to_file = false

[trajectory]
directory = "./trajectories"
auto_save = true
save_interval_steps = 5
enable_compression = true

[mcp]
enabled = false
default_timeout_secs = 300
auto_connect = true
"#;
        fs::write(&config_path, config_toml).unwrap();

        let config = load_from_file(&config_path).unwrap();
        assert_eq!(config.default_provider, "anthropic");
        assert_eq!(config.max_steps, Some(50));
    }

    #[test]
    fn test_load_from_yaml_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test.yaml");
        let yaml_content = r#"
default_provider: openai
max_steps: 50
enable_lakeview: false
model_providers:
  openai:
    model: gpt-4
    api_key: test_key
tools:
  enabled_tools:
    - task_done
  max_execution_time: 300
  allow_parallel_execution: true
  tool_settings: {}
logging:
  level: info
  format: json
  log_to_console: true
  log_to_file: false
trajectory:
  directory: ./trajectories
  auto_save: true
  save_interval_steps: 5
  enable_compression: true
mcp:
  enabled: false
  servers: {}
  default_timeout_secs: 300
  auto_connect: true
"#;
        fs::write(&config_path, yaml_content).unwrap();

        let config = load_from_file(&config_path).unwrap();
        assert_eq!(config.default_provider, "openai");
        assert_eq!(config.max_steps, Some(50));
    }

    #[test]
    fn test_load_from_nonexistent_file() {
        let config = load_from_file(Path::new("/nonexistent/config.json")).unwrap();
        assert!(!config.default_provider.is_empty());
    }

    #[test]
    fn test_load_from_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("invalid.json");
        fs::write(&config_path, "{ invalid json }").unwrap();

        let result = load_from_file(&config_path);
        assert!(result.is_err());
    }
}
