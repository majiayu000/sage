//! Settings loader
//!
//! This module handles loading settings from multiple sources
//! and merging them according to priority.

use crate::error::{SageError, SageResult};
use std::path::Path;

use super::locations::SettingsLocations;
use super::types::Settings;
use super::validation::SettingsValidator;

/// Settings loader that merges settings from multiple sources
#[derive(Debug)]
pub struct SettingsLoader {
    /// Discovered settings locations
    locations: SettingsLocations,
    /// Settings validator
    validator: SettingsValidator,
    /// Whether to validate settings
    validate: bool,
}

impl SettingsLoader {
    /// Create a new settings loader
    pub fn new() -> Self {
        Self {
            locations: SettingsLocations::discover(),
            validator: SettingsValidator::new(),
            validate: true,
        }
    }

    /// Create a settings loader from a starting directory
    pub fn from_directory(dir: impl AsRef<Path>) -> Self {
        Self {
            locations: SettingsLocations::discover_from(dir),
            validator: SettingsValidator::new(),
            validate: true,
        }
    }

    /// Disable validation
    pub fn without_validation(mut self) -> Self {
        self.validate = false;
        self
    }

    /// Load and merge settings from all sources
    pub fn load(&self) -> SageResult<Settings> {
        let mut settings = Settings::default();

        // Load user settings
        if self.locations.user.exists() {
            match self.load_from_file(&self.locations.user) {
                Ok(user_settings) => {
                    tracing::debug!("Loaded user settings from {:?}", self.locations.user);
                    settings.merge(user_settings);
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to load user settings from {:?}: {}",
                        self.locations.user,
                        e
                    );
                }
            }
        }

        // Load project settings
        if let Some(ref project_path) = self.locations.project {
            if project_path.exists() {
                match self.load_from_file(project_path) {
                    Ok(project_settings) => {
                        tracing::debug!("Loaded project settings from {:?}", project_path);
                        settings.merge(project_settings);
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to load project settings from {:?}: {}",
                            project_path,
                            e
                        );
                    }
                }
            }
        }

        // Load local settings
        if let Some(ref local_path) = self.locations.local {
            if local_path.exists() {
                match self.load_from_file(local_path) {
                    Ok(local_settings) => {
                        tracing::debug!("Loaded local settings from {:?}", local_path);
                        settings.merge(local_settings);
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to load local settings from {:?}: {}",
                            local_path,
                            e
                        );
                    }
                }
            }
        }

        // Apply environment variable overrides
        settings.apply_env_overrides();

        // Validate if enabled
        if self.validate {
            self.validator.validate(&settings)?;
        }

        Ok(settings)
    }

    /// Load settings from a specific file
    pub fn load_from_file(&self, path: impl AsRef<Path>) -> SageResult<Settings> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|e| {
            SageError::config(format!("Failed to read settings file {:?}: {}", path, e))
        })?;

        self.parse_settings(&content, path)
    }

    /// Parse settings from JSON string
    pub fn parse_settings(&self, content: &str, path: &Path) -> SageResult<Settings> {
        // Support JSON with comments by stripping them
        let stripped = Self::strip_json_comments(content);

        serde_json::from_str(&stripped).map_err(|e| {
            SageError::config(format!("Failed to parse settings file {:?}: {}", path, e))
        })
    }

    /// Strip JSON comments (// and /* */)
    fn strip_json_comments(content: &str) -> String {
        let mut result = String::new();
        let mut chars = content.chars().peekable();
        let mut in_string = false;
        let mut escape_next = false;

        while let Some(c) = chars.next() {
            if escape_next {
                result.push(c);
                escape_next = false;
                continue;
            }

            if c == '\\' && in_string {
                result.push(c);
                escape_next = true;
                continue;
            }

            if c == '"' {
                in_string = !in_string;
                result.push(c);
                continue;
            }

            if !in_string && c == '/' {
                if let Some(&next) = chars.peek() {
                    if next == '/' {
                        // Line comment - skip until newline
                        chars.next();
                        while let Some(&ch) = chars.peek() {
                            if ch == '\n' {
                                break;
                            }
                            chars.next();
                        }
                        continue;
                    } else if next == '*' {
                        // Block comment - skip until */
                        chars.next();
                        while let Some(ch) = chars.next() {
                            if ch == '*' {
                                if let Some(&'/') = chars.peek() {
                                    chars.next();
                                    break;
                                }
                            }
                        }
                        continue;
                    }
                }
            }

            result.push(c);
        }

        result
    }

    /// Save settings to a specific file
    pub fn save_to_file(&self, settings: &Settings, path: impl AsRef<Path>) -> SageResult<()> {
        let path = path.as_ref();

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                SageError::config(format!("Failed to create settings directory: {}", e))
            })?;
        }

        let content = serde_json::to_string_pretty(settings)
            .map_err(|e| SageError::config(format!("Failed to serialize settings: {}", e)))?;

        std::fs::write(path, content).map_err(|e| {
            SageError::config(format!("Failed to write settings file {:?}: {}", path, e))
        })?;

        Ok(())
    }
}

impl Default for SettingsLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::types::SettingsPermissionBehavior;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_load_empty_settings() {
        let temp_dir = TempDir::new().unwrap();
        let loader = SettingsLoader::from_directory(temp_dir.path());
        let settings = loader.load().unwrap();

        assert_eq!(
            settings.permissions.default_behavior,
            SettingsPermissionBehavior::Ask
        );
    }

    #[test]
    fn test_load_project_settings() {
        let temp_dir = TempDir::new().unwrap();
        let sage_dir = temp_dir.path().join(".sage");
        fs::create_dir(&sage_dir).unwrap();

        let settings_content = r#"{
            "permissions": {
                "allow": ["Read(src/*)"],
                "deny": ["Bash(rm -rf *)"],
                "default_behavior": "allow"
            }
        }"#;

        fs::write(sage_dir.join("settings.json"), settings_content).unwrap();

        let loader = SettingsLoader::from_directory(temp_dir.path());
        let settings = loader.load().unwrap();

        assert_eq!(settings.permissions.allow.len(), 1);
        assert_eq!(settings.permissions.deny.len(), 1);
        assert_eq!(
            settings.permissions.default_behavior,
            SettingsPermissionBehavior::Allow
        );
    }

    #[test]
    fn test_settings_merge_precedence() {
        let temp_dir = TempDir::new().unwrap();
        let sage_dir = temp_dir.path().join(".sage");
        fs::create_dir(&sage_dir).unwrap();

        // Project settings
        let project_content = r#"{
            "permissions": {
                "allow": ["Read(src/*)"],
                "default_behavior": "ask"
            }
        }"#;
        fs::write(sage_dir.join("settings.json"), project_content).unwrap();

        // Local settings (should override)
        let local_content = r#"{
            "permissions": {
                "allow": ["Write(src/*)"],
                "default_behavior": "allow"
            }
        }"#;
        fs::write(sage_dir.join("settings.local.json"), local_content).unwrap();

        let loader = SettingsLoader::from_directory(temp_dir.path());
        let settings = loader.load().unwrap();

        // Should have both allow patterns
        assert_eq!(settings.permissions.allow.len(), 2);
        // Local behavior should win
        assert_eq!(
            settings.permissions.default_behavior,
            SettingsPermissionBehavior::Allow
        );
    }

    #[test]
    fn test_strip_json_comments() {
        let content = r#"{
            // This is a comment
            "key": "value", // inline comment
            /* block
               comment */
            "key2": "value2"
        }"#;

        let stripped = SettingsLoader::strip_json_comments(content);
        let parsed: serde_json::Value = serde_json::from_str(&stripped).unwrap();

        assert_eq!(parsed["key"], "value");
        assert_eq!(parsed["key2"], "value2");
    }

    #[test]
    fn test_strip_comments_preserves_strings() {
        let content = r#"{"key": "// not a comment", "key2": "/* also not */"}"#;
        let stripped = SettingsLoader::strip_json_comments(content);
        let parsed: serde_json::Value = serde_json::from_str(&stripped).unwrap();

        assert_eq!(parsed["key"], "// not a comment");
        assert_eq!(parsed["key2"], "/* also not */");
    }

    #[test]
    fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let settings_path = temp_dir.path().join("settings.json");

        let loader = SettingsLoader::from_directory(temp_dir.path());

        let mut settings = Settings::default();
        settings.permissions.allow.push("Read(*)".to_string());

        loader.save_to_file(&settings, &settings_path).unwrap();
        let loaded = loader.load_from_file(&settings_path).unwrap();

        assert_eq!(loaded.permissions.allow, settings.permissions.allow);
    }
}
