//! Settings validation
//!
//! This module provides validation for settings to ensure
//! they are well-formed and consistent.

use crate::error::{SageError, SageResult};

use super::types::{PermissionSettings, Settings, ToolSettings};

/// Settings validator
#[derive(Debug, Default)]
pub struct SettingsValidator {
    /// Known tool names for validation
    known_tools: Vec<String>,
    /// Whether to allow unknown tools
    allow_unknown_tools: bool,
}

impl SettingsValidator {
    /// Create a new validator
    pub fn new() -> Self {
        Self {
            known_tools: vec![
                "Read".to_string(),
                "Write".to_string(),
                "Edit".to_string(),
                "Bash".to_string(),
                "Glob".to_string(),
                "Grep".to_string(),
                "Task".to_string(),
                "WebFetch".to_string(),
                "WebSearch".to_string(),
                "TodoWrite".to_string(),
                "AskUserQuestion".to_string(),
                "NotebookEdit".to_string(),
            ],
            allow_unknown_tools: true,
        }
    }

    /// Create a strict validator that rejects unknown tools
    pub fn strict() -> Self {
        Self {
            allow_unknown_tools: false,
            ..Self::new()
        }
    }

    /// Add known tool names
    pub fn with_tools(mut self, tools: impl IntoIterator<Item = String>) -> Self {
        self.known_tools.extend(tools);
        self
    }

    /// Validate settings
    pub fn validate(&self, settings: &Settings) -> SageResult<()> {
        let mut errors = Vec::new();

        // Validate permission patterns
        if let Err(e) = self.validate_permissions(&settings.permissions) {
            errors.push(e);
        }

        // Validate tool settings
        if let Err(e) = self.validate_tools(&settings.tools) {
            errors.push(e);
        }

        // Validate model settings
        if let Some(ref temp) = settings.model.temperature {
            if !(0.0..=2.0).contains(temp) {
                errors.push("Model temperature must be between 0.0 and 2.0".to_string());
            }
        }

        if let Some(max_tokens) = settings.model.max_tokens {
            if max_tokens == 0 {
                errors.push("Model max_tokens must be greater than 0".to_string());
            }
        }

        // Validate UI settings
        if let Some(ref theme) = settings.ui.theme {
            let valid_themes = ["light", "dark", "auto", "system"];
            if !valid_themes.contains(&theme.to_lowercase().as_str()) {
                errors.push(format!(
                    "Invalid theme '{}'. Valid options: {:?}",
                    theme, valid_themes
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(SageError::Config(format!(
                "Settings validation failed:\n- {}",
                errors.join("\n- ")
            )))
        }
    }

    /// Validate permission settings
    fn validate_permissions(&self, permissions: &PermissionSettings) -> Result<(), String> {
        // Validate allow patterns
        for pattern in &permissions.allow {
            self.validate_permission_pattern(pattern, "allow")?;
        }

        // Validate deny patterns
        for pattern in &permissions.deny {
            self.validate_permission_pattern(pattern, "deny")?;
        }

        // Check for conflicting patterns
        for allow_pattern in &permissions.allow {
            for deny_pattern in &permissions.deny {
                if allow_pattern == deny_pattern {
                    return Err(format!(
                        "Pattern '{}' appears in both allow and deny lists",
                        allow_pattern
                    ));
                }
            }
        }

        Ok(())
    }

    /// Validate a single permission pattern
    fn validate_permission_pattern(&self, pattern: &str, list_name: &str) -> Result<(), String> {
        let parsed = PermissionSettings::parse_pattern(pattern)
            .ok_or_else(|| format!("Invalid {} pattern: '{}'", list_name, pattern))?;

        // Check if tool is known
        if !self.allow_unknown_tools {
            let tool_lower = parsed.tool_name.to_lowercase();
            let known = self
                .known_tools
                .iter()
                .any(|t| t.to_lowercase() == tool_lower);

            if !known {
                return Err(format!(
                    "Unknown tool '{}' in {} pattern. Known tools: {:?}",
                    parsed.tool_name, list_name, self.known_tools
                ));
            }
        }

        // Validate path patterns don't contain dangerous sequences
        if let Some(ref arg_pattern) = parsed.arg_pattern {
            if arg_pattern.contains("..") {
                return Err(format!(
                    "Pattern '{}' contains potentially dangerous '..' sequence",
                    pattern
                ));
            }
        }

        Ok(())
    }

    /// Validate tool settings
    fn validate_tools(&self, tools: &ToolSettings) -> Result<(), String> {
        // Check for tools in both enabled and disabled lists
        for tool in &tools.enabled {
            if tools.disabled.contains(tool) {
                return Err(format!(
                    "Tool '{}' appears in both enabled and disabled lists",
                    tool
                ));
            }
        }

        // Validate timeout values
        for (tool, timeout) in &tools.timeouts {
            if *timeout == 0 {
                return Err(format!("Timeout for tool '{}' cannot be 0", tool));
            }
            // Maximum timeout: 10 minutes
            if *timeout > 600_000 {
                return Err(format!(
                    "Timeout for tool '{}' exceeds maximum (600000ms / 10 minutes)",
                    tool
                ));
            }
        }

        Ok(())
    }
}

/// Validation result with details
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether validation passed
    pub valid: bool,
    /// Warning messages (non-fatal)
    pub warnings: Vec<String>,
    /// Error messages (fatal)
    pub errors: Vec<String>,
}

impl ValidationResult {
    /// Create a successful validation result
    pub fn success() -> Self {
        Self {
            valid: true,
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Create a failed validation result
    pub fn failure(errors: Vec<String>) -> Self {
        Self {
            valid: false,
            warnings: Vec::new(),
            errors,
        }
    }

    /// Add a warning
    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }

    /// Add an error
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.errors.push(error.into());
        self.valid = false;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_empty_settings() {
        let validator = SettingsValidator::new();
        let settings = Settings::default();

        assert!(validator.validate(&settings).is_ok());
    }

    #[test]
    fn test_validate_valid_patterns() {
        let validator = SettingsValidator::new();
        let mut settings = Settings::default();
        settings.permissions.allow = vec![
            "Read(src/*)".to_string(),
            "Write(src/**/*)".to_string(),
            "Bash(npm *)".to_string(),
        ];

        assert!(validator.validate(&settings).is_ok());
    }

    #[test]
    fn test_validate_conflicting_patterns() {
        let validator = SettingsValidator::new();
        let mut settings = Settings::default();
        settings.permissions.allow = vec!["Read(src/*)".to_string()];
        settings.permissions.deny = vec!["Read(src/*)".to_string()];

        assert!(validator.validate(&settings).is_err());
    }

    #[test]
    fn test_validate_unknown_tool_strict() {
        let validator = SettingsValidator::strict();
        let mut settings = Settings::default();
        settings.permissions.allow = vec!["UnknownTool(*)".to_string()];

        assert!(validator.validate(&settings).is_err());
    }

    #[test]
    fn test_validate_unknown_tool_lenient() {
        let validator = SettingsValidator::new();
        let mut settings = Settings::default();
        settings.permissions.allow = vec!["UnknownTool(*)".to_string()];

        // Should pass with default (lenient) validation
        assert!(validator.validate(&settings).is_ok());
    }

    #[test]
    fn test_validate_tool_in_both_lists() {
        let validator = SettingsValidator::new();
        let mut settings = Settings::default();
        settings.tools.enabled = vec!["bash".to_string()];
        settings.tools.disabled = vec!["bash".to_string()];

        assert!(validator.validate(&settings).is_err());
    }

    #[test]
    fn test_validate_invalid_timeout() {
        let validator = SettingsValidator::new();
        let mut settings = Settings::default();
        settings.tools.timeouts.insert("bash".to_string(), 0);

        assert!(validator.validate(&settings).is_err());
    }

    #[test]
    fn test_validate_timeout_too_large() {
        let validator = SettingsValidator::new();
        let mut settings = Settings::default();
        settings
            .tools
            .timeouts
            .insert("bash".to_string(), 1_000_000);

        assert!(validator.validate(&settings).is_err());
    }

    #[test]
    fn test_validate_invalid_temperature() {
        let validator = SettingsValidator::new();
        let mut settings = Settings::default();
        settings.model.temperature = Some(3.0);

        assert!(validator.validate(&settings).is_err());
    }

    #[test]
    fn test_validate_valid_temperature() {
        let validator = SettingsValidator::new();
        let mut settings = Settings::default();
        settings.model.temperature = Some(0.7);

        assert!(validator.validate(&settings).is_ok());
    }

    #[test]
    fn test_validate_invalid_theme() {
        let validator = SettingsValidator::new();
        let mut settings = Settings::default();
        settings.ui.theme = Some("invalid_theme".to_string());

        assert!(validator.validate(&settings).is_err());
    }

    #[test]
    fn test_validate_valid_theme() {
        let validator = SettingsValidator::new();
        let mut settings = Settings::default();
        settings.ui.theme = Some("dark".to_string());

        assert!(validator.validate(&settings).is_ok());
    }

    #[test]
    fn test_validate_dangerous_pattern() {
        let validator = SettingsValidator::new();
        let mut settings = Settings::default();
        settings.permissions.allow = vec!["Read(../../../etc/passwd)".to_string()];

        assert!(validator.validate(&settings).is_err());
    }

    #[test]
    fn test_validation_result() {
        let result = ValidationResult::success()
            .with_warning("This is a warning")
            .with_error("This is an error");

        assert!(!result.valid);
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.errors.len(), 1);
    }
}
