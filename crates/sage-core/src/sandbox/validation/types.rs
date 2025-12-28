//! Types for command validation following Claude Code patterns.

use serde::{Deserialize, Serialize};

/// Result of command validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the command is allowed
    pub allowed: bool,
    /// Reason for blocking (if not allowed)
    pub reason: Option<String>,
    /// Non-blocking warnings
    pub warnings: Vec<ValidationWarning>,
    /// Type of check that produced this result
    pub check_type: CheckType,
}

impl ValidationResult {
    /// Create a passing validation result
    pub fn pass(check_type: CheckType) -> Self {
        Self {
            allowed: true,
            reason: None,
            warnings: Vec::new(),
            check_type,
        }
    }

    /// Create a passing result with warnings
    pub fn pass_with_warnings(check_type: CheckType, warnings: Vec<ValidationWarning>) -> Self {
        Self {
            allowed: true,
            reason: None,
            warnings,
            check_type,
        }
    }

    /// Create a blocking validation result
    pub fn block(check_type: CheckType, reason: impl Into<String>) -> Self {
        Self {
            allowed: false,
            reason: Some(reason.into()),
            warnings: Vec::new(),
            check_type,
        }
    }

    /// Create a blocking result with warnings
    pub fn block_with_warnings(
        check_type: CheckType,
        reason: impl Into<String>,
        warnings: Vec<ValidationWarning>,
    ) -> Self {
        Self {
            allowed: false,
            reason: Some(reason.into()),
            warnings,
            check_type,
        }
    }

    /// Add a warning to the result
    pub fn with_warning(mut self, warning: ValidationWarning) -> Self {
        self.warnings.push(warning);
        self
    }
}

/// Warning from validation (non-blocking)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    /// Warning message
    pub message: String,
    /// Severity level
    pub severity: WarningSeverity,
    /// Optional suggestion for fixing
    pub suggestion: Option<String>,
}

impl ValidationWarning {
    /// Create a new warning
    pub fn new(message: impl Into<String>, severity: WarningSeverity) -> Self {
        Self {
            message: message.into(),
            severity,
            suggestion: None,
        }
    }

    /// Create a warning with a suggestion
    pub fn with_suggestion(
        message: impl Into<String>,
        severity: WarningSeverity,
        suggestion: impl Into<String>,
    ) -> Self {
        Self {
            message: message.into(),
            severity,
            suggestion: Some(suggestion.into()),
        }
    }

    /// Create an info-level warning
    pub fn info(message: impl Into<String>) -> Self {
        Self::new(message, WarningSeverity::Info)
    }

    /// Create a warning-level warning
    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(message, WarningSeverity::Warning)
    }

    /// Create a critical warning
    pub fn critical(message: impl Into<String>) -> Self {
        Self::new(message, WarningSeverity::Critical)
    }
}

/// Severity level for warnings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WarningSeverity {
    /// Informational, no action needed
    Info,
    /// Warning, may cause issues
    Warning,
    /// Critical, likely to cause problems
    Critical,
}

/// Type of validation check
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckType {
    /// Heredoc injection check
    Heredoc,
    /// Shell metacharacter check
    ShellMetacharacter,
    /// Dangerous variable check
    DangerousVariable,
    /// Dangerous pattern check
    DangerousPattern,
    /// Dangerous removal check
    DangerousRemoval,
    /// Composite check (multiple)
    Composite,
}

impl CheckType {
    /// Get a human-readable name for the check type
    pub fn as_str(&self) -> &'static str {
        match self {
            CheckType::Heredoc => "heredoc_injection",
            CheckType::ShellMetacharacter => "shell_metacharacter",
            CheckType::DangerousVariable => "dangerous_variable",
            CheckType::DangerousPattern => "dangerous_pattern",
            CheckType::DangerousRemoval => "dangerous_removal",
            CheckType::Composite => "composite",
        }
    }
}

/// Context for validation
#[derive(Debug, Clone, Default)]
pub struct ValidationContext {
    /// Allow command chaining with && and ||
    pub allow_chaining: bool,
    /// Allow backgrounding with &
    pub allow_background: bool,
    /// Working directory for path resolution
    pub working_directory: Option<String>,
    /// List of dangerous commands to check against
    pub dangerous_commands: Vec<String>,
}

impl ValidationContext {
    /// Create a new context with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a permissive context (allows chaining and background)
    pub fn permissive() -> Self {
        Self {
            allow_chaining: true,
            allow_background: true,
            ..Default::default()
        }
    }

    /// Create a strict context (disallows chaining and background)
    pub fn strict() -> Self {
        Self {
            allow_chaining: false,
            allow_background: false,
            dangerous_commands: vec![
                "rm".to_string(),
                "dd".to_string(),
                "mkfs".to_string(),
                "sudo".to_string(),
                "su".to_string(),
            ],
            ..Default::default()
        }
    }

    /// Set the working directory
    pub fn with_working_directory(mut self, dir: impl Into<String>) -> Self {
        self.working_directory = Some(dir.into());
        self
    }

    /// Add dangerous commands to check
    pub fn with_dangerous_commands(mut self, commands: Vec<String>) -> Self {
        self.dangerous_commands = commands;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result_pass() {
        let result = ValidationResult::pass(CheckType::Heredoc);
        assert!(result.allowed);
        assert!(result.reason.is_none());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_validation_result_block() {
        let result = ValidationResult::block(CheckType::DangerousRemoval, "dangerous command");
        assert!(!result.allowed);
        assert_eq!(result.reason, Some("dangerous command".to_string()));
    }

    #[test]
    fn test_validation_warning() {
        let warning = ValidationWarning::with_suggestion(
            "Variable in redirect",
            WarningSeverity::Warning,
            "Use explicit path instead",
        );
        assert_eq!(warning.severity, WarningSeverity::Warning);
        assert!(warning.suggestion.is_some());
    }

    #[test]
    fn test_validation_context() {
        let ctx = ValidationContext::strict();
        assert!(!ctx.allow_chaining);
        assert!(!ctx.allow_background);
        assert!(!ctx.dangerous_commands.is_empty());
    }
}
