//! Permission settings and patterns

use serde::{Deserialize, Serialize};

/// Permission settings
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PermissionSettings {
    /// Patterns to allow (e.g., "Bash(npm *)", "Read(src/**)")
    #[serde(default)]
    pub allow: Vec<String>,

    /// Patterns to deny
    #[serde(default)]
    pub deny: Vec<String>,

    /// Default behavior when no rule matches
    #[serde(default)]
    pub default_behavior: SettingsPermissionBehavior,
}

impl PermissionSettings {
    /// Merge another permission settings
    pub fn merge(&mut self, other: PermissionSettings) {
        // Extend allow/deny lists
        self.allow.extend(other.allow);
        self.deny.extend(other.deny);

        // Override default behavior if explicitly set
        if other.default_behavior != SettingsPermissionBehavior::default() {
            self.default_behavior = other.default_behavior;
        }
    }

    /// Parse a permission pattern
    /// Format: "ToolName(pattern)" or just "ToolName"
    pub fn parse_pattern(pattern: &str) -> Option<ParsedPattern> {
        let pattern = pattern.trim();

        if let Some(open) = pattern.find('(') {
            if let Some(close) = pattern.rfind(')') {
                let tool_name = pattern[..open].trim().to_string();
                let arg_pattern = pattern[open + 1..close].trim().to_string();

                return Some(ParsedPattern {
                    tool_name,
                    arg_pattern: Some(arg_pattern),
                });
            }
        }

        // Just tool name, no argument pattern
        Some(ParsedPattern {
            tool_name: pattern.to_string(),
            arg_pattern: None,
        })
    }
}

/// Parsed permission pattern
#[derive(Debug, Clone)]
pub struct ParsedPattern {
    /// Tool name
    pub tool_name: String,
    /// Optional argument pattern (path or command)
    pub arg_pattern: Option<String>,
}

/// Permission behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SettingsPermissionBehavior {
    /// Always allow
    Allow,
    /// Always deny
    Deny,
    /// Ask the user
    Ask,
}

impl Default for SettingsPermissionBehavior {
    fn default() -> Self {
        Self::Ask
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pattern_with_args() {
        let pattern = PermissionSettings::parse_pattern("Bash(npm *)").unwrap();
        assert_eq!(pattern.tool_name, "Bash");
        assert_eq!(pattern.arg_pattern, Some("npm *".to_string()));
    }

    #[test]
    fn test_parse_pattern_without_args() {
        let pattern = PermissionSettings::parse_pattern("Read").unwrap();
        assert_eq!(pattern.tool_name, "Read");
        assert!(pattern.arg_pattern.is_none());
    }

    #[test]
    fn test_parse_pattern_with_path() {
        let pattern = PermissionSettings::parse_pattern("Read(src/**/*)").unwrap();
        assert_eq!(pattern.tool_name, "Read");
        assert_eq!(pattern.arg_pattern, Some("src/**/*".to_string()));
    }

    #[test]
    fn test_permission_behavior_default() {
        assert_eq!(
            SettingsPermissionBehavior::default(),
            SettingsPermissionBehavior::Ask
        );
    }
}
