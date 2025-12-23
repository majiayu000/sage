//! Prompt hook implementation

use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;

use super::hook_types::default_timeout;

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
