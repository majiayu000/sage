//! Permission system types for tool authorization

use serde::{Deserialize, Serialize};

/// Permission behavior types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionBehavior {
    /// Automatically allow
    Allow,
    /// Automatically deny
    Deny,
    /// Ask the user
    Ask,
    /// Pass through without checking
    Passthrough,
}

/// Permission check result
#[derive(Debug, Clone)]
pub enum PermissionResult {
    /// Tool execution allowed
    Allow,
    /// Tool execution denied
    Deny { message: String },
    /// Need to ask user
    Ask {
        message: String,
        suggestions: Vec<PermissionSuggestion>,
    },
}

impl PermissionResult {
    /// Check if permission is granted
    pub fn is_allowed(&self) -> bool {
        matches!(self, PermissionResult::Allow)
    }

    /// Check if permission is denied
    pub fn is_denied(&self) -> bool {
        matches!(self, PermissionResult::Deny { .. })
    }

    /// Check if user input is needed
    pub fn needs_user_input(&self) -> bool {
        matches!(self, PermissionResult::Ask { .. })
    }
}

/// Permission suggestion for the user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionSuggestion {
    /// Type of suggestion
    pub suggestion_type: SuggestionType,
    /// Tool name this applies to
    pub tool_name: String,
    /// Rule pattern/content
    pub rule_content: String,
    /// Behavior to apply
    pub behavior: PermissionBehavior,
    /// Where to save this rule
    pub destination: RuleDestination,
}

/// Types of permission suggestions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuggestionType {
    AddRule,
    RemoveRule,
    ModifyRule,
}

/// Where to save permission rules
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleDestination {
    /// Only for this session
    Session,
    /// Local project settings
    LocalSettings,
    /// User-level settings
    UserSettings,
    /// Project-specific settings
    ProjectSettings,
}
