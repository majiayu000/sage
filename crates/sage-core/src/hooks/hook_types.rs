//! Core hook type definitions

use serde::{Deserialize, Serialize};
use std::fmt;

use super::callback_hook::CallbackHook;
use super::command_hook::CommandHook;
use super::prompt_hook::PromptHook;

/// Hook trigger event type
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookType {
    /// Before agent starts
    PreStart,
    /// After agent completes
    PostComplete,
    /// Before tool execution
    PreToolExecution,
    /// After tool execution
    PostToolExecution,
    /// Before LLM call
    PreLlmCall,
    /// After LLM call
    PostLlmCall,
    /// On error
    OnError,
    /// Custom hook type
    Custom(String),
}

impl HookType {
    pub fn as_str(&self) -> &str {
        match self {
            HookType::PreStart => "pre_start",
            HookType::PostComplete => "post_complete",
            HookType::PreToolExecution => "pre_tool_execution",
            HookType::PostToolExecution => "post_tool_execution",
            HookType::PreLlmCall => "pre_llm_call",
            HookType::PostLlmCall => "post_llm_call",
            HookType::OnError => "on_error",
            HookType::Custom(name) => name,
        }
    }
}

impl fmt::Display for HookType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Hook variant types (Command, Prompt, or Callback)
#[derive(Debug, Clone)]
pub enum HookVariant {
    Command(CommandHook),
    Prompt(PromptHook),
    Callback(CallbackHook),
}

impl fmt::Display for HookVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HookVariant::Command(cmd) => write!(f, "Command({})", cmd),
            HookVariant::Prompt(prompt) => write!(f, "Prompt({})", prompt),
            HookVariant::Callback(_) => write!(f, "Callback"),
        }
    }
}

/// Hook implementation variants
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HookImplementation {
    /// Execute a shell command
    Command(CommandHook),
    /// Execute a prompt with LLM
    Prompt(PromptHook),
}

impl fmt::Display for HookImplementation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HookImplementation::Command(cmd) => write!(f, "Command: {}", cmd),
            HookImplementation::Prompt(prompt) => write!(f, "Prompt: {}", prompt),
        }
    }
}

/// Permission decision from hook
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HookPermissionDecision {
    Allow,
    Deny,
    Ask,
}

impl fmt::Display for HookPermissionDecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HookPermissionDecision::Allow => write!(f, "Allow"),
            HookPermissionDecision::Deny => write!(f, "Deny"),
            HookPermissionDecision::Ask => write!(f, "Ask"),
        }
    }
}

impl Default for HookPermissionDecision {
    fn default() -> Self {
        HookPermissionDecision::Ask
    }
}

/// Default timeout in seconds for hooks
pub(crate) fn default_timeout() -> u64 {
    60
}

/// Default enabled state for hooks
pub(crate) fn default_enabled() -> bool {
    true
}

/// Default continue state for hook output
pub(crate) fn default_continue() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_type_display() {
        assert_eq!(format!("{}", HookType::PreStart), "pre_start");
        assert_eq!(format!("{}", HookType::PostComplete), "post_complete");
        assert_eq!(
            format!("{}", HookType::PreToolExecution),
            "pre_tool_execution"
        );
    }

    #[test]
    fn test_hook_type_as_str() {
        assert_eq!(HookType::PreStart.as_str(), "pre_start");
        assert_eq!(HookType::PostComplete.as_str(), "post_complete");
        assert_eq!(HookType::PreToolExecution.as_str(), "pre_tool_execution");
    }

    #[test]
    fn test_hook_type_clone() {
        let hook_type = HookType::PreStart.clone();
        assert_eq!(hook_type, HookType::PreStart);
    }

    #[test]
    fn test_permission_decision_display() {
        assert_eq!(format!("{}", HookPermissionDecision::Allow), "Allow");
        assert_eq!(format!("{}", HookPermissionDecision::Deny), "Deny");
        assert_eq!(format!("{}", HookPermissionDecision::Ask), "Ask");
    }

    #[test]
    fn test_permission_decision_default() {
        assert_eq!(HookPermissionDecision::default(), HookPermissionDecision::Ask);
    }

    #[test]
    fn test_permission_decision_serialization() {
        let decision = HookPermissionDecision::Allow;
        let json = serde_json::to_string(&decision).unwrap();
        let deserialized: HookPermissionDecision = serde_json::from_str(&json).unwrap();
        assert_eq!(decision, deserialized);
    }

    #[test]
    fn test_default_timeout() {
        assert_eq!(default_timeout(), 60);
    }
}
