//! Configuration for spawning a sub-agent
//!
//! Provides configuration options for sub-agent execution, including
//! working directory inheritance and tool access control.

use super::{AgentType, Thoroughness, ToolAccessControl, WorkingDirectoryConfig};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

/// Configuration for spawning a sub-agent
///
/// This struct contains all the configuration needed to spawn a sub-agent,
/// including task description, working directory, tool access, and model settings.
///
/// # Example
///
/// ```rust
/// use sage_core::agent::subagent::{SubAgentConfig, AgentType, WorkingDirectoryConfig};
/// use std::path::PathBuf;
///
/// let config = SubAgentConfig::new(AgentType::Explore, "Analyze the codebase structure")
///     .with_working_directory(WorkingDirectoryConfig::Inherited)
///     .with_tool_access(ToolAccessControl::Inherited);
/// ```
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
    /// Working directory configuration
    ///
    /// Controls how the sub-agent determines its working directory.
    /// Defaults to `Inherited` which uses the parent agent's working directory.
    #[serde(default)]
    pub working_directory: WorkingDirectoryConfig,
    /// Tool access control
    ///
    /// Controls which tools the sub-agent can use.
    /// Defaults to `Inherited` which uses the parent agent's tools.
    #[serde(default = "default_tool_access")]
    pub tool_access: ToolAccessControl,
    /// Parent's working directory (set at runtime)
    ///
    /// This is populated by the parent agent when spawning the sub-agent.
    /// Used to resolve `WorkingDirectoryConfig::Inherited`.
    #[serde(skip)]
    pub parent_cwd: Option<PathBuf>,
    /// Parent's available tools (set at runtime)
    ///
    /// This is populated by the parent agent when spawning the sub-agent.
    /// Used to resolve `ToolAccessControl::Inherited`.
    #[serde(skip)]
    pub parent_tools: Option<Vec<String>>,
}

fn default_tool_access() -> ToolAccessControl {
    ToolAccessControl::Inherited
}

impl fmt::Display for SubAgentConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SubAgentConfig(type: {}, background: {}, cwd: {}, tools: {}, prompt_len: {})",
            self.agent_type,
            self.run_in_background,
            self.working_directory,
            self.tool_access,
            self.prompt.len()
        )
    }
}

impl SubAgentConfig {
    /// Create a new sub-agent configuration
    ///
    /// By default, the sub-agent will:
    /// - Inherit the working directory from the parent
    /// - Inherit tool access from the parent
    pub fn new(agent_type: AgentType, prompt: impl Into<String>) -> Self {
        Self {
            agent_type,
            prompt: prompt.into(),
            resume_id: None,
            run_in_background: false,
            model_override: None,
            thoroughness: Thoroughness::default(),
            working_directory: WorkingDirectoryConfig::default(),
            tool_access: default_tool_access(),
            parent_cwd: None,
            parent_tools: None,
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

    /// Set working directory configuration
    pub fn with_working_directory(mut self, config: WorkingDirectoryConfig) -> Self {
        self.working_directory = config;
        self
    }

    /// Set tool access control
    pub fn with_tool_access(mut self, access: ToolAccessControl) -> Self {
        self.tool_access = access;
        self
    }

    /// Set parent context for inheritance resolution
    ///
    /// This should be called by the parent agent when spawning the sub-agent.
    pub fn with_parent_context(
        mut self,
        parent_cwd: PathBuf,
        parent_tools: Vec<String>,
    ) -> Self {
        self.parent_cwd = Some(parent_cwd);
        self.parent_tools = Some(parent_tools);
        self
    }

    /// Set only the parent working directory
    pub fn with_parent_cwd(mut self, cwd: PathBuf) -> Self {
        self.parent_cwd = Some(cwd);
        self
    }

    /// Set only the parent tools
    pub fn with_parent_tools(mut self, tools: Vec<String>) -> Self {
        self.parent_tools = Some(tools);
        self
    }

    /// Resolve the effective working directory
    ///
    /// Returns the actual working directory path based on configuration
    /// and parent context.
    pub fn resolve_working_directory(&self) -> std::io::Result<PathBuf> {
        self.working_directory.resolve(self.parent_cwd.as_ref())
    }

    /// Check if a tool is allowed for this sub-agent
    ///
    /// Takes into account inheritance from parent agent.
    pub fn allows_tool(&self, tool_name: &str) -> bool {
        self.tool_access.resolve_allows_tool(
            tool_name,
            self.parent_tools.as_ref().map(|v| v.as_slice()),
        )
    }
}
