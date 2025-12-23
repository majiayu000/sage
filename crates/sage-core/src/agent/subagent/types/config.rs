//! Configuration for spawning a sub-agent

use super::{AgentType, Thoroughness};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Configuration for spawning a sub-agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentConfig {
    /// Type of agent to spawn
    pub agent_type: AgentType,
    /// Initial prompt/task for the agent
    pub prompt: String,
    /// Optional resume ID for continuing previous execution
    #[serde(default)]
    pub resume_id: Option<String>,
    /// Whether to run in background
    #[serde(default)]
    pub run_in_background: bool,
    /// Optional model override
    #[serde(default)]
    pub model_override: Option<String>,
    /// Thoroughness level for exploration tasks
    #[serde(default)]
    pub thoroughness: Thoroughness,
}

impl fmt::Display for SubAgentConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SubAgentConfig(type: {}, background: {}, prompt_len: {})",
            self.agent_type,
            self.run_in_background,
            self.prompt.len()
        )
    }
}

impl SubAgentConfig {
    /// Create a new sub-agent configuration
    pub fn new(agent_type: AgentType, prompt: impl Into<String>) -> Self {
        Self {
            agent_type,
            prompt: prompt.into(),
            resume_id: None,
            run_in_background: false,
            model_override: None,
            thoroughness: Thoroughness::default(),
        }
    }

    /// Set resume ID for continuing execution
    pub fn with_resume_id(mut self, resume_id: String) -> Self {
        self.resume_id = Some(resume_id);
        self
    }

    /// Set to run in background
    pub fn with_background(mut self, background: bool) -> Self {
        self.run_in_background = background;
        self
    }

    /// Set model override
    pub fn with_model(mut self, model: String) -> Self {
        self.model_override = Some(model);
        self
    }

    /// Set thoroughness level for exploration
    pub fn with_thoroughness(mut self, thoroughness: Thoroughness) -> Self {
        self.thoroughness = thoroughness;
        self
    }
}
