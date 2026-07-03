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

    pub fn is_enabled(&self, tool_name: &str) -> bool {
        let normalized = normalize_tool_name(tool_name);
        let enabled = self.enabled.iter().map(|name| normalize_tool_name(name));
        let disabled = self.disabled.iter().map(|name| normalize_tool_name(name));

        (self.enabled.is_empty() || enabled.into_iter().any(|name| name == normalized))
            && !disabled.into_iter().any(|name| name == normalized)
    }
}

fn normalize_tool_name(tool_name: &str) -> String {
    tool_name.trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_enabled_list_allows_tools_not_disabled() {
        let settings = ToolSettings {
            disabled: vec!["Bash".to_string()],
            ..Default::default()
        };

        assert!(settings.is_enabled("Read"));
        assert!(!settings.is_enabled("bash"));
    }

    #[test]
    fn enabled_list_is_authoritative_before_disabled_filter() {
        let settings = ToolSettings {
            enabled: vec!["Read".to_string()],
            disabled: vec!["Write".to_string()],
            ..Default::default()
        };

        assert!(settings.is_enabled("read"));
        assert!(!settings.is_enabled("Bash"));
        assert!(!settings.is_enabled("Write"));
    }
}
