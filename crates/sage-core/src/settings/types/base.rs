//! Main settings structure

use super::{HooksSettings, ModelSettings, PermissionSettings, ToolSettings, UiSettings, WorkspaceSettings};
use super::permissions::PermissionBehavior;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert!(settings.permissions.allow.is_empty());
        assert!(settings.permissions.deny.is_empty());
        assert_eq!(
            settings.permissions.default_behavior,
            PermissionBehavior::Ask
        );
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
