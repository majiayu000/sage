//! Configuration for spawning a sub-agent
//!
//! Provides configuration options for sub-agent execution, including
//! working directory inheritance and tool access control.

use super::{
    AgentType, ForkContextMessage, ForkContextPolicy, Thoroughness, ToolAccessControl,
    WorkingDirectoryConfig,
};
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
/// use sage_core::agent::subagent::{SubAgentConfig, AgentType, WorkingDirectoryConfig, ToolAccessControl};
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
    /// Optional reasoning override
    #[serde(default)]
    pub reasoning_override: Option<String>,
    /// Optional permission/profile override
    #[serde(default)]
    pub profile_override: Option<String>,
    /// Thoroughness level for exploration tasks
    #[serde(default)]
    pub thoroughness: Thoroughness,
    /// Optional role config file path. Relative paths resolve under `role_root`.
    #[serde(default)]
    pub role_path: Option<PathBuf>,
    /// Optional role root. Defaults to `<parent_cwd>/.sage/agents`.
    #[serde(default)]
    pub role_root: Option<PathBuf>,
    /// Parent context fork policy
    #[serde(default)]
    pub fork_context: ForkContextPolicy,
    /// Whether fork_context was set explicitly by the spawn request
    #[serde(skip)]
    pub fork_context_explicit: bool,
    /// Parent messages available for fork_context selection
    #[serde(default)]
    pub parent_context: Vec<ForkContextMessage>,
    /// Whether parent context was explicitly available, even if empty
    #[serde(default)]
    pub parent_context_available: bool,
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
    /// Additional context for the task
    #[serde(default)]
    pub context: Option<String>,
    /// Override maximum execution steps
    #[serde(default)]
    pub max_steps: Option<usize>,
    /// Override temperature for LLM calls
    #[serde(default)]
    pub temperature: Option<f64>,
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
            reasoning_override: None,
            profile_override: None,
            thoroughness: Thoroughness::default(),
            role_path: None,
            role_root: None,
            fork_context: ForkContextPolicy::default(),
            fork_context_explicit: false,
            parent_context: Vec::new(),
            parent_context_available: false,
            working_directory: WorkingDirectoryConfig::default(),
            tool_access: default_tool_access(),
            context: None,
            max_steps: None,
            temperature: None,
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

    /// Set reasoning override
    pub fn with_reasoning(mut self, reasoning: String) -> Self {
        self.reasoning_override = Some(reasoning);
        self
    }

    /// Set profile override
    pub fn with_profile(mut self, profile: String) -> Self {
        self.profile_override = Some(profile);
        self
    }

    /// Set thoroughness level for exploration
    pub fn with_thoroughness(mut self, thoroughness: Thoroughness) -> Self {
        self.thoroughness = thoroughness;
        self
    }

    /// Set custom role config path
    pub fn with_role_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.role_path = Some(path.into());
        self.agent_type = AgentType::Custom;
        self
    }

    /// Set custom role config root
    pub fn with_role_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.role_root = Some(root.into());
        self
    }

    /// Set parent context fork policy
    pub fn with_fork_context(mut self, policy: ForkContextPolicy) -> Self {
        self.fork_context = policy;
        self.fork_context_explicit = true;
        self
    }

    /// Set parent context messages available for fork policies
    pub fn with_forked_parent_context(mut self, context: Vec<ForkContextMessage>) -> Self {
        self.parent_context = context;
        self.parent_context_available = true;
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

    /// Set additional context for the task
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Set maximum execution steps
    pub fn with_max_steps(mut self, max_steps: usize) -> Self {
        self.max_steps = Some(max_steps);
        self
    }

    /// Set temperature for LLM calls
    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Get the task description (alias for `prompt`)
    pub fn task(&self) -> &str {
        &self.prompt
    }

    /// Set parent context for inheritance resolution
    ///
    /// This should be called by the parent agent when spawning the sub-agent.
    pub fn with_parent_context(mut self, parent_cwd: PathBuf, parent_tools: Vec<String>) -> Self {
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
        self.tool_access
            .resolve_allows_tool(tool_name, self.parent_tools.as_deref())
    }
}
