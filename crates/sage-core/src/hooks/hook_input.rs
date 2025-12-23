//! Hook input types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

use super::events::HookEvent;

/// Input to a hook
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HookInput {
    pub event: HookEvent,
    pub session_id: String,
    pub cwd: PathBuf,
    pub tool_name: Option<String>,
    pub tool_input: Option<serde_json::Value>,
    pub tool_result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub agent_type: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl HookInput {
    /// Create a new hook input
    pub fn new(event: HookEvent, session_id: impl Into<String>) -> Self {
        Self {
            event,
            session_id: session_id.into(),
            cwd: PathBuf::from("."),
            tool_name: None,
            tool_input: None,
            tool_result: None,
            error: None,
            agent_type: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the current working directory
    pub fn with_cwd(mut self, cwd: PathBuf) -> Self {
        self.cwd = cwd;
        self
    }

    /// Set the tool name
    pub fn with_tool_name(mut self, tool_name: impl Into<String>) -> Self {
        self.tool_name = Some(tool_name.into());
        self
    }

    /// Set the tool input
    pub fn with_tool_input(mut self, tool_input: serde_json::Value) -> Self {
        self.tool_input = Some(tool_input);
        self
    }

    /// Set the tool result
    pub fn with_tool_result(mut self, tool_result: serde_json::Value) -> Self {
        self.tool_result = Some(tool_result);
        self
    }

    /// Set the error
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }

    /// Set the agent type
    pub fn with_agent_type(mut self, agent_type: impl Into<String>) -> Self {
        self.agent_type = Some(agent_type.into());
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

impl Default for HookInput {
    fn default() -> Self {
        Self {
            event: HookEvent::default(),
            session_id: String::new(),
            cwd: PathBuf::from("."),
            tool_name: None,
            tool_input: None,
            tool_result: None,
            error: None,
            agent_type: None,
            metadata: HashMap::new(),
        }
    }
}

impl fmt::Display for HookInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Event: {}, Session: {}, Tool: {:?}",
            self.event, self.session_id, self.tool_name
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_input_new() {
        let input = HookInput::new(HookEvent::PreToolUse, "test-session");
        assert_eq!(input.event, HookEvent::PreToolUse);
        assert_eq!(input.session_id, "test-session");
        assert_eq!(input.tool_name, None);
    }

    #[test]
    fn test_hook_input_builder() {
        let input = HookInput::new(HookEvent::PreToolUse, "test-session")
            .with_tool_name("bash")
            .with_tool_input(serde_json::json!({"command": "ls"}))
            .with_error("Test error")
            .with_agent_type("main")
            .with_metadata("key", serde_json::json!("value"));

        assert_eq!(input.tool_name, Some("bash".to_string()));
        assert!(input.tool_input.is_some());
        assert_eq!(input.error, Some("Test error".to_string()));
        assert_eq!(input.agent_type, Some("main".to_string()));
        assert_eq!(input.metadata.len(), 1);
    }

    #[test]
    fn test_hook_input_display() {
        let input = HookInput::new(HookEvent::PreToolUse, "test-session").with_tool_name("bash");
        let display = format!("{}", input);
        assert!(display.contains("PreToolUse"));
        assert!(display.contains("test-session"));
        assert!(display.contains("bash"));
    }

    #[test]
    fn test_hook_input_default() {
        let input = HookInput::default();
        assert_eq!(input.event, HookEvent::default());
        assert_eq!(input.session_id, "");
        assert_eq!(input.metadata.len(), 0);
    }

    #[test]
    fn test_hook_input_serialization() {
        let input = HookInput::new(HookEvent::PreToolUse, "test-session").with_tool_name("bash");
        let json = serde_json::to_string(&input).unwrap();
        let deserialized: HookInput = serde_json::from_str(&json).unwrap();
        assert_eq!(input, deserialized);
    }
}
