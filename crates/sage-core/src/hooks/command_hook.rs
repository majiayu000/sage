//! Command hook implementation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

use super::hook_types::default_timeout;

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
}
