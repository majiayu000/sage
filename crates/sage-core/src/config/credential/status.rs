//! Configuration status detection and reporting
//!
//! This module provides types and functions to detect the current state of
//! configuration, helping the system provide appropriate guidance to users.

use serde::{Deserialize, Serialize};
use std::fmt;

/// The overall status of the configuration
///
/// Used to determine what actions the system should take:
/// - `Complete`: Ready to run, all required credentials available
/// - `Partial`: Some providers configured, may need more for full functionality
/// - `Unconfigured`: No credentials found, needs onboarding
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigStatus {
    /// All required configuration is present and valid
    Complete,
    /// Some configuration exists but may be incomplete
    Partial,
    /// No configuration found, system needs setup
    Unconfigured,
}

impl ConfigStatus {
    /// Check if the configuration is ready for operation
    pub fn is_ready(&self) -> bool {
        matches!(self, ConfigStatus::Complete | ConfigStatus::Partial)
    }

    /// Check if onboarding should be triggered
    pub fn needs_onboarding(&self) -> bool {
        matches!(self, ConfigStatus::Unconfigured)
    }

    /// Check if status hints should be shown
    pub fn should_show_hint(&self) -> bool {
        matches!(self, ConfigStatus::Partial | ConfigStatus::Unconfigured)
    }

    /// Get a human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            ConfigStatus::Complete => "Configuration complete",
            ConfigStatus::Partial => "Configuration incomplete",
            ConfigStatus::Unconfigured => "No configuration found",
        }
    }
}

impl Default for ConfigStatus {
    fn default() -> Self {
        ConfigStatus::Unconfigured
    }
}

impl fmt::Display for ConfigStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

/// Detailed status report for configuration
#[derive(Debug, Clone, Default)]
pub struct ConfigStatusReport {
    /// Overall configuration status
    pub status: ConfigStatus,
    /// List of configured providers
    pub configured_providers: Vec<String>,
    /// List of providers with missing credentials
    pub missing_credentials: Vec<String>,
    /// Human-readable status message
    pub message: String,
    /// Suggested action for the user
    pub suggestion: Option<String>,
}

impl ConfigStatusReport {
    /// Create a new status report
    pub fn new(status: ConfigStatus) -> Self {
        Self {
            status,
            message: status.description().to_string(),
            ..Default::default()
        }
    }

    /// Create a complete status report
    pub fn complete(configured_providers: Vec<String>) -> Self {
        Self {
            status: ConfigStatus::Complete,
            configured_providers,
            missing_credentials: vec![],
            message: "All providers configured and ready".to_string(),
            suggestion: None,
        }
    }

    /// Create a partial status report
    pub fn partial(configured: Vec<String>, missing: Vec<String>) -> Self {
        let message = format!(
            "{} provider(s) configured, {} need credentials",
            configured.len(),
            missing.len()
        );
        Self {
            status: ConfigStatus::Partial,
            configured_providers: configured,
            missing_credentials: missing.clone(),
            message,
            suggestion: Some(format!("Run /login to configure: {}", missing.join(", "))),
        }
    }

    /// Create an unconfigured status report
    pub fn unconfigured() -> Self {
        Self {
            status: ConfigStatus::Unconfigured,
            configured_providers: vec![],
            missing_credentials: vec!["anthropic".to_string(), "openai".to_string()],
            message: "No API keys configured".to_string(),
            suggestion: Some("Run /login to get started".to_string()),
        }
    }

    /// Add a configured provider
    pub fn with_configured(mut self, provider: impl Into<String>) -> Self {
        self.configured_providers.push(provider.into());
        self
    }

    /// Add a missing credential
    pub fn with_missing(mut self, provider: impl Into<String>) -> Self {
        self.missing_credentials.push(provider.into());
        self
    }

    /// Set a custom message
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    /// Set a suggestion
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_status_is_ready() {
        assert!(ConfigStatus::Complete.is_ready());
        assert!(ConfigStatus::Partial.is_ready());
        assert!(!ConfigStatus::Unconfigured.is_ready());
    }

    #[test]
    fn test_config_status_needs_onboarding() {
        assert!(!ConfigStatus::Complete.needs_onboarding());
        assert!(!ConfigStatus::Partial.needs_onboarding());
        assert!(ConfigStatus::Unconfigured.needs_onboarding());
    }

    #[test]
    fn test_config_status_should_show_hint() {
        assert!(!ConfigStatus::Complete.should_show_hint());
        assert!(ConfigStatus::Partial.should_show_hint());
        assert!(ConfigStatus::Unconfigured.should_show_hint());
    }

    #[test]
    fn test_config_status_description() {
        assert_eq!(
            ConfigStatus::Complete.description(),
            "Configuration complete"
        );
        assert_eq!(
            ConfigStatus::Partial.description(),
            "Configuration incomplete"
        );
        assert_eq!(
            ConfigStatus::Unconfigured.description(),
            "No configuration found"
        );
    }

    #[test]
    fn test_config_status_display() {
        assert_eq!(
            format!("{}", ConfigStatus::Complete),
            "Configuration complete"
        );
        assert_eq!(
            format!("{}", ConfigStatus::Partial),
            "Configuration incomplete"
        );
        assert_eq!(
            format!("{}", ConfigStatus::Unconfigured),
            "No configuration found"
        );
    }

    #[test]
    fn test_config_status_default() {
        assert_eq!(ConfigStatus::default(), ConfigStatus::Unconfigured);
    }

    #[test]
    fn test_config_status_serialize() {
        let status = ConfigStatus::Complete;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"complete\"");

        let status = ConfigStatus::Partial;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"partial\"");

        let status = ConfigStatus::Unconfigured;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"unconfigured\"");
    }

    #[test]
    fn test_config_status_deserialize() {
        let status: ConfigStatus = serde_json::from_str("\"complete\"").unwrap();
        assert_eq!(status, ConfigStatus::Complete);

        let status: ConfigStatus = serde_json::from_str("\"partial\"").unwrap();
        assert_eq!(status, ConfigStatus::Partial);

        let status: ConfigStatus = serde_json::from_str("\"unconfigured\"").unwrap();
        assert_eq!(status, ConfigStatus::Unconfigured);
    }

    #[test]
    fn test_status_report_new() {
        let report = ConfigStatusReport::new(ConfigStatus::Complete);
        assert_eq!(report.status, ConfigStatus::Complete);
        assert_eq!(report.message, "Configuration complete");
        assert!(report.configured_providers.is_empty());
        assert!(report.missing_credentials.is_empty());
        assert!(report.suggestion.is_none());
    }

    #[test]
    fn test_status_report_complete() {
        let report =
            ConfigStatusReport::complete(vec!["anthropic".to_string(), "openai".to_string()]);
        assert_eq!(report.status, ConfigStatus::Complete);
        assert_eq!(report.configured_providers.len(), 2);
        assert!(report.missing_credentials.is_empty());
        assert!(report.suggestion.is_none());
    }

    #[test]
    fn test_status_report_partial() {
        let report =
            ConfigStatusReport::partial(vec!["anthropic".to_string()], vec!["openai".to_string()]);
        assert_eq!(report.status, ConfigStatus::Partial);
        assert_eq!(report.configured_providers.len(), 1);
        assert_eq!(report.missing_credentials.len(), 1);
        assert!(report.suggestion.is_some());
        assert!(report.suggestion.as_ref().unwrap().contains("/login"));
    }

    #[test]
    fn test_status_report_unconfigured() {
        let report = ConfigStatusReport::unconfigured();
        assert_eq!(report.status, ConfigStatus::Unconfigured);
        assert!(report.configured_providers.is_empty());
        assert!(!report.missing_credentials.is_empty());
        assert!(report.suggestion.is_some());
        assert!(report.suggestion.as_ref().unwrap().contains("/login"));
    }

    #[test]
    fn test_status_report_builder() {
        let report = ConfigStatusReport::new(ConfigStatus::Partial)
            .with_configured("anthropic")
            .with_missing("openai")
            .with_message("Custom message")
            .with_suggestion("Custom suggestion");

        assert_eq!(report.status, ConfigStatus::Partial);
        assert_eq!(report.configured_providers, vec!["anthropic"]);
        assert_eq!(report.missing_credentials, vec!["openai"]);
        assert_eq!(report.message, "Custom message");
        assert_eq!(report.suggestion, Some("Custom suggestion".to_string()));
    }

    #[test]
    fn test_status_report_default() {
        let report = ConfigStatusReport::default();
        assert_eq!(report.status, ConfigStatus::Unconfigured);
        assert!(report.configured_providers.is_empty());
        assert!(report.missing_credentials.is_empty());
        assert!(report.message.is_empty());
        assert!(report.suggestion.is_none());
    }
}
