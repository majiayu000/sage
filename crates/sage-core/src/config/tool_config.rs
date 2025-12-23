//! Tool configuration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_config_default() {
        let config = ToolConfig::default();
        assert!(config.enabled_tools.contains(&"bash".to_string()));
        assert!(config.enabled_tools.contains(&"task_done".to_string()));
        assert_eq!(config.max_execution_time, 300);
        assert!(config.allow_parallel_execution);
    }

    #[test]
    fn test_tool_config_is_tool_enabled() {
        let config = ToolConfig::default();
        assert!(config.is_tool_enabled("bash"));
        assert!(!config.is_tool_enabled("nonexistent"));
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
            enabled_tools: vec!["custom_tool".to_string()],
            max_execution_time: 600,
            allow_parallel_execution: false,
            tool_settings: HashMap::new(),
        };
        config2
            .tool_settings
            .insert("custom".to_string(), serde_json::json!({"key": "value"}));

        config1.merge(config2);
        assert!(config1.enabled_tools.contains(&"custom_tool".to_string()));
        assert_eq!(config1.max_execution_time, 600);
        assert!(!config1.allow_parallel_execution);
        assert!(config1.tool_settings.contains_key("custom"));
    }

    #[test]
    fn test_tool_config_merge_empty_tools() {
        let mut config1 = ToolConfig::default();
        let config2 = ToolConfig {
            enabled_tools: vec![],
            max_execution_time: 0,
            allow_parallel_execution: false,
            tool_settings: HashMap::new(),
        };

        let original_tools = config1.enabled_tools.clone();
        config1.merge(config2);
        // Empty tools should not override
        assert_eq!(config1.enabled_tools, original_tools);
        // But max_execution_time of 0 should be ignored
        assert_eq!(config1.max_execution_time, 300);
    }
}
