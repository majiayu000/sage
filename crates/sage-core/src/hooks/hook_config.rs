//! Hook configuration types

use serde::{Deserialize, Serialize};
use std::fmt;

use super::hook_types::{default_enabled, default_timeout, HookImplementation, HookType};

/// Hook matcher - combines a pattern with hook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookMatcher {
    /// Pattern to match against (e.g., tool name, event type)
    /// Use None or "*" to match everything
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    /// Hook configuration
    #[serde(flatten)]
    pub hook: HookConfig,
}

impl HookMatcher {
    /// Create a new hook matcher
    pub fn new(pattern: Option<String>, hook: HookConfig) -> Self {
        Self { pattern, hook }
    }

    /// Check if this matcher matches the given value
    pub fn matches(&self, value: &str) -> bool {
        super::matcher::matches(self.pattern.as_deref(), value)
    }

    /// Check if this is a wildcard matcher (matches everything)
    pub fn is_wildcard(&self) -> bool {
        self.pattern.is_none() || self.pattern.as_deref() == Some("*")
    }
}

impl fmt::Display for HookMatcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(pattern) = &self.pattern {
            write!(f, "Pattern: {}, Hook: {}", pattern, self.hook)
        } else {
            write!(f, "Match all, Hook: {}", self.hook)
        }
    }
}

/// Hook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookConfig {
    /// Hook name
    pub name: String,
    /// Hook type (when to trigger)
    pub hook_type: HookType,
    /// Hook implementation
    #[serde(flatten)]
    pub implementation: HookImplementation,
    /// Whether this hook can block execution
    #[serde(default)]
    pub can_block: bool,
    /// Timeout in seconds (uses implementation timeout during serialization to avoid conflict)
    #[serde(default = "default_timeout", skip_serializing)]
    pub timeout_secs: u64,
    /// Whether this hook is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

impl fmt::Display for HookConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Hook '{}' ({}, {})",
            self.name, self.hook_type, self.implementation
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::command_hook::CommandHook;

    #[test]
    fn test_hook_matcher_new() {
        let config = HookConfig {
            name: "test".to_string(),
            hook_type: HookType::PreStart,
            implementation: HookImplementation::Command(CommandHook::new("echo test")),
            can_block: false,
            timeout_secs: 30,
            enabled: true,
        };
        let matcher = HookMatcher::new(Some("test".to_string()), config);
        assert_eq!(matcher.pattern, Some("test".to_string()));
        assert_eq!(matcher.hook.name, "test");
    }

    #[test]
    fn test_hook_matcher_wildcard() {
        let config = HookConfig {
            name: "test".to_string(),
            hook_type: HookType::PreStart,
            implementation: HookImplementation::Command(CommandHook::new("echo test")),
            can_block: false,
            timeout_secs: 30,
            enabled: true,
        };
        let matcher = HookMatcher::new(None, config);
        assert!(matcher.is_wildcard());
    }

    #[test]
    fn test_hook_matcher_display() {
        let config = HookConfig {
            name: "test".to_string(),
            hook_type: HookType::PreStart,
            implementation: HookImplementation::Command(CommandHook::new("echo test")),
            can_block: false,
            timeout_secs: 30,
            enabled: true,
        };
        let matcher = HookMatcher::new(Some("test".to_string()), config);
        let display = format!("{}", matcher);
        assert!(display.contains("Pattern: test"));
    }

    #[test]
    fn test_hook_config_new() {
        let config = HookConfig {
            name: "test_hook".to_string(),
            hook_type: HookType::PreStart,
            implementation: HookImplementation::Command(CommandHook::new("echo test")),
            can_block: true,
            timeout_secs: 30,
            enabled: true,
        };
        assert_eq!(config.name, "test_hook");
        assert!(config.can_block);
        assert_eq!(config.timeout_secs, 30);
    }

    #[test]
    fn test_hook_implementation_display() {
        use crate::hooks::prompt_hook::PromptHook;

        let impl1 = HookImplementation::Command(CommandHook::new("echo test"));
        assert_eq!(format!("{}", impl1), "Command: echo test");

        let impl2 = HookImplementation::Prompt(PromptHook::new("Test prompt"));
        assert_eq!(format!("{}", impl2), "Prompt: Test prompt");
    }
}
