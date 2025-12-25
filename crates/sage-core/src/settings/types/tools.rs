//! Tool settings and configuration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Tool settings
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolSettings {
    /// Enabled tools (if empty, all are enabled)
    #[serde(default)]
    pub enabled: Vec<String>,

    /// Disabled tools
    #[serde(default)]
    pub disabled: Vec<String>,

    /// Tool-specific configuration
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,

    /// Tool-specific timeouts (in milliseconds)
    #[serde(default)]
    pub timeouts: HashMap<String, u64>,
}

impl ToolSettings {
    /// Merge another tool settings
    pub fn merge(&mut self, other: ToolSettings) {
        self.enabled.extend(other.enabled);
        self.disabled.extend(other.disabled);
        self.config.extend(other.config);
        self.timeouts.extend(other.timeouts);
    }

    /// Check if a tool is enabled
    pub fn is_enabled(&self, tool_name: &str) -> bool {
        // If explicitly disabled, return false
        if self.disabled.contains(&tool_name.to_string()) {
            return false;
        }

        // If enabled list is empty, all tools are enabled
        // Otherwise, only tools in the enabled list are enabled
        self.enabled.is_empty() || self.enabled.contains(&tool_name.to_string())
    }

    /// Get timeout for a tool (in milliseconds)
    pub fn get_timeout(&self, tool_name: &str) -> Option<u64> {
        self.timeouts.get(tool_name).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_is_enabled() {
        let mut tools = ToolSettings::default();

        // All enabled by default
        assert!(tools.is_enabled("bash"));
        assert!(tools.is_enabled("read"));

        // Disable one
        tools.disabled.push("bash".to_string());
        assert!(!tools.is_enabled("bash"));
        assert!(tools.is_enabled("read"));

        // Enable list takes precedence
        let mut tools2 = ToolSettings::default();
        tools2.enabled.push("read".to_string());
        assert!(tools2.is_enabled("read"));
        assert!(!tools2.is_enabled("bash"));
    }
}
