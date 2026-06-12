//! Permission settings and patterns

use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};

/// Permission settings
#[derive(Debug, Clone, Default)]
pub struct PermissionSettings {
    /// Patterns to allow (e.g., "Bash(npm *)", "Read(src/**)")
    pub allow: Vec<String>,

    /// Patterns to deny
    pub deny: Vec<String>,

    /// Default behavior when no rule matches
    pub default_behavior: SettingsPermissionBehavior,

    /// Whether default_behavior was explicitly declared by a settings file.
    pub default_behavior_set: bool,
}

impl Serialize for PermissionSettings {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut len = 0;
        if !self.allow.is_empty() {
            len += 1;
        }
        if !self.deny.is_empty() {
            len += 1;
        }
        if self.default_behavior_set
            || self.default_behavior != SettingsPermissionBehavior::default()
        {
            len += 1;
        }

        let mut state = serializer.serialize_struct("PermissionSettings", len)?;
        if !self.allow.is_empty() {
            state.serialize_field("allow", &self.allow)?;
        }
        if !self.deny.is_empty() {
            state.serialize_field("deny", &self.deny)?;
        }
        if self.default_behavior_set
            || self.default_behavior != SettingsPermissionBehavior::default()
        {
            state.serialize_field("default_behavior", &self.default_behavior)?;
        }
        state.end()
    }
}

impl PermissionSettings {
    /// Merge another permission settings
    pub fn merge(&mut self, other: PermissionSettings) {
        // Extend allow/deny lists
        self.allow.extend(other.allow);
        self.deny.extend(other.deny);

        // Override default behavior if explicitly set. Programmatic settings
        // with allow/deny still override without the deserialization marker.
        if other.default_behavior_set
            || other.default_behavior != SettingsPermissionBehavior::default()
        {
            self.default_behavior = other.default_behavior;
            self.default_behavior_set = true;
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

impl<'de> Deserialize<'de> for PermissionSettings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct PermissionSettingsWire {
            #[serde(default)]
            allow: Vec<String>,
            #[serde(default)]
            deny: Vec<String>,
            default_behavior: Option<SettingsPermissionBehavior>,
        }

        let wire = PermissionSettingsWire::deserialize(deserializer)?;
        Ok(Self {
            allow: wire.allow,
            deny: wire.deny,
            default_behavior: wire.default_behavior.unwrap_or_default(),
            default_behavior_set: wire.default_behavior.is_some(),
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

    #[test]
    fn test_deserialize_tracks_explicit_default_behavior() {
        let implicit: PermissionSettings = serde_json::from_str("{}").unwrap();
        let explicit: PermissionSettings =
            serde_json::from_str(r#"{"default_behavior": "ask"}"#).unwrap();

        assert_eq!(implicit.default_behavior, SettingsPermissionBehavior::Ask);
        assert!(!implicit.default_behavior_set);
        assert_eq!(explicit.default_behavior, SettingsPermissionBehavior::Ask);
        assert!(explicit.default_behavior_set);
    }

    #[test]
    fn test_implicit_default_behavior_is_not_serialized() {
        let settings = PermissionSettings::default();

        let json = serde_json::to_string(&settings).unwrap();

        assert!(!json.contains("default_behavior"));
    }
}
