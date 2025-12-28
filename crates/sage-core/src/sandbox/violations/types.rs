//! Violation types for security tracking.

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Type of security violation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ViolationType {
    /// Heredoc injection attempt
    HeredocInjection,
    /// Shell metacharacter abuse
    ShellMetacharacterAbuse,
    /// Variable injection (e.g., > $file)
    VariableInjection,
    /// Dangerous pattern detected
    DangerousPattern,
    /// Critical path removal attempt
    CriticalPathRemoval,
    /// Sensitive file access attempt
    SensitiveFileAccess,
    /// Path access denied
    PathAccessDenied,
    /// Command blocked by policy
    CommandBlocked,
    /// Write to disallowed temp path
    DisallowedTempWrite,
    /// Network access violation
    NetworkViolation,
}

impl ViolationType {
    /// Get a human-readable name
    pub fn as_str(&self) -> &'static str {
        match self {
            ViolationType::HeredocInjection => "heredoc_injection",
            ViolationType::ShellMetacharacterAbuse => "shell_metacharacter_abuse",
            ViolationType::VariableInjection => "variable_injection",
            ViolationType::DangerousPattern => "dangerous_pattern",
            ViolationType::CriticalPathRemoval => "critical_path_removal",
            ViolationType::SensitiveFileAccess => "sensitive_file_access",
            ViolationType::PathAccessDenied => "path_access_denied",
            ViolationType::CommandBlocked => "command_blocked",
            ViolationType::DisallowedTempWrite => "disallowed_temp_write",
            ViolationType::NetworkViolation => "network_violation",
        }
    }

    /// Get the default severity for this violation type
    pub fn default_severity(&self) -> ViolationSeverity {
        match self {
            ViolationType::HeredocInjection => ViolationSeverity::High,
            ViolationType::ShellMetacharacterAbuse => ViolationSeverity::Medium,
            ViolationType::VariableInjection => ViolationSeverity::High,
            ViolationType::DangerousPattern => ViolationSeverity::Medium,
            ViolationType::CriticalPathRemoval => ViolationSeverity::Critical,
            ViolationType::SensitiveFileAccess => ViolationSeverity::High,
            ViolationType::PathAccessDenied => ViolationSeverity::Medium,
            ViolationType::CommandBlocked => ViolationSeverity::Medium,
            ViolationType::DisallowedTempWrite => ViolationSeverity::Low,
            ViolationType::NetworkViolation => ViolationSeverity::Medium,
        }
    }
}

/// Severity level of a violation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ViolationSeverity {
    /// Low severity - informational
    Low,
    /// Medium severity - potential issue
    Medium,
    /// High severity - security concern
    High,
    /// Critical severity - must be blocked
    Critical,
}

impl ViolationSeverity {
    /// Get a human-readable name
    pub fn as_str(&self) -> &'static str {
        match self {
            ViolationSeverity::Low => "low",
            ViolationSeverity::Medium => "medium",
            ViolationSeverity::High => "high",
            ViolationSeverity::Critical => "critical",
        }
    }
}

/// A recorded security violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    /// Type of violation
    pub violation_type: ViolationType,
    /// Severity level
    pub severity: ViolationSeverity,
    /// Human-readable description
    pub message: String,
    /// The command or path that triggered the violation
    pub trigger: String,
    /// Timestamp when the violation occurred
    #[serde(with = "system_time_serde")]
    pub timestamp: SystemTime,
    /// Whether the action was blocked
    pub blocked: bool,
    /// Additional context
    pub context: Option<String>,
}

impl Violation {
    /// Create a new violation
    pub fn new(
        violation_type: ViolationType,
        message: impl Into<String>,
        trigger: impl Into<String>,
        blocked: bool,
    ) -> Self {
        Self {
            violation_type,
            severity: violation_type.default_severity(),
            message: message.into(),
            trigger: trigger.into(),
            timestamp: SystemTime::now(),
            blocked,
            context: None,
        }
    }

    /// Create a violation with custom severity
    pub fn with_severity(mut self, severity: ViolationSeverity) -> Self {
        self.severity = severity;
        self
    }

    /// Add context to the violation
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Create a blocked violation
    pub fn blocked(
        violation_type: ViolationType,
        message: impl Into<String>,
        trigger: impl Into<String>,
    ) -> Self {
        Self::new(violation_type, message, trigger, true)
    }

    /// Create a warning violation (not blocked)
    pub fn warning(
        violation_type: ViolationType,
        message: impl Into<String>,
        trigger: impl Into<String>,
    ) -> Self {
        Self::new(violation_type, message, trigger, false)
    }
}

/// Custom serde for SystemTime
mod system_time_serde {
    use serde::{self, Deserialize, Deserializer, Serializer};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let duration = time.duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO);
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + Duration::from_secs(secs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_violation_type_severity() {
        assert_eq!(
            ViolationType::CriticalPathRemoval.default_severity(),
            ViolationSeverity::Critical
        );
        assert_eq!(
            ViolationType::HeredocInjection.default_severity(),
            ViolationSeverity::High
        );
        assert_eq!(
            ViolationType::DisallowedTempWrite.default_severity(),
            ViolationSeverity::Low
        );
    }

    #[test]
    fn test_violation_creation() {
        let v = Violation::blocked(
            ViolationType::CriticalPathRemoval,
            "Attempted to remove /",
            "rm -rf /",
        );
        assert!(v.blocked);
        assert_eq!(v.severity, ViolationSeverity::Critical);
    }

    #[test]
    fn test_violation_with_context() {
        let v = Violation::warning(
            ViolationType::SensitiveFileAccess,
            "Accessing .bashrc",
            "cat ~/.bashrc",
        )
        .with_context("User's shell configuration");

        assert!(!v.blocked);
        assert!(v.context.is_some());
    }

    #[test]
    fn test_severity_ordering() {
        assert!(ViolationSeverity::Critical > ViolationSeverity::High);
        assert!(ViolationSeverity::High > ViolationSeverity::Medium);
        assert!(ViolationSeverity::Medium > ViolationSeverity::Low);
    }
}
