//! Tool configuration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Tool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
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
            tool_settings: HashMap::new(),
            max_execution_time: 300, // 5 minutes
            allow_parallel_execution: true,
        }
    }
}

impl ToolConfig {
    /// Get settings for a specific tool
    pub fn get_tool_settings(&self, tool_name: &str) -> Option<&serde_json::Value> {
        self.tool_settings.get(tool_name)
    }

    /// Merge with another tool config
    pub fn merge(&mut self, other: ToolConfig) {
        for (tool, settings) in other.tool_settings {
            self.tool_settings.insert(tool, settings);
        }

        if other.max_execution_time > 0 {
            self.max_execution_time = other.max_execution_time;
        }

        self.allow_parallel_execution = other.allow_parallel_execution;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_config_default() {
        let config = ToolConfig::default();
        assert_eq!(config.max_execution_time, 300);
        assert!(config.allow_parallel_execution);
        assert!(config.tool_settings.is_empty());
    }

    #[test]
    fn test_tool_config_get_tool_settings() {
        let mut config = ToolConfig::default();
        config
            .tool_settings
            .insert("bash".to_string(), serde_json::json!({"timeout": 60}));
        assert!(config.get_tool_settings("bash").is_some());
        assert!(config.get_tool_settings("nonexistent").is_none());
    }

    #[test]
    fn test_tool_config_merge() {
        let mut config1 = ToolConfig::default();
        let mut config2 = ToolConfig {
            max_execution_time: 600,
            allow_parallel_execution: false,
            tool_settings: HashMap::new(),
        };
        config2
            .tool_settings
            .insert("custom".to_string(), serde_json::json!({"key": "value"}));

        config1.merge(config2);
        assert_eq!(config1.max_execution_time, 600);
        assert!(!config1.allow_parallel_execution);
        assert!(config1.tool_settings.contains_key("custom"));
    }

    #[test]
    fn test_tool_config_merge_zero_timeout() {
        let mut config1 = ToolConfig::default();
        let config2 = ToolConfig {
            max_execution_time: 0,
            allow_parallel_execution: false,
            tool_settings: HashMap::new(),
        };

        config1.merge(config2);
        // max_execution_time of 0 should be ignored
        assert_eq!(config1.max_execution_time, 300);
    }
}
