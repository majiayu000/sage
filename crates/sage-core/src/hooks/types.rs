//! Hook type definitions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use super::events::HookEvent;

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

/// Shell command hook
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandHook {
    pub command: String,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    pub status_message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
}

impl CommandHook {
    /// Create a new command hook
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            timeout_secs: default_timeout(),
            status_message: None,
            working_dir: None,
            env: HashMap::new(),
        }
    }

    /// Set the timeout in seconds
    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }

    /// Set the status message
    pub fn with_status_message(mut self, message: impl Into<String>) -> Self {
        self.status_message = Some(message.into());
        self
    }

    /// Get the timeout duration
    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_secs)
    }
}

impl fmt::Display for CommandHook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.command)?;
        if let Some(msg) = &self.status_message {
            write!(f, " ({})", msg)?;
        }
        Ok(())
    }
}

impl Default for CommandHook {
    fn default() -> Self {
        Self {
            command: String::new(),
            timeout_secs: default_timeout(),
            status_message: None,
            working_dir: None,
            env: HashMap::new(),
        }
    }
}

/// LLM prompt hook
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptHook {
    pub prompt: String, // Can use $ARGUMENTS placeholder
    pub model: Option<String>,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
}

impl PromptHook {
    /// Create a new prompt hook
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            model: None,
            timeout_secs: default_timeout(),
            system: None,
        }
    }

    /// Set the model to use
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the timeout in seconds
    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }

    /// Get the timeout duration
    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_secs)
    }

    /// Replace placeholders in the prompt
    pub fn render(&self, arguments: &str) -> String {
        self.prompt.replace("$ARGUMENTS", arguments)
    }
}

impl fmt::Display for PromptHook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(model) = &self.model {
            write!(f, "{} (model: {})", self.prompt, model)
        } else {
            write!(f, "{}", self.prompt)
        }
    }
}

impl Default for PromptHook {
    fn default() -> Self {
        Self {
            prompt: String::new(),
            model: None,
            timeout_secs: default_timeout(),
            system: None,
        }
    }
}

/// Rust callback hook
pub struct CallbackHook {
    pub callback: Arc<dyn Fn(HookInput) -> HookOutput + Send + Sync>,
    pub timeout: Duration,
}

impl CallbackHook {
    /// Create a new callback hook
    pub fn new<F>(callback: F) -> Self
    where
        F: Fn(HookInput) -> HookOutput + Send + Sync + 'static,
    {
        Self {
            callback: Arc::new(callback),
            timeout: Duration::from_secs(default_timeout()),
        }
    }

    /// Set the timeout duration
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

impl Clone for CallbackHook {
    fn clone(&self) -> Self {
        Self {
            callback: Arc::clone(&self.callback),
            timeout: self.timeout,
        }
    }
}

impl fmt::Debug for CallbackHook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CallbackHook")
            .field("timeout", &self.timeout)
            .finish()
    }
}

impl Default for CallbackHook {
    fn default() -> Self {
        Self::new(|_| HookOutput::default())
    }
}

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

fn default_enabled() -> bool {
    true
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

/// Output from a hook
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HookOutput {
    #[serde(default = "default_continue")]
    pub should_continue: bool,
    pub modified_input: Option<serde_json::Value>,
    pub permission_decision: Option<PermissionDecision>,
    #[serde(default)]
    pub additional_context: Vec<String>,
    pub reason: Option<String>,
    pub system_message: Option<String>,
    #[serde(default)]
    pub data: HashMap<String, serde_json::Value>,
}

fn default_continue() -> bool {
    true
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
    pub fn with_permission(mut self, decision: PermissionDecision) -> Self {
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

/// Permission decision from hook
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionDecision {
    Allow,
    Deny,
    Ask,
}

impl fmt::Display for PermissionDecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PermissionDecision::Allow => write!(f, "Allow"),
            PermissionDecision::Deny => write!(f, "Deny"),
            PermissionDecision::Ask => write!(f, "Ask"),
        }
    }
}

impl Default for PermissionDecision {
    fn default() -> Self {
        PermissionDecision::Ask
    }
}

fn default_timeout() -> u64 {
    60
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_hook_new() {
        let hook = CommandHook::new("echo test");
        assert_eq!(hook.command, "echo test");
        assert_eq!(hook.timeout_secs, 60);
        assert_eq!(hook.status_message, None);
    }

    #[test]
    fn test_command_hook_builder() {
        let hook = CommandHook::new("echo test")
            .with_timeout(30)
            .with_status_message("Running test");

        assert_eq!(hook.command, "echo test");
        assert_eq!(hook.timeout_secs, 30);
        assert_eq!(hook.status_message, Some("Running test".to_string()));
        assert_eq!(hook.timeout(), Duration::from_secs(30));
    }

    #[test]
    fn test_command_hook_display() {
        let hook = CommandHook::new("echo test");
        assert_eq!(format!("{}", hook), "echo test");

        let hook = hook.with_status_message("Running");
        assert_eq!(format!("{}", hook), "echo test (Running)");
    }

    #[test]
    fn test_command_hook_default() {
        let hook = CommandHook::default();
        assert_eq!(hook.command, "");
        assert_eq!(hook.timeout_secs, 60);
        assert_eq!(hook.status_message, None);
    }

    #[test]
    fn test_command_hook_serialization() {
        let hook = CommandHook::new("echo test").with_timeout(30);
        let json = serde_json::to_string(&hook).unwrap();
        let deserialized: CommandHook = serde_json::from_str(&json).unwrap();
        assert_eq!(hook, deserialized);
    }

    #[test]
    fn test_prompt_hook_new() {
        let hook = PromptHook::new("Test prompt");
        assert_eq!(hook.prompt, "Test prompt");
        assert_eq!(hook.model, None);
        assert_eq!(hook.timeout_secs, 60);
    }

    #[test]
    fn test_prompt_hook_builder() {
        let hook = PromptHook::new("Test prompt")
            .with_model("gpt-4")
            .with_timeout(120);

        assert_eq!(hook.prompt, "Test prompt");
        assert_eq!(hook.model, Some("gpt-4".to_string()));
        assert_eq!(hook.timeout_secs, 120);
        assert_eq!(hook.timeout(), Duration::from_secs(120));
    }

    #[test]
    fn test_prompt_hook_render() {
        let hook = PromptHook::new("Process $ARGUMENTS");
        let rendered = hook.render("test data");
        assert_eq!(rendered, "Process test data");
    }

    #[test]
    fn test_prompt_hook_display() {
        let hook = PromptHook::new("Test prompt");
        assert_eq!(format!("{}", hook), "Test prompt");

        let hook = hook.with_model("gpt-4");
        assert_eq!(format!("{}", hook), "Test prompt (model: gpt-4)");
    }

    #[test]
    fn test_prompt_hook_default() {
        let hook = PromptHook::default();
        assert_eq!(hook.prompt, "");
        assert_eq!(hook.model, None);
        assert_eq!(hook.timeout_secs, 60);
    }

    #[test]
    fn test_prompt_hook_serialization() {
        let hook = PromptHook::new("Test").with_model("gpt-4");
        let json = serde_json::to_string(&hook).unwrap();
        let deserialized: PromptHook = serde_json::from_str(&json).unwrap();
        assert_eq!(hook, deserialized);
    }

    #[test]
    fn test_callback_hook_new() {
        let hook = CallbackHook::new(|_| HookOutput::allow());
        assert_eq!(hook.timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_callback_hook_with_timeout() {
        let hook = CallbackHook::new(|_| HookOutput::allow()).with_timeout(Duration::from_secs(30));
        assert_eq!(hook.timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_callback_hook_execute() {
        let hook = CallbackHook::new(|input| {
            HookOutput::allow().with_reason(format!("Processed {}", input.event))
        });

        let input = HookInput::new(HookEvent::PreToolUse, "test-session");
        let output = (hook.callback)(input);
        assert!(output.should_continue);
        assert_eq!(output.reason, Some("Processed PreToolUse".to_string()));
    }

    #[test]
    fn test_callback_hook_clone() {
        let hook = CallbackHook::new(|_| HookOutput::allow());
        let cloned = hook.clone();
        assert_eq!(hook.timeout, cloned.timeout);
    }

    #[test]
    fn test_callback_hook_default() {
        let hook = CallbackHook::default();
        let input = HookInput::default();
        let output = (hook.callback)(input);
        assert_eq!(output, HookOutput::default());
    }

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
        let impl1 = HookImplementation::Command(CommandHook::new("echo test"));
        assert_eq!(format!("{}", impl1), "Command: echo test");

        let impl2 = HookImplementation::Prompt(PromptHook::new("Test prompt"));
        assert_eq!(format!("{}", impl2), "Prompt: Test prompt");
    }

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
            .with_permission(PermissionDecision::Allow)
            .with_context("Additional info")
            .with_system_message("System msg")
            .with_reason("Test reason");

        assert!(output.should_continue);
        assert_eq!(output.permission_decision, Some(PermissionDecision::Allow));
        assert_eq!(output.additional_context.len(), 1);
        assert_eq!(output.system_message, Some("System msg".to_string()));
        assert_eq!(output.reason, Some("Test reason".to_string()));
    }

    #[test]
    fn test_hook_output_display() {
        let output = HookOutput::allow()
            .with_permission(PermissionDecision::Allow)
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

    #[test]
    fn test_permission_decision_display() {
        assert_eq!(format!("{}", PermissionDecision::Allow), "Allow");
        assert_eq!(format!("{}", PermissionDecision::Deny), "Deny");
        assert_eq!(format!("{}", PermissionDecision::Ask), "Ask");
    }

    #[test]
    fn test_permission_decision_default() {
        assert_eq!(PermissionDecision::default(), PermissionDecision::Ask);
    }

    #[test]
    fn test_permission_decision_serialization() {
        let decision = PermissionDecision::Allow;
        let json = serde_json::to_string(&decision).unwrap();
        let deserialized: PermissionDecision = serde_json::from_str(&json).unwrap();
        assert_eq!(decision, deserialized);
    }

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
    fn test_hook_variant_display() {
        let variant = HookVariant::Command(CommandHook::new("echo test"));
        let display = format!("{}", variant);
        assert!(display.contains("Command"));
        assert!(display.contains("echo test"));

        let variant = HookVariant::Prompt(PromptHook::new("Test prompt"));
        let display = format!("{}", variant);
        assert!(display.contains("Prompt"));
        assert!(display.contains("Test prompt"));

        let variant = HookVariant::Callback(CallbackHook::default());
        assert_eq!(format!("{}", variant), "Callback");
    }

    #[test]
    fn test_default_timeout() {
        assert_eq!(default_timeout(), 60);
    }
}
