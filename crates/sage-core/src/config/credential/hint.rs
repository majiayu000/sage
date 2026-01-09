//! Status bar hints for configuration state
//!
//! This module provides hints that can be shown to users about their
//! configuration status, similar to Claude Code's status bar.

use super::status::{ConfigStatus, ConfigStatusReport};
use serde::{Deserialize, Serialize};

/// Type of status hint
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HintType {
    /// Informational hint
    Info,
    /// Warning hint
    Warning,
    /// Error hint
    Error,
    /// Success hint
    Success,
}

impl HintType {
    /// Get the icon for terminal display
    pub fn icon(&self) -> &'static str {
        match self {
            HintType::Info => "ℹ",
            HintType::Warning => "⚠",
            HintType::Error => "✗",
            HintType::Success => "✓",
        }
    }

    /// Get the ANSI color code for terminal display
    pub fn color(&self) -> &'static str {
        match self {
            HintType::Info => "\x1b[36m",  // Cyan
            HintType::Warning => "\x1b[33m", // Yellow
            HintType::Error => "\x1b[31m",   // Red
            HintType::Success => "\x1b[32m", // Green
        }
    }

    /// Reset ANSI color
    pub fn reset() -> &'static str {
        "\x1b[0m"
    }
}

/// A status bar hint with message and action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusBarHint {
    /// Type of hint
    pub hint_type: HintType,
    /// Short message to display
    pub message: String,
    /// Suggested action/command
    pub action: Option<String>,
    /// Full help text (for expanded view)
    pub help_text: Option<String>,
}

impl StatusBarHint {
    /// Create a new hint
    pub fn new(hint_type: HintType, message: impl Into<String>) -> Self {
        Self {
            hint_type,
            message: message.into(),
            action: None,
            help_text: None,
        }
    }

    /// Create an info hint
    pub fn info(message: impl Into<String>) -> Self {
        Self::new(HintType::Info, message)
    }

    /// Create a warning hint
    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(HintType::Warning, message)
    }

    /// Create an error hint
    pub fn error(message: impl Into<String>) -> Self {
        Self::new(HintType::Error, message)
    }

    /// Create a success hint
    pub fn success(message: impl Into<String>) -> Self {
        Self::new(HintType::Success, message)
    }

    /// Add an action suggestion
    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.action = Some(action.into());
        self
    }

    /// Add help text
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help_text = Some(help.into());
        self
    }

    /// Format for terminal display (compact)
    pub fn format_compact(&self) -> String {
        let icon = self.hint_type.icon();
        let color = self.hint_type.color();
        let reset = HintType::reset();

        if let Some(action) = &self.action {
            format!("{}{} {} · {}{}", color, icon, self.message, action, reset)
        } else {
            format!("{}{} {}{}", color, icon, self.message, reset)
        }
    }

    /// Format for terminal display (full)
    pub fn format_full(&self) -> String {
        let mut output = self.format_compact();

        if let Some(help) = &self.help_text {
            output.push_str(&format!("\n  {}", help));
        }

        output
    }

    /// Format without colors (for non-terminal output)
    pub fn format_plain(&self) -> String {
        let icon = self.hint_type.icon();

        if let Some(action) = &self.action {
            format!("{} {} · {}", icon, self.message, action)
        } else {
            format!("{} {}", icon, self.message)
        }
    }
}

impl std::fmt::Display for StatusBarHint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.format_compact())
    }
}

/// Generate a hint from configuration status
pub fn hint_from_status(status: &ConfigStatusReport) -> Option<StatusBarHint> {
    match status.status {
        ConfigStatus::Complete => None, // No hint needed when everything is configured
        ConfigStatus::Partial => Some(
            StatusBarHint::warning(&status.message)
                .with_action("Run /login to configure missing providers")
                .with_help(format!(
                    "Configured: {}. Missing: {}",
                    status.configured_providers.join(", "),
                    status.missing_credentials.join(", ")
                )),
        ),
        ConfigStatus::Unconfigured => Some(
            StatusBarHint::error("Missing API key")
                .with_action("Run /login")
                .with_help(
                    "No API keys configured. Run /login to set up your preferred AI provider.",
                ),
        ),
    }
}

/// Generate a welcome hint for new users
pub fn hint_welcome() -> StatusBarHint {
    StatusBarHint::info("Welcome to Sage Agent")
        .with_action("Run /help for commands")
        .with_help("Type a message to start, or use /login to configure API keys.")
}

/// Generate a hint after successful configuration
pub fn hint_configured(provider: &str) -> StatusBarHint {
    StatusBarHint::success(format!("{} configured", provider))
        .with_help("You're all set! Start chatting with the AI.")
}

/// Generate a hint for validation failure
pub fn hint_validation_failed(error: &str) -> StatusBarHint {
    StatusBarHint::error("API key validation failed")
        .with_action("Check your key and try again")
        .with_help(error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hint_type_icon() {
        assert_eq!(HintType::Info.icon(), "ℹ");
        assert_eq!(HintType::Warning.icon(), "⚠");
        assert_eq!(HintType::Error.icon(), "✗");
        assert_eq!(HintType::Success.icon(), "✓");
    }

    #[test]
    fn test_hint_type_color() {
        assert!(!HintType::Info.color().is_empty());
        assert!(!HintType::Warning.color().is_empty());
        assert!(!HintType::Error.color().is_empty());
        assert!(!HintType::Success.color().is_empty());
    }

    #[test]
    fn test_status_bar_hint_new() {
        let hint = StatusBarHint::new(HintType::Info, "Test message");
        assert_eq!(hint.hint_type, HintType::Info);
        assert_eq!(hint.message, "Test message");
        assert!(hint.action.is_none());
        assert!(hint.help_text.is_none());
    }

    #[test]
    fn test_status_bar_hint_shortcuts() {
        let info = StatusBarHint::info("Info");
        assert_eq!(info.hint_type, HintType::Info);

        let warning = StatusBarHint::warning("Warning");
        assert_eq!(warning.hint_type, HintType::Warning);

        let error = StatusBarHint::error("Error");
        assert_eq!(error.hint_type, HintType::Error);

        let success = StatusBarHint::success("Success");
        assert_eq!(success.hint_type, HintType::Success);
    }

    #[test]
    fn test_status_bar_hint_with_action() {
        let hint = StatusBarHint::info("Test").with_action("Run /help");
        assert_eq!(hint.action, Some("Run /help".to_string()));
    }

    #[test]
    fn test_status_bar_hint_with_help() {
        let hint = StatusBarHint::info("Test").with_help("Additional help");
        assert_eq!(hint.help_text, Some("Additional help".to_string()));
    }

    #[test]
    fn test_status_bar_hint_format_compact() {
        let hint = StatusBarHint::info("Test message");
        let formatted = hint.format_compact();
        assert!(formatted.contains("Test message"));
        assert!(formatted.contains(HintType::Info.icon()));

        let with_action = StatusBarHint::info("Test").with_action("Run /help");
        let formatted = with_action.format_compact();
        assert!(formatted.contains("Run /help"));
    }

    #[test]
    fn test_status_bar_hint_format_full() {
        let hint = StatusBarHint::info("Test").with_help("Help text");
        let formatted = hint.format_full();
        assert!(formatted.contains("Test"));
        assert!(formatted.contains("Help text"));
    }

    #[test]
    fn test_status_bar_hint_format_plain() {
        let hint = StatusBarHint::info("Test").with_action("Action");
        let formatted = hint.format_plain();
        assert!(formatted.contains("Test"));
        assert!(formatted.contains("Action"));
        // Should not contain ANSI codes
        assert!(!formatted.contains("\x1b["));
    }

    #[test]
    fn test_status_bar_hint_display() {
        let hint = StatusBarHint::info("Test");
        let display = format!("{}", hint);
        assert!(display.contains("Test"));
    }

    #[test]
    fn test_hint_from_status_complete() {
        let status = ConfigStatusReport::complete(vec!["anthropic".to_string()]);
        let hint = hint_from_status(&status);
        assert!(hint.is_none());
    }

    #[test]
    fn test_hint_from_status_partial() {
        let status = ConfigStatusReport::partial(
            vec!["anthropic".to_string()],
            vec!["openai".to_string()],
        );
        let hint = hint_from_status(&status);
        assert!(hint.is_some());
        let hint = hint.unwrap();
        assert_eq!(hint.hint_type, HintType::Warning);
        assert!(hint.action.is_some());
    }

    #[test]
    fn test_hint_from_status_unconfigured() {
        let status = ConfigStatusReport::unconfigured();
        let hint = hint_from_status(&status);
        assert!(hint.is_some());
        let hint = hint.unwrap();
        assert_eq!(hint.hint_type, HintType::Error);
        assert!(hint.message.contains("Missing API key"));
    }

    #[test]
    fn test_hint_welcome() {
        let hint = hint_welcome();
        assert_eq!(hint.hint_type, HintType::Info);
        assert!(hint.message.contains("Welcome"));
    }

    #[test]
    fn test_hint_configured() {
        let hint = hint_configured("anthropic");
        assert_eq!(hint.hint_type, HintType::Success);
        assert!(hint.message.contains("anthropic"));
    }

    #[test]
    fn test_hint_validation_failed() {
        let hint = hint_validation_failed("Invalid format");
        assert_eq!(hint.hint_type, HintType::Error);
        assert!(hint.help_text.as_ref().unwrap().contains("Invalid format"));
    }

    #[test]
    fn test_hint_type_serialize() {
        let hint_type = HintType::Warning;
        let json = serde_json::to_string(&hint_type).unwrap();
        assert_eq!(json, "\"warning\"");
    }

    #[test]
    fn test_hint_type_deserialize() {
        let hint_type: HintType = serde_json::from_str("\"error\"").unwrap();
        assert_eq!(hint_type, HintType::Error);
    }
}
