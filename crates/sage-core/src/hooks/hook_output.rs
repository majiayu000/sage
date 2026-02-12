//! Hook output types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

use super::hook_types::{HookPermissionDecision, default_continue};

/// Output from a hook
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HookOutput {
    #[serde(default = "default_continue")]
    pub should_continue: bool,
    pub modified_input: Option<serde_json::Value>,
    pub permission_decision: Option<HookPermissionDecision>,
    #[serde(default)]
    pub additional_context: Vec<String>,
    pub reason: Option<String>,
    pub system_message: Option<String>,
    #[serde(default)]
    pub data: HashMap<String, serde_json::Value>,
}

impl Default for HookOutput {
    fn default() -> Self {
        Self {
            should_continue: true,
            modified_input: None,
            permission_decision: None,
            additional_context: Vec::new(),
            reason: None,
            system_message: None,
            data: HashMap::new(),
        }
    }
}

impl HookOutput {
    /// Create a new hook output that allows continuation
    pub fn allow() -> Self {
        Self {
            should_continue: true,
            ..Default::default()
        }
    }

    /// Create a new hook output that allows continuation (alias for compatibility)
    pub fn continue_execution() -> Self {
        Self::allow()
    }

    /// Create a new hook output that denies continuation
    pub fn deny(reason: impl Into<String>) -> Self {
        Self {
            should_continue: false,
            reason: Some(reason.into()),
            ..Default::default()
        }
    }

    /// Create a new hook output that blocks execution (alias for compatibility)
    pub fn block(message: impl Into<String>) -> Self {
        Self::deny(message)
    }

    /// Add data to the hook output
    pub fn with_data(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.data.insert(key.into(), value);
        self
    }

    /// Set the modified input
    pub fn with_modified_input(mut self, input: serde_json::Value) -> Self {
        self.modified_input = Some(input);
        self
    }

    /// Set the permission decision
    pub fn with_permission(mut self, decision: HookPermissionDecision) -> Self {
        self.permission_decision = Some(decision);
        self
    }

    /// Add additional context
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.additional_context.push(context.into());
        self
    }

    /// Set the system message
    pub fn with_system_message(mut self, message: impl Into<String>) -> Self {
        self.system_message = Some(message.into());
        self
    }

    /// Set the reason
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }
}

impl fmt::Display for HookOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Continue: {}", self.should_continue)?;
        if let Some(decision) = &self.permission_decision {
            write!(f, ", Permission: {}", decision)?;
        }
        if let Some(reason) = &self.reason {
            write!(f, ", Reason: {}", reason)?;
        }
        if let Some(msg) = &self.system_message {
            write!(f, ", Message: {}", msg)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_output_allow() {
        let output = HookOutput::allow();
        assert!(output.should_continue);
        assert_eq!(output.permission_decision, None);
    }

    #[test]
    fn test_hook_output_deny() {
        let output = HookOutput::deny("Test reason");
        assert!(!output.should_continue);
        assert_eq!(output.reason, Some("Test reason".to_string()));
    }

    #[test]
    fn test_hook_output_builder() {
        let output = HookOutput::allow()
            .with_permission(HookPermissionDecision::Allow)
            .with_context("Additional info")
            .with_system_message("System msg")
            .with_reason("Test reason");

        assert!(output.should_continue);
        assert_eq!(
            output.permission_decision,
            Some(HookPermissionDecision::Allow)
        );
        assert_eq!(output.additional_context.len(), 1);
        assert_eq!(output.system_message, Some("System msg".to_string()));
        assert_eq!(output.reason, Some("Test reason".to_string()));
    }

    #[test]
    fn test_hook_output_display() {
        let output = HookOutput::allow()
            .with_permission(HookPermissionDecision::Allow)
            .with_reason("Test");
        let display = format!("{}", output);
        assert!(display.contains("Continue: true"));
        assert!(display.contains("Permission: Allow"));
        assert!(display.contains("Reason: Test"));
    }

    #[test]
    fn test_hook_output_default() {
        let output = HookOutput::default();
        assert!(output.should_continue);
        assert_eq!(output.permission_decision, None);
        assert_eq!(output.additional_context.len(), 0);
    }

    #[test]
    fn test_hook_output_serialization() {
        let output = HookOutput::allow().with_reason("Test");
        let json = serde_json::to_string(&output).unwrap();
        let deserialized: HookOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(output, deserialized);
    }
}
