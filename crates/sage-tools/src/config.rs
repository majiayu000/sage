//! Configuration system for Sage Tools

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// Global configuration for all tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsConfig {
    /// Default working directory for file operations
    pub default_working_directory: PathBuf,
    /// Maximum execution time for tools (in seconds)
    pub max_execution_time_seconds: u64,
    /// Maximum output size (in bytes)
    pub max_output_size_bytes: usize,
    /// Enable debug logging
    pub debug_logging: bool,
    /// Tool-specific configurations
    pub tool_configs: HashMap<String, ToolInstanceConfig>,
}

impl Default for ToolsConfig {
    fn default() -> Self {
        Self {
            default_working_directory: std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from(".")),
            max_execution_time_seconds: 300,    // 5 minutes
            max_output_size_bytes: 1024 * 1024, // 1MB
            debug_logging: false,
            tool_configs: HashMap::new(),
        }
    }
}

/// Configuration for individual tool instances
///
/// This is distinct from `sage_core::config::ToolConfig`, which holds
/// global tool orchestration settings (parallel execution, shared settings).
/// `ToolInstanceConfig` holds per-tool overrides like timeouts, output limits,
/// and whether the tool is enabled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInstanceConfig {
    /// Tool-specific maximum execution time
    pub max_execution_time_seconds: Option<u64>,
    /// Tool-specific maximum output size
    pub max_output_size_bytes: Option<usize>,
    /// Whether this tool is enabled
    pub enabled: bool,
    /// Tool-specific settings
    pub settings: HashMap<String, serde_json::Value>,
}

impl Default for ToolInstanceConfig {
    fn default() -> Self {
        Self {
            max_execution_time_seconds: None,
            max_output_size_bytes: None,
            enabled: true,
            settings: HashMap::new(),
        }
    }
}

/// Configuration for BashTool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BashToolConfig {
    /// List of allowed commands (empty means all allowed)
    pub allowed_commands: Vec<String>,
    /// Working directory for command execution
    pub working_directory: Option<PathBuf>,
    /// Environment variables to set
    pub environment_variables: HashMap<String, String>,
    /// Whether to capture stderr
    pub capture_stderr: bool,
}

impl Default for BashToolConfig {
    fn default() -> Self {
        Self {
            allowed_commands: Vec::new(),
            working_directory: None,
            environment_variables: HashMap::new(),
            capture_stderr: true,
        }
    }
}

/// Configuration for EditTool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditToolConfig {
    /// Working directory for file operations
    pub working_directory: Option<PathBuf>,
    /// Maximum file size to edit (in bytes)
    pub max_file_size_bytes: usize,
    /// Whether to create backup files
    pub create_backups: bool,
    /// File extensions that are allowed to be edited
    pub allowed_extensions: Vec<String>,
}

impl Default for EditToolConfig {
    fn default() -> Self {
        Self {
            working_directory: None,
            max_file_size_bytes: 10 * 1024 * 1024, // 10MB
            create_backups: false,
            allowed_extensions: Vec::new(), // Empty means all allowed
        }
    }
}

/// Configuration for TaskManagementTools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskManagementConfig {
    /// Maximum number of tasks allowed
    pub max_tasks: usize,
    /// Whether to persist tasks to disk
    pub persist_tasks: bool,
    /// File path for task persistence
    pub persistence_file: Option<PathBuf>,
    /// Auto-save interval (in seconds)
    pub auto_save_interval_seconds: u64,
}

impl Default for TaskManagementConfig {
    fn default() -> Self {
        Self {
            max_tasks: 1000,
            persist_tasks: false,
            persistence_file: None,
            auto_save_interval_seconds: 300, // 5 minutes
        }
    }
}

impl ToolsConfig {
    /// Load configuration from file
    pub fn load_from_file(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to file
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get configuration for a specific tool instance
    pub fn get_tool_instance_config(&self, tool_name: &str) -> ToolInstanceConfig {
        self.tool_configs
            .get(tool_name)
            .cloned()
            .unwrap_or_default()
    }

    /// Set configuration for a specific tool instance
    pub fn set_tool_instance_config(&mut self, tool_name: String, config: ToolInstanceConfig) {
        self.tool_configs.insert(tool_name, config);
    }

    /// Get maximum execution time for a tool
    pub fn get_max_execution_time(&self, tool_name: &str) -> Duration {
        let tool_config = self.get_tool_instance_config(tool_name);
        let seconds = tool_config
            .max_execution_time_seconds
            .unwrap_or(self.max_execution_time_seconds);
        Duration::from_secs(seconds)
    }

    /// Get maximum output size for a tool
    pub fn get_max_output_size(&self, tool_name: &str) -> usize {
        let tool_config = self.get_tool_instance_config(tool_name);
        tool_config
            .max_output_size_bytes
            .unwrap_or(self.max_output_size_bytes)
    }

    /// Check if a tool is enabled
    pub fn is_tool_enabled(&self, tool_name: &str) -> bool {
        self.get_tool_instance_config(tool_name).enabled
    }
}

// Global configuration instance
pub static GLOBAL_CONFIG: Lazy<std::sync::RwLock<ToolsConfig>> =
    Lazy::new(|| std::sync::RwLock::new(ToolsConfig::default()));

/// Helper function to get global configuration
pub fn get_global_config() -> ToolsConfig {
    GLOBAL_CONFIG
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

/// Helper function to update global configuration
pub fn update_global_config<F>(updater: F)
where
    F: FnOnce(&mut ToolsConfig),
{
    let mut config = GLOBAL_CONFIG.write().unwrap_or_else(|e| e.into_inner());
    updater(&mut config);
}
