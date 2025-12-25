//! Hook definitions and settings

use serde::{Deserialize, Serialize};

/// Hook settings
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HooksSettings {
    /// Pre-tool-use hooks
    #[serde(default)]
    pub pre_tool_use: Vec<HookDefinition>,

    /// Post-tool-use hooks
    #[serde(default)]
    pub post_tool_use: Vec<HookDefinition>,

    /// User prompt submit hooks
    #[serde(default)]
    pub user_prompt_submit: Vec<HookDefinition>,

    /// Session start hooks
    #[serde(default)]
    pub session_start: Vec<HookDefinition>,

    /// Session end hooks
    #[serde(default)]
    pub session_end: Vec<HookDefinition>,
}

impl HooksSettings {
    /// Merge another hooks settings
    pub fn merge(&mut self, other: HooksSettings) {
        self.pre_tool_use.extend(other.pre_tool_use);
        self.post_tool_use.extend(other.post_tool_use);
        self.user_prompt_submit.extend(other.user_prompt_submit);
        self.session_start.extend(other.session_start);
        self.session_end.extend(other.session_end);
    }
}

/// Hook definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookDefinition {
    /// Hook type
    #[serde(rename = "type")]
    pub hook_type: HookDefinitionType,

    /// Command to run (for command hooks)
    #[serde(default)]
    pub command: Option<String>,

    /// Prompt to use (for prompt hooks)
    #[serde(default)]
    pub prompt: Option<String>,

    /// Pattern to match (optional, for filtering)
    #[serde(default)]
    pub pattern: Option<String>,

    /// Timeout in milliseconds
    #[serde(default)]
    pub timeout_ms: Option<u64>,

    /// Status message to display
    #[serde(default)]
    pub status_message: Option<String>,
}

/// Hook definition type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HookDefinitionType {
    /// Shell command
    Command,
    /// LLM prompt
    Prompt,
}
