//! Configuration loading and management

use crate::config::model::Config;
use crate::error::SageResult;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// Import the delegated loading functions
use super::args_loader;
use super::env_loader;
use super::file_loader;

/// Source of configuration data
#[derive(Debug, Clone)]
pub enum ConfigSource {
    /// Configuration from a file
    File(PathBuf),
    /// Configuration from environment variables
    Environment,
    /// Configuration from command line arguments
    CommandLine(HashMap<String, String>),
    /// Default configuration
    Default,
}

/// Configuration loader with support for multiple sources
pub struct ConfigLoader {
    sources: Vec<ConfigSource>,
}

impl ConfigLoader {
    /// Create a new config loader
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    /// Add a configuration source
    pub fn add_source(mut self, source: ConfigSource) -> Self {
        self.sources.push(source);
        self
    }

    /// Add a file source
    pub fn with_file<P: AsRef<Path>>(self, path: P) -> Self {
        self.add_source(ConfigSource::File(path.as_ref().to_path_buf()))
    }

    /// Add environment variables source
    pub fn with_env(self) -> Self {
        self.add_source(ConfigSource::Environment)
    }

    /// Add command line arguments source
    pub fn with_args(self, args: HashMap<String, String>) -> Self {
        self.add_source(ConfigSource::CommandLine(args))
    }

    /// Add default configuration source
    pub fn with_defaults(self) -> Self {
        self.add_source(ConfigSource::Default)
    }

    /// Load configuration from all sources
    pub fn load(self) -> SageResult<Config> {
        let mut config = Config::default();
        tracing::debug!("Initial config provider: {}", config.default_provider);

        for source in &self.sources {
            let source_config = self.load_from_source(source)?;
            tracing::debug!(
                "Before merge - config provider: {}",
                config.default_provider
            );
            config.merge(source_config);
            tracing::debug!("After merge - config provider: {}", config.default_provider);
        }

        config.validate()?;
        Ok(config)
    }

    /// Load configuration from a specific source
    fn load_from_source(&self, source: &ConfigSource) -> SageResult<Config> {
        match source {
            ConfigSource::File(path) => {
                tracing::debug!("Loading config from file: {}", path.display());
                let config = file_loader::load_from_file(path)?;
                tracing::debug!("File config provider: {}", config.default_provider);
                Ok(config)
            }
            ConfigSource::Environment => {
                tracing::debug!("Loading config from environment");
                let config = env_loader::load_from_env()?;
                tracing::debug!("Env config provider: {}", config.default_provider);
                Ok(config)
            }
            ConfigSource::CommandLine(args) => {
                tracing::debug!("Loading config from command line");
                args_loader::load_from_args(args)
            }
            ConfigSource::Default => {
                tracing::debug!("Loading default config");
                let config = Config::default();
                tracing::debug!("Default config provider: {}", config.default_provider);
                Ok(config)
            }
        }
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_json_config(dir: &TempDir, filename: &str) -> PathBuf {
        let config_path = dir.path().join(filename);
        let config_json = r#"{
            "default_provider": "openai",
            "max_steps": 100,
            "enable_lakeview": false,
            "model_providers": {
                "openai": {
                    "model": "gpt-4",
                    "api_key": "test_key_123",
                    "max_tokens": 4096,
                    "temperature": 0.7
                }
            },
            "tools": {
                "enabled_tools": ["task_done", "bash"],
                "max_execution_time": 300,
                "allow_parallel_execution": true,
                "tool_settings": {}
            },
            "logging": {
                "level": "info",
                "format": "json",
                "log_to_console": true,
                "log_to_file": false,
                "log_file": null
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
        config_path
    }

    fn create_test_toml_config(dir: &TempDir, filename: &str) -> PathBuf {
        let config_path = dir.path().join(filename);
        let config_toml = r#"
default_provider = "anthropic"
max_steps = 50
enable_lakeview = false

[model_providers.anthropic]
model = "claude-3"
api_key = "test_anthropic_key"
max_tokens = 8192
temperature = 0.5

[tools]
enabled_tools = ["task_done", "bash"]
max_execution_time = 600
allow_parallel_execution = true
tool_settings = {}

[logging]
level = "debug"
format = "pretty"
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
        config_path
    }

    #[test]
    fn test_config_loader_new() {
        let loader = ConfigLoader::new();
        assert_eq!(loader.sources.len(), 0);
    }

    #[test]
    fn test_config_loader_with_defaults() {
        let loader = ConfigLoader::new().with_defaults();
        assert_eq!(loader.sources.len(), 1);
    }

    #[test]
    fn test_config_loader_with_env() {
        let loader = ConfigLoader::new().with_env();
        assert_eq!(loader.sources.len(), 1);
    }

    #[test]
    fn test_config_loader_with_args() {
        let args = HashMap::from([("provider".to_string(), "openai".to_string())]);
        let loader = ConfigLoader::new().with_args(args);
        assert_eq!(loader.sources.len(), 1);
    }

    #[test]
    fn test_config_loader_multiple_sources() {
        let loader = ConfigLoader::new()
            .with_defaults()
            .with_env()
            .with_file("test.json");
        assert_eq!(loader.sources.len(), 3);
    }

    #[test]
    fn test_load_config_from_json_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_json_config(&temp_dir, "test_config.json");

        let config = ConfigLoader::new()
            .with_defaults()
            .with_file(&config_path)
            .load()
            .unwrap();

        assert_eq!(config.default_provider, "openai");
        assert_eq!(config.max_steps, Some(100));
        assert!(config.model_providers.contains_key("openai"));

        let openai_params = &config.model_providers["openai"];
        assert_eq!(openai_params.model, "gpt-4");
        assert_eq!(openai_params.api_key, Some("test_key_123".to_string()));
    }

    #[test]
    fn test_load_config_from_toml_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_toml_config(&temp_dir, "test_config.toml");

        let config = ConfigLoader::new()
            .with_defaults()
            .with_file(&config_path)
            .load()
            .unwrap();

        assert_eq!(config.default_provider, "anthropic");
        assert_eq!(config.max_steps, Some(50));
        assert!(config.model_providers.contains_key("anthropic"));

        let anthropic_params = &config.model_providers["anthropic"];
        assert_eq!(anthropic_params.model, "claude-3");
    }

    #[test]
    fn test_load_config_from_nonexistent_file() {
        // Loading from a non-existent file should use defaults
        let config = ConfigLoader::new()
            .with_defaults()
            .with_file("/nonexistent/path/config.json")
            .load()
            .unwrap();

        // Should have default values
        assert!(!config.default_provider.is_empty());
    }

    #[test]
    fn test_load_config_from_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("invalid.json");
        fs::write(&config_path, "{ invalid json }").unwrap();

        let result = ConfigLoader::new()
            .with_defaults()
            .with_file(&config_path)
            .load();

        assert!(result.is_err());
    }

    #[test]
    fn test_load_config_from_env() {
        // Set environment variables
        unsafe {
            std::env::set_var("SAGE_DEFAULT_PROVIDER", "google");
            std::env::set_var("SAGE_MAX_STEPS", "75");
            std::env::set_var("OPENAI_API_KEY", "env_test_key");
            std::env::set_var("OPENAI_MODEL", "gpt-4-turbo");
        }

        let config = ConfigLoader::new()
            .with_defaults()
            .with_env()
            .load()
            .unwrap();

        // Provider from env should override default
        assert_eq!(config.default_provider, "google");
        assert_eq!(config.max_steps, Some(75));

        // Check if openai provider was loaded from env
        if let Some(openai_params) = config.model_providers.get("openai") {
            assert_eq!(openai_params.model, "gpt-4-turbo");
        }

        // Clean up
        unsafe {
            std::env::remove_var("SAGE_DEFAULT_PROVIDER");
            std::env::remove_var("SAGE_MAX_STEPS");
            std::env::remove_var("OPENAI_API_KEY");
            std::env::remove_var("OPENAI_MODEL");
        }
    }

    #[test]
    fn test_load_config_from_args() {
        let args = HashMap::from([
            ("provider".to_string(), "anthropic".to_string()),
            ("max_steps".to_string(), "25".to_string()),
        ]);

        let config = ConfigLoader::new()
            .with_defaults()
            .with_args(args)
            .load()
            .unwrap();

        assert_eq!(config.default_provider, "anthropic");
        assert_eq!(config.max_steps, Some(25));
    }

    #[test]
    fn test_load_config_args_override_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_json_config(&temp_dir, "test_config.json");

        let args = HashMap::from([
            ("provider".to_string(), "google".to_string()),
            ("max_steps".to_string(), "200".to_string()),
        ]);

        let config = ConfigLoader::new()
            .with_defaults()
            .with_file(&config_path)
            .with_args(args)
            .load()
            .unwrap();

        // Args should override file values
        assert_eq!(config.default_provider, "google");
        assert_eq!(config.max_steps, Some(200));
    }

    #[test]
    fn test_load_config_with_invalid_max_steps() {
        let args = HashMap::from([("max_steps".to_string(), "invalid".to_string())]);

        let result = ConfigLoader::new().with_defaults().with_args(args).load();

        assert!(result.is_err());
    }

    #[test]
    fn test_load_provider_from_env_temperature() {
        unsafe {
            std::env::set_var("ANTHROPIC_API_KEY", "test_key");
            std::env::set_var("ANTHROPIC_MODEL", "claude-3");
            std::env::set_var("ANTHROPIC_TEMPERATURE", "0.9");
            std::env::set_var("ANTHROPIC_MAX_TOKENS", "8192");
        }

        let config = ConfigLoader::new()
            .with_defaults()
            .with_env()
            .load()
            .unwrap();

        if let Some(params) = config.model_providers.get("anthropic") {
            assert_eq!(params.temperature, Some(0.9));
            assert_eq!(params.max_tokens, Some(8192));
        }

        // Clean up
        unsafe {
            std::env::remove_var("ANTHROPIC_API_KEY");
            std::env::remove_var("ANTHROPIC_MODEL");
            std::env::remove_var("ANTHROPIC_TEMPERATURE");
            std::env::remove_var("ANTHROPIC_MAX_TOKENS");
        }
    }

    #[test]
    fn test_load_provider_from_env_invalid_temperature() {
        unsafe {
            std::env::set_var("GOOGLE_API_KEY", "test_key");
            std::env::set_var("GOOGLE_TEMPERATURE", "invalid");
        }

        let result = ConfigLoader::new().with_defaults().with_env().load();

        assert!(result.is_err());

        // Clean up
        unsafe {
            std::env::remove_var("GOOGLE_API_KEY");
            std::env::remove_var("GOOGLE_TEMPERATURE");
        }
    }

    #[test]
    fn test_load_provider_from_env_base_url() {
        // Clean up any leftover env vars from other tests that might run in parallel
        unsafe {
            std::env::remove_var("GOOGLE_TEMPERATURE");
            std::env::set_var("OLLAMA_BASE_URL", "http://localhost:11434");
            std::env::set_var("OLLAMA_MODEL", "llama2");
        }

        let config = ConfigLoader::new()
            .with_defaults()
            .with_env()
            .load()
            .unwrap();

        if let Some(params) = config.model_providers.get("ollama") {
            assert_eq!(params.base_url, Some("http://localhost:11434".to_string()));
        }

        // Clean up
        unsafe {
            std::env::remove_var("OLLAMA_BASE_URL");
            std::env::remove_var("OLLAMA_MODEL");
        }
    }

    #[test]
    fn test_config_source_debug() {
        let source = ConfigSource::File(PathBuf::from("/test/path"));
        assert!(format!("{:?}", source).contains("File"));

        let source = ConfigSource::Environment;
        assert!(format!("{:?}", source).contains("Environment"));
    }

    #[test]
    fn test_load_config_with_working_directory() {
        let temp_dir = TempDir::new().unwrap();
        let args = HashMap::from([(
            "working_dir".to_string(),
            temp_dir.path().to_str().unwrap().to_string(),
        )]);

        let config = ConfigLoader::new()
            .with_defaults()
            .with_args(args)
            .load()
            .unwrap();

        assert_eq!(
            config.working_directory,
            Some(temp_dir.path().to_path_buf())
        );
    }

    #[test]
    fn test_convenience_load_config_from_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_json_config(&temp_dir, "test.json");

        let config = super::super::defaults::load_config_from_file(&config_path).unwrap();
        assert_eq!(config.default_provider, "openai");
    }

    #[test]
    fn test_config_loader_default() {
        let loader = ConfigLoader::default();
        assert_eq!(loader.sources.len(), 0);
    }

    #[test]
    fn test_merge_multiple_sources() {
        let temp_dir = TempDir::new().unwrap();

        // Create first config with provider "openai"
        let config1_path = create_test_json_config(&temp_dir, "config1.json");

        // Create second config override
        let config2_path = temp_dir.path().join("config2.json");
        let config2_json = r#"{
            "default_provider": "anthropic",
            "max_steps": 200,
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
        fs::write(&config2_path, config2_json).unwrap();

        // Second config should override first
        let config = ConfigLoader::new()
            .with_defaults()
            .with_file(&config1_path)
            .with_file(&config2_path)
            .load()
            .unwrap();

        assert_eq!(config.default_provider, "anthropic");
        assert_eq!(config.max_steps, Some(200));
    }

    #[test]
    fn test_load_from_yaml_extension() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        let yaml_content = r#"
default_provider: openai
max_steps: 50
enable_lakeview: false
model_providers:
  openai:
    model: gpt-4
    api_key: test_yaml_key
    max_tokens: 4096
tools:
  enabled_tools:
    - task_done
    - bash
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

        let config = ConfigLoader::new()
            .with_defaults()
            .with_file(&config_path)
            .load()
            .unwrap();

        assert_eq!(config.default_provider, "openai");
        assert_eq!(config.max_steps, Some(50));
    }

    #[test]
    fn test_load_config_with_lakeview_enabled() {
        unsafe {
            std::env::set_var("SAGE_ENABLE_LAKEVIEW", "true");
        }

        let config = ConfigLoader::new()
            .with_defaults()
            .with_env()
            .load()
            .unwrap();

        assert!(config.enable_lakeview);

        // Clean up
        unsafe {
            std::env::remove_var("SAGE_ENABLE_LAKEVIEW");
        }
    }

    #[test]
    fn test_load_config_with_model_and_api_key_from_args() {
        let args = HashMap::from([
            ("provider".to_string(), "openai".to_string()),
            ("model".to_string(), "gpt-4-turbo".to_string()),
            ("api_key".to_string(), "test_api_key_from_args".to_string()),
        ]);

        let config = ConfigLoader::new()
            .with_defaults()
            .with_args(args)
            .load()
            .unwrap();

        assert_eq!(config.default_provider, "openai");

        if let Some(params) = config.model_providers.get("openai") {
            assert_eq!(params.model, "gpt-4-turbo");
            assert_eq!(params.api_key, Some("test_api_key_from_args".to_string()));
        } else {
            panic!("OpenAI provider should be configured from args");
        }
    }

    #[test]
    fn test_load_config_with_base_url_from_args() {
        let args = HashMap::from([
            ("provider".to_string(), "ollama".to_string()),
            (
                "model_base_url".to_string(),
                "http://custom-host:8080".to_string(),
            ),
        ]);

        let config = ConfigLoader::new()
            .with_defaults()
            .with_args(args)
            .load()
            .unwrap();

        if let Some(params) = config.model_providers.get("ollama") {
            assert_eq!(params.base_url, Some("http://custom-host:8080".to_string()));
        }
    }

    #[test]
    fn test_config_source_clone() {
        let source = ConfigSource::File(PathBuf::from("/test/path"));
        let cloned = source.clone();
        assert!(matches!(cloned, ConfigSource::File(_)));
    }

    #[test]
    fn test_load_provider_from_env_all_providers() {
        // Test loading multiple providers from environment
        unsafe {
            std::env::set_var("OPENAI_API_KEY", "openai_key");
            std::env::set_var("OPENAI_MODEL", "gpt-4");
            std::env::set_var("ANTHROPIC_API_KEY", "anthropic_key");
            std::env::set_var("ANTHROPIC_MODEL", "claude-3");
            std::env::set_var("GOOGLE_API_KEY", "google_key");
            std::env::set_var("GOOGLE_MODEL", "gemini-pro");
        }

        let config = ConfigLoader::new()
            .with_defaults()
            .with_env()
            .load()
            .unwrap();

        assert!(config.model_providers.contains_key("openai"));
        assert!(config.model_providers.contains_key("anthropic"));
        assert!(config.model_providers.contains_key("google"));

        // Clean up
        unsafe {
            std::env::remove_var("OPENAI_API_KEY");
            std::env::remove_var("OPENAI_MODEL");
            std::env::remove_var("ANTHROPIC_API_KEY");
            std::env::remove_var("ANTHROPIC_MODEL");
            std::env::remove_var("GOOGLE_API_KEY");
            std::env::remove_var("GOOGLE_MODEL");
        }
    }
}
