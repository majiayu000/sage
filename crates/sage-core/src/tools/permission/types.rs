//! Core types for the permission system

use serde::{Deserialize, Serialize};
use std::fmt;

/// Risk level for tool operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RiskLevel {
    /// Low risk - read-only operations, no side effects
    Low,
    /// Medium risk - local modifications, reversible
    Medium,
    /// High risk - significant changes, network access
    High,
    /// Critical risk - system modifications, irreversible operations
    Critical,
}

impl RiskLevel {
    /// Check if this risk level requires user confirmation by default
    pub fn requires_confirmation(&self) -> bool {
        matches!(self, RiskLevel::High | RiskLevel::Critical)
    }

    /// Get a human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            RiskLevel::Low => "Low risk - safe, read-only operation",
            RiskLevel::Medium => "Medium risk - local changes, reversible",
            RiskLevel::High => "High risk - significant changes",
            RiskLevel::Critical => "Critical risk - irreversible or system-wide",
        }
    }
}

impl Default for RiskLevel {
    fn default() -> Self {
        RiskLevel::Medium
    }
}

/// Source of a permission rule
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleSource {
    /// Project settings (.sage/settings.json or .claude/settings.json)
    ProjectSettings,
    /// Local project settings (.sage/settings.local.json)
    LocalSettings,
    /// User-level settings (~/.config/sage/settings.json)
    UserSettings,
    /// Session-level settings (runtime)
    SessionSettings,
    /// Command line argument
    CliArg,
    /// Builtin default rules
    Builtin,
}

impl RuleSource {
    /// Get the priority of this rule source (lower = higher priority)
    pub fn priority(&self) -> u8 {
        match self {
            RuleSource::CliArg => 0, // Highest priority
            RuleSource::SessionSettings => 1,
            RuleSource::LocalSettings => 2,
            RuleSource::ProjectSettings => 3,
            RuleSource::UserSettings => 4,
            RuleSource::Builtin => 5, // Lowest priority
        }
    }

    /// Get a human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            RuleSource::ProjectSettings => "project settings",
            RuleSource::LocalSettings => "local settings",
            RuleSource::UserSettings => "user settings",
            RuleSource::SessionSettings => "session settings",
            RuleSource::CliArg => "command line",
            RuleSource::Builtin => "builtin",
        }
    }
}

impl fmt::Display for RuleSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl Default for RuleSource {
    fn default() -> Self {
        RuleSource::Builtin
    }
}

/// Permission behavior for a rule
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionBehavior {
    /// Allow the operation
    Allow,
    /// Deny the operation
    Deny,
    /// Ask the user
    Ask,
    /// Pass through to next rule (no decision)
    Passthrough,
}

impl fmt::Display for PermissionBehavior {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PermissionBehavior::Allow => write!(f, "allow"),
            PermissionBehavior::Deny => write!(f, "deny"),
            PermissionBehavior::Ask => write!(f, "ask"),
            PermissionBehavior::Passthrough => write!(f, "passthrough"),
        }
    }
}

impl Default for PermissionBehavior {
    fn default() -> Self {
        PermissionBehavior::Ask
    }
}
