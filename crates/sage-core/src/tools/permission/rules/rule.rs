//! Permission rule definition

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::hooks::matcher::matches as pattern_matches;
use crate::tools::permission::types::{PermissionBehavior, RuleSource};

fn default_true() -> bool {
    true
}

/// A permission rule with pattern matchers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRule {
    /// Source of this rule
    #[serde(default)]
    pub source: RuleSource,
    /// Tool name pattern (e.g., "bash", "edit|write", "^file_.*")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_pattern: Option<String>,
    /// File path pattern (for file tools like read, write, edit)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_pattern: Option<String>,
    /// Command pattern (for bash tool)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_pattern: Option<String>,
    /// The permission behavior
    pub behavior: PermissionBehavior,
    /// Optional reason for this rule
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Whether this rule is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl PermissionRule {
    /// Create a new permission rule
    pub fn new(behavior: PermissionBehavior) -> Self {
        Self {
            source: RuleSource::default(),
            tool_pattern: None,
            path_pattern: None,
            command_pattern: None,
            behavior,
            reason: None,
            enabled: true,
        }
    }

    /// Set the rule source
    pub fn with_source(mut self, source: RuleSource) -> Self {
        self.source = source;
        self
    }

    /// Set the tool pattern
    pub fn with_tool_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.tool_pattern = Some(pattern.into());
        self
    }

    /// Set the path pattern
    pub fn with_path_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.path_pattern = Some(pattern.into());
        self
    }

    /// Set the command pattern
    pub fn with_command_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.command_pattern = Some(pattern.into());
        self
    }

    /// Set the reason
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// Check if this rule matches a given tool call
    pub fn matches(&self, tool_name: &str, path: Option<&str>, command: Option<&str>) -> bool {
        if !self.enabled {
            return false;
        }

        // Tool name must match if pattern is specified
        if !pattern_matches(self.tool_pattern.as_deref(), tool_name) {
            return false;
        }

        // Path must match if pattern is specified and path is provided
        if self.path_pattern.is_some() {
            match path {
                Some(p) => {
                    if !pattern_matches(self.path_pattern.as_deref(), p) {
                        return false;
                    }
                }
                None => return false, // Path pattern specified but no path provided
            }
        }

        // Command must match if pattern is specified and command is provided
        if self.command_pattern.is_some() {
            match command {
                Some(c) => {
                    if !pattern_matches(self.command_pattern.as_deref(), c) {
                        return false;
                    }
                }
                None => return false, // Command pattern specified but no command provided
            }
        }

        true
    }
}

impl fmt::Display for PermissionRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (source: {})", self.behavior, self.source)?;
        if let Some(ref tool) = self.tool_pattern {
            write!(f, " tool={}", tool)?;
        }
        if let Some(ref path) = self.path_pattern {
            write!(f, " path={}", path)?;
        }
        if let Some(ref cmd) = self.command_pattern {
            write!(f, " cmd={}", cmd)?;
        }
        Ok(())
    }
}
