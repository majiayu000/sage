//! Settings type definitions
//!
//! This module defines the settings structure for Sage Agent,
//! supporting multi-level configuration (user, project, local).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Main settings structure
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    /// Permission settings
    #[serde(default)]
    pub permissions: PermissionSettings,

    /// Tool settings
    #[serde(default)]
    pub tools: ToolSettings,

    /// Environment variables to set
    #[serde(default)]
    pub environment: HashMap<String, String>,

    /// Hook settings
    #[serde(default)]
    pub hooks: HooksSettings,

    /// UI settings
    #[serde(default)]
    pub ui: UiSettings,

    /// Workspace settings
    #[serde(default)]
    pub workspace: WorkspaceSettings,

    /// Model settings
    #[serde(default)]
    pub model: ModelSettings,
}

impl Settings {
    /// Create new default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Merge another settings instance into this one
    /// The other settings take precedence (override this)
    pub fn merge(&mut self, other: Settings) {
        self.permissions.merge(other.permissions);
        self.tools.merge(other.tools);
        self.environment.extend(other.environment);
        self.hooks.merge(other.hooks);
        self.ui.merge(other.ui);
        self.workspace.merge(other.workspace);
        self.model.merge(other.model);
    }

    /// Apply environment variable overrides
    pub fn apply_env_overrides(&mut self) {
        // SAGE_MODEL
        if let Ok(model) = std::env::var("SAGE_MODEL") {
            self.model.default_model = Some(model);
        }

        // SAGE_MAX_TOKENS
        if let Ok(tokens) = std::env::var("SAGE_MAX_TOKENS") {
            if let Ok(n) = tokens.parse() {
                self.model.max_tokens = Some(n);
            }
        }

        // SAGE_ALLOW_ALL
        if std::env::var("SAGE_ALLOW_ALL").is_ok() {
            self.permissions.default_behavior = PermissionBehavior::Allow;
        }
    }
}

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
    pub default_behavior: PermissionBehavior,
}

impl PermissionSettings {
    /// Merge another permission settings
    pub fn merge(&mut self, other: PermissionSettings) {
        // Extend allow/deny lists
        self.allow.extend(other.allow);
        self.deny.extend(other.deny);

        // Override default behavior if explicitly set
        if other.default_behavior != PermissionBehavior::default() {
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
pub enum PermissionBehavior {
    /// Always allow
    Allow,
    /// Always deny
    Deny,
    /// Ask the user
    Ask,
}

impl Default for PermissionBehavior {
    fn default() -> Self {
        Self::Ask
    }
}

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

/// Hook settings
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HooksSettings {
    /// Pre-tool-use hooks
    #[serde(default)]
    pub pre_tool_use: Vec<HookDefinition>,

    /// Post-tool-use hooks
    #[serde(default)]
    pub post_tool_use: Vec<HookDefinition>,

    /// User prompt submit hooks
    #[serde(default)]
    pub user_prompt_submit: Vec<HookDefinition>,

    /// Session start hooks
    #[serde(default)]
    pub session_start: Vec<HookDefinition>,

    /// Session end hooks
    #[serde(default)]
    pub session_end: Vec<HookDefinition>,
}

impl HooksSettings {
    /// Merge another hooks settings
    pub fn merge(&mut self, other: HooksSettings) {
        self.pre_tool_use.extend(other.pre_tool_use);
        self.post_tool_use.extend(other.post_tool_use);
        self.user_prompt_submit.extend(other.user_prompt_submit);
        self.session_start.extend(other.session_start);
        self.session_end.extend(other.session_end);
    }
}

/// Hook definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookDefinition {
    /// Hook type
    #[serde(rename = "type")]
    pub hook_type: HookDefinitionType,

    /// Command to run (for command hooks)
    #[serde(default)]
    pub command: Option<String>,

    /// Prompt to use (for prompt hooks)
    #[serde(default)]
    pub prompt: Option<String>,

    /// Pattern to match (optional, for filtering)
    #[serde(default)]
    pub pattern: Option<String>,

    /// Timeout in milliseconds
    #[serde(default)]
    pub timeout_ms: Option<u64>,

    /// Status message to display
    #[serde(default)]
    pub status_message: Option<String>,
}

/// Hook definition type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HookDefinitionType {
    /// Shell command
    Command,
    /// LLM prompt
    Prompt,
}

/// UI settings
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UiSettings {
    /// Show progress indicators
    #[serde(default)]
    pub show_progress: Option<bool>,

    /// Theme (light/dark/auto)
    #[serde(default)]
    pub theme: Option<String>,

    /// Enable colors
    #[serde(default)]
    pub colors: Option<bool>,

    /// Verbose output
    #[serde(default)]
    pub verbose: Option<bool>,

    /// Maximum output width
    #[serde(default)]
    pub max_width: Option<usize>,
}

impl UiSettings {
    /// Merge another UI settings
    pub fn merge(&mut self, other: UiSettings) {
        if other.show_progress.is_some() {
            self.show_progress = other.show_progress;
        }
        if other.theme.is_some() {
            self.theme = other.theme;
        }
        if other.colors.is_some() {
            self.colors = other.colors;
        }
        if other.verbose.is_some() {
            self.verbose = other.verbose;
        }
        if other.max_width.is_some() {
            self.max_width = other.max_width;
        }
    }
}

/// Workspace settings
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkspaceSettings {
    /// Files/directories to ignore
    #[serde(default)]
    pub ignore: Vec<String>,

    /// Include patterns
    #[serde(default)]
    pub include: Vec<String>,

    /// Working directory override
    #[serde(default)]
    pub working_directory: Option<String>,

    /// Project type hint
    #[serde(default)]
    pub project_type: Option<String>,
}

impl WorkspaceSettings {
    /// Merge another workspace settings
    pub fn merge(&mut self, other: WorkspaceSettings) {
        self.ignore.extend(other.ignore);
        self.include.extend(other.include);
        if other.working_directory.is_some() {
            self.working_directory = other.working_directory;
        }
        if other.project_type.is_some() {
            self.project_type = other.project_type;
        }
    }
}

/// Model settings
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelSettings {
    /// Default model to use
    #[serde(default)]
    pub default_model: Option<String>,

    /// Maximum tokens
    #[serde(default)]
    pub max_tokens: Option<usize>,

    /// Temperature
    #[serde(default)]
    pub temperature: Option<f32>,

    /// Provider override
    #[serde(default)]
    pub provider: Option<String>,

    /// API base URL override
    #[serde(default)]
    pub api_base: Option<String>,
}

impl ModelSettings {
    /// Merge another model settings
    pub fn merge(&mut self, other: ModelSettings) {
        if other.default_model.is_some() {
            self.default_model = other.default_model;
        }
        if other.max_tokens.is_some() {
            self.max_tokens = other.max_tokens;
        }
        if other.temperature.is_some() {
            self.temperature = other.temperature;
        }
        if other.provider.is_some() {
            self.provider = other.provider;
        }
        if other.api_base.is_some() {
            self.api_base = other.api_base;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert!(settings.permissions.allow.is_empty());
        assert!(settings.permissions.deny.is_empty());
        assert_eq!(settings.permissions.default_behavior, PermissionBehavior::Ask);
    }

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
    fn test_merge_settings() {
        let mut base = Settings::default();
        base.permissions.allow.push("Read(src/*)".to_string());

        let override_settings = Settings {
            permissions: PermissionSettings {
                allow: vec!["Write(src/*)".to_string()],
                deny: vec!["Bash(rm -rf *)".to_string()],
                default_behavior: PermissionBehavior::Allow,
            },
            ..Default::default()
        };

        base.merge(override_settings);

        assert_eq!(base.permissions.allow.len(), 2);
        assert_eq!(base.permissions.deny.len(), 1);
        assert_eq!(base.permissions.default_behavior, PermissionBehavior::Allow);
    }

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

    #[test]
    fn test_permission_behavior_default() {
        assert_eq!(PermissionBehavior::default(), PermissionBehavior::Ask);
    }

    #[test]
    fn test_settings_serialization() {
        let settings = Settings {
            permissions: PermissionSettings {
                allow: vec!["Read(src/*)".to_string()],
                deny: vec![],
                default_behavior: PermissionBehavior::Ask,
            },
            ..Default::default()
        };

        let json = serde_json::to_string_pretty(&settings).unwrap();
        assert!(json.contains("Read(src/*)"));

        let deserialized: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.permissions.allow.len(), 1);
    }
}
