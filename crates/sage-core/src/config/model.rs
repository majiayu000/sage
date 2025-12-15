//! Configuration data models

use crate::error::{SageError, SageResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Model parameters for LLM providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelParameters {
    /// Model name/ID
    pub model: String,
    /// API key for the provider
    pub api_key: Option<String>,
    /// Maximum tokens to generate
    pub max_tokens: Option<u32>,
    /// Temperature (0.0 to 2.0)
    pub temperature: Option<f32>,
    /// Top-p sampling
    pub top_p: Option<f32>,
    /// Top-k sampling (for supported models)
    pub top_k: Option<u32>,
    /// Whether to enable parallel tool calls
    pub parallel_tool_calls: Option<bool>,
    /// Maximum retries for failed requests
    pub max_retries: Option<u32>,
    /// Base URL for the API
    pub base_url: Option<String>,
    /// API version
    pub api_version: Option<String>,
    /// Stop sequences
    pub stop_sequences: Option<Vec<String>>,
}

impl Default for ModelParameters {
    fn default() -> Self {
        Self {
            model: "gpt-4".to_string(),
            api_key: None,
            max_tokens: Some(4096),
            temperature: Some(0.7),
            top_p: Some(1.0),
            top_k: None,
            parallel_tool_calls: Some(true),
            max_retries: Some(3),
            base_url: None,
            api_version: None,
            stop_sequences: None,
        }
    }
}

impl ModelParameters {
    /// Get API key from environment or config
    pub fn get_api_key(&self) -> Option<String> {
        self.api_key.clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
            .or_else(|| std::env::var("GOOGLE_API_KEY").ok())
    }

    /// Get base URL for the provider
    pub fn get_base_url(&self) -> String {
        if let Some(base_url) = &self.base_url {
            base_url.clone()
        } else {
            // Default base URLs for different providers
            // Note: This is a fallback, provider should be determined by context
            "https://api.openai.com/v1".to_string()
        }
    }

    /// Get base URL for a specific provider
    pub fn get_base_url_for_provider(&self, provider: &str) -> String {
        if let Some(base_url) = &self.base_url {
            base_url.clone()
        } else {
            match provider {
                "openai" => "https://api.openai.com/v1".to_string(),
                "anthropic" => "https://api.anthropic.com".to_string(),
                "google" => "https://generativelanguage.googleapis.com".to_string(),
                "ollama" => "http://localhost:11434".to_string(),
                _ => "http://localhost:8000".to_string(),
            }
        }
    }

    /// Convert to LLM model parameters
    pub fn to_llm_parameters(&self) -> crate::llm::providers::ModelParameters {
        crate::llm::providers::ModelParameters {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            top_p: self.top_p,
            top_k: self.top_k,
            stop: self.stop_sequences.clone(),
            parallel_tool_calls: self.parallel_tool_calls,
            frequency_penalty: None,
            presence_penalty: None,
            seed: None,
        }
    }

    /// Validate the model parameters
    pub fn validate(&self) -> SageResult<()> {
        if self.model.is_empty() {
            return Err(SageError::config("Model name cannot be empty"));
        }

        if let Some(temp) = self.temperature {
            if temp < 0.0 || temp > 2.0 {
                return Err(SageError::config("Temperature must be between 0.0 and 2.0"));
            }
        }

        if let Some(top_p) = self.top_p {
            if top_p < 0.0 || top_p > 1.0 {
                return Err(SageError::config("Top-p must be between 0.0 and 1.0"));
            }
        }

        if let Some(max_tokens) = self.max_tokens {
            if max_tokens == 0 {
                return Err(SageError::config("Max tokens must be greater than 0"));
            }
        }

        Ok(())
    }
}

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

/// Main configuration for Sage Agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Default LLM provider to use
    pub default_provider: String,
    /// Maximum number of execution steps
    pub max_steps: u32,
    /// Model parameters for each provider
    pub model_providers: HashMap<String, ModelParameters>,
    /// Lakeview configuration
    pub lakeview_config: Option<LakeviewConfig>,
    /// Whether to enable Lakeview
    pub enable_lakeview: bool,
    /// Working directory for the agent
    pub working_directory: Option<PathBuf>,
    /// Tool configuration
    pub tools: ToolConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Trajectory configuration
    #[serde(default)]
    pub trajectory: TrajectoryConfig,
    /// MCP (Model Context Protocol) configuration
    #[serde(default)]
    pub mcp: McpConfig,
}

impl Default for Config {
    fn default() -> Self {
        let mut model_providers = HashMap::new();

        // Default to Anthropic like Python version
        let anthropic_params = ModelParameters {
            model: "claude-sonnet-4-20250514".to_string(),
            api_key: None,
            base_url: Some("https://api.anthropic.com".to_string()),
            max_tokens: Some(4096),
            temperature: Some(0.5),
            top_p: Some(1.0),
            top_k: Some(0),
            parallel_tool_calls: Some(false),
            max_retries: Some(10),
            api_version: None,
            stop_sequences: None,
        };

        model_providers.insert("anthropic".to_string(), anthropic_params);

        // Add other providers with default configurations
        let openai_params = ModelParameters {
            model: "gpt-4".to_string(),
            api_key: None,
            base_url: Some("https://api.openai.com".to_string()),
            max_tokens: Some(4096),
            temperature: Some(0.5),
            top_p: Some(1.0),
            top_k: None,
            parallel_tool_calls: Some(true),
            max_retries: Some(10),
            api_version: None,
            stop_sequences: None,
        };
        model_providers.insert("openai".to_string(), openai_params);

        let google_params = ModelParameters {
            model: "gemini-1.5-pro".to_string(),
            api_key: None,
            base_url: Some("https://generativelanguage.googleapis.com".to_string()),
            max_tokens: Some(4096),
            temperature: Some(0.5),
            top_p: Some(1.0),
            top_k: Some(0),
            parallel_tool_calls: Some(false),
            max_retries: Some(10),
            api_version: None,
            stop_sequences: None,
        };
        model_providers.insert("google".to_string(), google_params);

        let azure_params = ModelParameters {
            model: "gpt-4".to_string(),
            api_key: None,
            base_url: Some("https://your-resource.openai.azure.com".to_string()),
            max_tokens: Some(4096),
            temperature: Some(0.5),
            top_p: Some(1.0),
            top_k: None,
            parallel_tool_calls: Some(true),
            max_retries: Some(10),
            api_version: Some("2024-02-15-preview".to_string()),
            stop_sequences: None,
        };
        model_providers.insert("azure".to_string(), azure_params);

        let openrouter_params = ModelParameters {
            model: "anthropic/claude-3.5-sonnet".to_string(),
            api_key: None,
            base_url: Some("https://openrouter.ai".to_string()),
            max_tokens: Some(4096),
            temperature: Some(0.5),
            top_p: Some(1.0),
            top_k: None,
            parallel_tool_calls: Some(true),
            max_retries: Some(10),
            api_version: None,
            stop_sequences: None,
        };
        model_providers.insert("openrouter".to_string(), openrouter_params);

        let doubao_params = ModelParameters {
            model: "doubao-pro-4k".to_string(),
            api_key: None,
            base_url: Some("https://ark.cn-beijing.volces.com".to_string()),
            max_tokens: Some(4096),
            temperature: Some(0.5),
            top_p: Some(1.0),
            top_k: None,
            parallel_tool_calls: Some(true),
            max_retries: Some(10),
            api_version: None,
            stop_sequences: None,
        };
        model_providers.insert("doubao".to_string(), doubao_params);

        let ollama_params = ModelParameters {
            model: "llama2".to_string(),
            api_key: None,
            base_url: Some("http://localhost:11434".to_string()),
            max_tokens: Some(4096),
            temperature: Some(0.5),
            top_p: Some(1.0),
            top_k: None,
            parallel_tool_calls: Some(false),
            max_retries: Some(3),
            api_version: None,
            stop_sequences: None,
        };
        model_providers.insert("ollama".to_string(), ollama_params);

        Self {
            default_provider: "anthropic".to_string(),
            max_steps: 20,
            model_providers,
            lakeview_config: None,
            enable_lakeview: true, // Python version defaults to true
            working_directory: None,
            tools: ToolConfig::default(),
            logging: LoggingConfig::default(),
            trajectory: TrajectoryConfig::default(),
            mcp: McpConfig::default(),
        }
    }
}

impl Config {
    /// Create a new config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the model parameters for the default provider
    pub fn default_model_parameters(&self) -> SageResult<&ModelParameters> {
        self.model_providers
            .get(&self.default_provider)
            .ok_or_else(|| {
                SageError::config(format!(
                    "No model parameters found for default provider: {}",
                    self.default_provider
                ))
            })
    }

    /// Get the default provider as string
    pub fn get_default_provider(&self) -> &str {
        &self.default_provider
    }

    /// Add or update model parameters for a provider
    pub fn set_model_parameters(&mut self, provider: String, params: ModelParameters) {
        self.model_providers.insert(provider, params);
    }

    /// Set the default provider
    pub fn set_default_provider(&mut self, provider: String) -> SageResult<()> {
        if !self.model_providers.contains_key(&provider) {
            return Err(SageError::config(format!(
                "Provider '{}' not found in model_providers",
                provider
            )));
        }
        self.default_provider = provider;
        Ok(())
    }

    /// Validate the entire configuration
    pub fn validate(&self) -> SageResult<()> {
        // Validate default provider exists
        if !self.model_providers.contains_key(&self.default_provider) {
            return Err(SageError::config(format!(
                "Default provider '{}' not found in model_providers",
                self.default_provider
            )));
        }

        // Validate max steps
        if self.max_steps == 0 {
            return Err(SageError::config("Max steps must be greater than 0"));
        }

        // Validate all model parameters
        for (provider, params) in &self.model_providers {
            params.validate().map_err(|e| {
                SageError::config(format!("Invalid parameters for provider '{}': {}", provider, e))
            })?;
        }

        // Validate working directory if set
        if let Some(working_dir) = &self.working_directory {
            if !working_dir.exists() {
                return Err(SageError::config(format!(
                    "Working directory does not exist: {}",
                    working_dir.display()
                )));
            }
        }

        Ok(())
    }

    /// Merge with another config (other takes precedence)
    pub fn merge(&mut self, other: Config) {
        if !other.default_provider.is_empty() {
            self.default_provider = other.default_provider;
        }

        if other.max_steps > 0 {
            self.max_steps = other.max_steps;
        }

        // Merge model providers
        for (provider, params) in other.model_providers {
            self.model_providers.insert(provider, params);
        }

        if other.lakeview_config.is_some() {
            self.lakeview_config = other.lakeview_config;
        }

        self.enable_lakeview = other.enable_lakeview;

        if other.working_directory.is_some() {
            self.working_directory = other.working_directory;
        }

        self.tools.merge(other.tools);
        self.logging.merge(other.logging);
        self.mcp.merge(other.mcp);
    }
}

/// Tool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    /// Enabled tools
    pub enabled_tools: Vec<String>,
    /// Tool-specific settings
    pub tool_settings: HashMap<String, serde_json::Value>,
    /// Maximum execution time for tools (in seconds)
    pub max_execution_time: u64,
    /// Whether to allow parallel tool execution
    pub allow_parallel_execution: bool,
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            enabled_tools: vec![
                "str_replace_based_edit_tool".to_string(),
                "sequentialthinking".to_string(),
                "json_edit_tool".to_string(),
                "task_done".to_string(),
                "bash".to_string(),
            ],
            tool_settings: HashMap::new(),
            max_execution_time: 300, // 5 minutes
            allow_parallel_execution: true,
        }
    }
}

impl ToolConfig {
    /// Check if a tool is enabled
    pub fn is_tool_enabled(&self, tool_name: &str) -> bool {
        self.enabled_tools.contains(&tool_name.to_string())
    }

    /// Get settings for a specific tool
    pub fn get_tool_settings(&self, tool_name: &str) -> Option<&serde_json::Value> {
        self.tool_settings.get(tool_name)
    }

    /// Merge with another tool config
    pub fn merge(&mut self, other: ToolConfig) {
        if !other.enabled_tools.is_empty() {
            self.enabled_tools = other.enabled_tools;
        }

        for (tool, settings) in other.tool_settings {
            self.tool_settings.insert(tool, settings);
        }

        if other.max_execution_time > 0 {
            self.max_execution_time = other.max_execution_time;
        }

        self.allow_parallel_execution = other.allow_parallel_execution;
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,
    /// Whether to log to file
    pub log_to_file: bool,
    /// Log file path
    pub log_file: Option<PathBuf>,
    /// Whether to log to console
    pub log_to_console: bool,
    /// Log format (json, pretty, compact)
    pub format: String,
}

/// Trajectory configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryConfig {
    /// Whether to enable trajectory recording by default
    pub enabled: bool,
    /// Directory to store trajectory files
    pub directory: PathBuf,
    /// Whether to auto-save trajectories during execution
    pub auto_save: bool,
    /// Number of steps between auto-saves
    pub save_interval_steps: usize,
}

/// MCP (Model Context Protocol) configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpConfig {
    /// Whether MCP integration is enabled
    #[serde(default)]
    pub enabled: bool,
    /// MCP servers to connect to
    #[serde(default)]
    pub servers: HashMap<String, McpServerConfig>,
    /// Default timeout for MCP requests in seconds
    #[serde(default = "default_mcp_timeout")]
    pub default_timeout_secs: u64,
    /// Whether to auto-connect to servers on startup
    #[serde(default = "default_true")]
    pub auto_connect: bool,
}

fn default_mcp_timeout() -> u64 {
    300 // 5 minutes
}

fn default_true() -> bool {
    true
}

/// Configuration for a single MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Transport type: "stdio", "http", or "websocket"
    pub transport: String,
    /// Command to execute (for stdio transport)
    pub command: Option<String>,
    /// Command arguments (for stdio transport)
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment variables (for stdio transport)
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Base URL (for http/websocket transport)
    pub url: Option<String>,
    /// HTTP headers (for http transport)
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Whether this server is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Request timeout in seconds (overrides default)
    pub timeout_secs: Option<u64>,
}

impl McpServerConfig {
    /// Create a stdio transport config
    pub fn stdio(command: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            transport: "stdio".to_string(),
            command: Some(command.into()),
            args,
            env: HashMap::new(),
            url: None,
            headers: HashMap::new(),
            enabled: true,
            timeout_secs: None,
        }
    }

    /// Create an HTTP transport config
    pub fn http(url: impl Into<String>) -> Self {
        Self {
            transport: "http".to_string(),
            command: None,
            args: Vec::new(),
            env: HashMap::new(),
            url: Some(url.into()),
            headers: HashMap::new(),
            enabled: true,
            timeout_secs: None,
        }
    }

    /// Create a WebSocket transport config
    pub fn websocket(url: impl Into<String>) -> Self {
        Self {
            transport: "websocket".to_string(),
            command: None,
            args: Vec::new(),
            env: HashMap::new(),
            url: Some(url.into()),
            headers: HashMap::new(),
            enabled: true,
            timeout_secs: None,
        }
    }

    /// Add environment variable
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Add HTTP header
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = Some(secs);
        self
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            log_to_file: false,
            log_file: None,
            log_to_console: true,
            format: "pretty".to_string(),
        }
    }
}

impl Default for TrajectoryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            directory: PathBuf::from("trajectories"),
            auto_save: true,
            save_interval_steps: 5,
        }
    }
}

impl LoggingConfig {
    /// Merge with another logging config
    pub fn merge(&mut self, other: LoggingConfig) {
        if !other.level.is_empty() {
            self.level = other.level;
        }

        self.log_to_file = other.log_to_file;

        if other.log_file.is_some() {
            self.log_file = other.log_file;
        }

        self.log_to_console = other.log_to_console;

        if !other.format.is_empty() {
            self.format = other.format;
        }
    }
}

impl McpConfig {
    /// Merge with another MCP config (other takes precedence)
    pub fn merge(&mut self, other: McpConfig) {
        if other.enabled {
            self.enabled = true;
        }

        // Merge servers
        for (name, config) in other.servers {
            self.servers.insert(name, config);
        }

        if other.default_timeout_secs > 0 {
            self.default_timeout_secs = other.default_timeout_secs;
        }

        self.auto_connect = other.auto_connect;
    }

    /// Get enabled servers
    pub fn enabled_servers(&self) -> impl Iterator<Item = (&String, &McpServerConfig)> {
        self.servers.iter().filter(|(_, config)| config.enabled)
    }

    /// Get timeout for a specific server
    pub fn get_timeout(&self, server_name: &str) -> u64 {
        self.servers
            .get(server_name)
            .and_then(|s| s.timeout_secs)
            .unwrap_or(self.default_timeout_secs)
    }
}
