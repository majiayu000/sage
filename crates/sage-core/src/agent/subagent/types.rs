//! Core types for sub-agent orchestration system
//!
//! This module defines the fundamental types used in the sub-agent system,
//! including agent definitions, execution states, and result types.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Instant;
use tokio_util::sync::CancellationToken;

/// Agent type enumeration defining different agent specializations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    /// General purpose agent with all tools available
    GeneralPurpose,
    /// Fast exploration agent with read-only tools (Glob, Grep, Read)
    Explore,
    /// Architecture planning agent with all tools
    Plan,
    /// Custom agent type
    Custom,
}

/// Thoroughness level for exploration tasks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Thoroughness {
    /// Basic search - fast but may miss edge cases
    Quick,
    /// Balanced search - good coverage with reasonable speed
    #[default]
    Medium,
    /// Comprehensive analysis - thorough but slower
    VeryThorough,
}

impl fmt::Display for Thoroughness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Thoroughness {
    /// Get the string identifier for this thoroughness level
    pub fn as_str(&self) -> &str {
        match self {
            Thoroughness::Quick => "quick",
            Thoroughness::Medium => "medium",
            Thoroughness::VeryThorough => "very_thorough",
        }
    }

    /// Get suggested max steps for this thoroughness level
    pub fn suggested_max_steps(&self) -> usize {
        match self {
            Thoroughness::Quick => 5,
            Thoroughness::Medium => 15,
            Thoroughness::VeryThorough => 30,
        }
    }

    /// Get description for prompting
    pub fn description(&self) -> &str {
        match self {
            Thoroughness::Quick => "Perform a quick search. Focus on the most obvious locations and patterns. Stop early if you find good matches.",
            Thoroughness::Medium => "Perform a moderate search. Check multiple locations and naming conventions. Balance thoroughness with speed.",
            Thoroughness::VeryThorough => "Perform a comprehensive search. Check all possible locations, naming patterns, and variations. Be thorough even if it takes longer.",
        }
    }
}

impl fmt::Display for AgentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Default for AgentType {
    fn default() -> Self {
        AgentType::GeneralPurpose
    }
}

impl AgentType {
    /// Get the string identifier for this agent type
    pub fn as_str(&self) -> &str {
        match self {
            AgentType::GeneralPurpose => "general_purpose",
            AgentType::Explore => "explore",
            AgentType::Plan => "plan",
            AgentType::Custom => "custom",
        }
    }
}

/// Tool access control for agents
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolAccessControl {
    /// Agent has access to all available tools
    All,
    /// Agent has access only to specific tools
    Specific(Vec<String>),
    /// No tool access
    None,
}

impl fmt::Display for ToolAccessControl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolAccessControl::All => write!(f, "all_tools"),
            ToolAccessControl::Specific(tools) => {
                write!(f, "tools[{}]", tools.join(", "))
            }
            ToolAccessControl::None => write!(f, "no_tools"),
        }
    }
}

impl Default for ToolAccessControl {
    fn default() -> Self {
        ToolAccessControl::All
    }
}

impl ToolAccessControl {
    /// Check if a tool is allowed
    pub fn allows_tool(&self, tool_name: &str) -> bool {
        match self {
            ToolAccessControl::All => true,
            ToolAccessControl::Specific(tools) => tools.iter().any(|t| t == tool_name),
            ToolAccessControl::None => false,
        }
    }

    /// Get the list of allowed tools (if specific)
    pub fn allowed_tools(&self) -> Option<&[String]> {
        match self {
            ToolAccessControl::Specific(tools) => Some(tools),
            _ => None,
        }
    }
}

/// Agent definition containing configuration and metadata
#[derive(Debug, Clone)]
pub struct AgentDefinition {
    /// Type of agent
    pub agent_type: AgentType,
    /// Human-readable name
    pub name: String,
    /// Description of agent's purpose
    pub description: String,
    /// Tools available to this agent
    pub available_tools: ToolAccessControl,
    /// Optional model override
    pub model: Option<String>,
    /// System prompt for this agent
    pub system_prompt: String,
}

impl fmt::Display for AgentDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AgentDefinition({}: {}, tools: {})",
            self.name, self.agent_type, self.available_tools
        )
    }
}

impl AgentDefinition {
    /// Create a new custom agent definition
    pub fn custom(
        name: String,
        description: String,
        available_tools: ToolAccessControl,
        system_prompt: String,
    ) -> Self {
        Self {
            agent_type: AgentType::Custom,
            name,
            description,
            available_tools,
            model: None,
            system_prompt,
        }
    }

    /// Get the agent's identifier (used for registry lookups)
    pub fn id(&self) -> String {
        self.agent_type.as_str().to_string()
    }

    /// Check if this agent can use a specific tool
    pub fn can_use_tool(&self, tool_name: &str) -> bool {
        self.available_tools.allows_tool(tool_name)
    }
}

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

/// Progress information for running agent
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentProgress {
    /// Recent activity descriptions
    pub recent_activities: Vec<String>,
    /// Total tokens consumed so far
    pub token_count: u64,
    /// Number of tools used
    pub tool_use_count: u32,
    /// Current execution step
    pub current_step: u32,
}

impl fmt::Display for AgentProgress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Progress(step: {}, tools: {}, tokens: {})",
            self.current_step, self.tool_use_count, self.token_count
        )
    }
}

impl AgentProgress {
    /// Create new progress
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a new activity to the progress tracker
    pub fn add_activity(&mut self, activity: String) {
        self.recent_activities.push(activity);
        // Keep only the last 10 activities
        if self.recent_activities.len() > 10 {
            self.recent_activities.remove(0);
        }
    }

    /// Increment tool use counter
    pub fn increment_tool_use(&mut self) {
        self.tool_use_count += 1;
    }

    /// Add tokens to the counter
    pub fn add_tokens(&mut self, tokens: u64) {
        self.token_count += tokens;
    }

    /// Advance to next step
    pub fn next_step(&mut self) {
        self.current_step += 1;
    }
}

/// Execution metadata collected during agent run
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionMetadata {
    /// Total tokens consumed
    pub total_tokens: u64,
    /// Total number of tool uses
    pub total_tool_uses: u32,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// List of tools used during execution
    pub tools_used: Vec<String>,
}

impl fmt::Display for ExecutionMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Metadata(tokens: {}, tools: {}, time: {}ms)",
            self.total_tokens, self.total_tool_uses, self.execution_time_ms
        )
    }
}

impl ExecutionMetadata {
    /// Create new metadata from agent progress
    pub fn from_progress(progress: &AgentProgress, elapsed_ms: u64) -> Self {
        Self {
            total_tokens: progress.token_count,
            total_tool_uses: progress.tool_use_count,
            execution_time_ms: elapsed_ms,
            tools_used: Vec::new(),
        }
    }

    /// Add a tool to the tools_used list (deduplicates)
    pub fn add_tool(&mut self, tool_name: String) {
        if !self.tools_used.contains(&tool_name) {
            self.tools_used.push(tool_name);
        }
    }
}

/// Result from sub-agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentResult {
    /// Unique agent identifier
    pub agent_id: String,
    /// Result content/output
    pub content: String,
    /// Execution metadata
    pub metadata: ExecutionMetadata,
}

impl fmt::Display for SubAgentResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SubAgentResult(id: {}, content_len: {}, {})",
            self.agent_id,
            self.content.len(),
            self.metadata
        )
    }
}

/// Agent status during execution lifecycle
#[derive(Debug, Clone)]
pub enum AgentStatus {
    /// Agent is queued but not yet running
    Pending,
    /// Agent is currently executing with progress information
    Running(AgentProgress),
    /// Agent completed successfully
    Completed(SubAgentResult),
    /// Agent failed with error message
    Failed(String),
    /// Agent was cancelled/killed
    Killed,
}

impl fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentStatus::Pending => write!(f, "Pending"),
            AgentStatus::Running(progress) => write!(f, "Running({})", progress),
            AgentStatus::Completed(result) => write!(f, "Completed({})", result.agent_id),
            AgentStatus::Failed(err) => write!(f, "Failed: {}", err),
            AgentStatus::Killed => write!(f, "Killed"),
        }
    }
}

impl AgentStatus {
    /// Check if agent is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            AgentStatus::Completed(_) | AgentStatus::Failed(_) | AgentStatus::Killed
        )
    }

    /// Check if agent is currently running
    pub fn is_running(&self) -> bool {
        matches!(self, AgentStatus::Running(_))
    }

    /// Get progress if agent is running
    pub fn progress(&self) -> Option<&AgentProgress> {
        match self {
            AgentStatus::Running(progress) => Some(progress),
            _ => None,
        }
    }

    /// Get mutable progress if agent is running
    pub fn progress_mut(&mut self) -> Option<&mut AgentProgress> {
        match self {
            AgentStatus::Running(progress) => Some(progress),
            _ => None,
        }
    }

    /// Get result if agent completed successfully
    pub fn result(&self) -> Option<&SubAgentResult> {
        match self {
            AgentStatus::Completed(result) => Some(result),
            _ => None,
        }
    }
}

/// Running agent state container
pub struct RunningAgent {
    /// Unique agent identifier
    pub id: String,
    /// Type of agent
    pub agent_type: AgentType,
    /// Configuration used to spawn this agent
    pub config: SubAgentConfig,
    /// Current execution status
    pub status: AgentStatus,
    /// Start time of execution
    pub start_time: Instant,
    /// Cancellation token for stopping the agent
    pub cancel_token: CancellationToken,
}

impl fmt::Debug for RunningAgent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RunningAgent")
            .field("id", &self.id)
            .field("agent_type", &self.agent_type)
            .field("config", &self.config)
            .field("status", &self.status)
            .field("elapsed", &self.start_time.elapsed())
            .finish()
    }
}

impl RunningAgent {
    /// Create a new running agent
    pub fn new(id: String, agent_type: AgentType, config: SubAgentConfig) -> Self {
        Self {
            id,
            agent_type,
            config,
            status: AgentStatus::Pending,
            start_time: Instant::now(),
            cancel_token: CancellationToken::new(),
        }
    }

    /// Get elapsed time in milliseconds
    pub fn elapsed_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }

    /// Check if agent is still active
    pub fn is_active(&self) -> bool {
        !self.status.is_terminal()
    }

    /// Request cancellation of the agent
    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }

    /// Check if cancellation was requested
    pub fn is_cancelled(&self) -> bool {
        self.cancel_token.is_cancelled()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_type_display() {
        assert_eq!(AgentType::GeneralPurpose.to_string(), "general_purpose");
        assert_eq!(AgentType::Explore.to_string(), "explore");
        assert_eq!(AgentType::Plan.to_string(), "plan");
        assert_eq!(AgentType::Custom.to_string(), "custom");
    }

    #[test]
    fn test_agent_type_default() {
        assert_eq!(AgentType::default(), AgentType::GeneralPurpose);
    }

    #[test]
    fn test_agent_type_as_str() {
        assert_eq!(AgentType::GeneralPurpose.as_str(), "general_purpose");
        assert_eq!(AgentType::Explore.as_str(), "explore");
        assert_eq!(AgentType::Plan.as_str(), "plan");
        assert_eq!(AgentType::Custom.as_str(), "custom");
    }

    #[test]
    fn test_agent_type_serde() {
        let agent_type = AgentType::Explore;
        let json = serde_json::to_string(&agent_type).unwrap();
        assert_eq!(json, "\"explore\"");

        let deserialized: AgentType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, agent_type);
    }

    #[test]
    fn test_tool_access_control_display() {
        let all = ToolAccessControl::All;
        assert_eq!(all.to_string(), "all_tools");

        let specific = ToolAccessControl::Specific(vec!["Read".to_string(), "Write".to_string()]);
        assert_eq!(specific.to_string(), "tools[Read, Write]");

        let none = ToolAccessControl::None;
        assert_eq!(none.to_string(), "no_tools");
    }

    #[test]
    fn test_tool_access_control_default() {
        assert_eq!(ToolAccessControl::default(), ToolAccessControl::All);
    }

    #[test]
    fn test_tool_access_control_all() {
        let access = ToolAccessControl::All;
        assert!(access.allows_tool("any_tool"));
        assert!(access.allows_tool("another_tool"));
        assert_eq!(access.allowed_tools(), None);
    }

    #[test]
    fn test_tool_access_control_specific() {
        let access = ToolAccessControl::Specific(vec![
            "glob".to_string(),
            "grep".to_string(),
            "read".to_string(),
        ]);
        assert!(access.allows_tool("glob"));
        assert!(access.allows_tool("grep"));
        assert!(access.allows_tool("read"));
        assert!(!access.allows_tool("write"));
        assert!(!access.allows_tool("bash"));

        let tools = access.allowed_tools().unwrap();
        assert_eq!(tools.len(), 3);
    }

    #[test]
    fn test_tool_access_control_none() {
        let access = ToolAccessControl::None;
        assert!(!access.allows_tool("any_tool"));
        assert_eq!(access.allowed_tools(), None);
    }

    #[test]
    fn test_agent_definition_custom() {
        let agent = AgentDefinition::custom(
            "Test Agent".to_string(),
            "A test agent".to_string(),
            ToolAccessControl::All,
            "Test prompt".to_string(),
        );

        assert_eq!(agent.agent_type, AgentType::Custom);
        assert_eq!(agent.name, "Test Agent");
        assert_eq!(agent.description, "A test agent");
        assert_eq!(agent.system_prompt, "Test prompt");
        assert!(agent.can_use_tool("any_tool"));
    }

    #[test]
    fn test_agent_definition_can_use_tool() {
        let agent = AgentDefinition {
            agent_type: AgentType::Explore,
            name: "Explorer".to_string(),
            description: "Fast explorer".to_string(),
            available_tools: ToolAccessControl::Specific(vec!["glob".to_string()]),
            model: Some("haiku".to_string()),
            system_prompt: "Explore!".to_string(),
        };

        assert!(agent.can_use_tool("glob"));
        assert!(!agent.can_use_tool("write"));
    }

    #[test]
    fn test_agent_definition_display() {
        let def = AgentDefinition {
            agent_type: AgentType::Explore,
            name: "Explorer".to_string(),
            description: "Fast exploration agent".to_string(),
            available_tools: ToolAccessControl::Specific(vec!["Read".to_string()]),
            model: Some("gpt-3.5-turbo".to_string()),
            system_prompt: "You are an explorer".to_string(),
        };

        let display = def.to_string();
        assert!(display.contains("Explorer"));
        assert!(display.contains("explore"));
        assert!(display.contains("Read"));
    }

    #[test]
    fn test_subagent_config_new() {
        let config = SubAgentConfig::new(AgentType::Explore, "Find files");
        assert_eq!(config.agent_type, AgentType::Explore);
        assert_eq!(config.prompt, "Find files");
        assert_eq!(config.resume_id, None);
        assert!(!config.run_in_background);
        assert_eq!(config.model_override, None);
    }

    #[test]
    fn test_subagent_config_builder() {
        let config = SubAgentConfig::new(AgentType::Plan, "Design system")
            .with_resume_id("resume-123".to_string())
            .with_background(true)
            .with_model("gpt-4".to_string());

        assert_eq!(config.agent_type, AgentType::Plan);
        assert_eq!(config.prompt, "Design system");
        assert_eq!(config.resume_id, Some("resume-123".to_string()));
        assert!(config.run_in_background);
        assert_eq!(config.model_override, Some("gpt-4".to_string()));
    }

    #[test]
    fn test_agent_progress_add_activity() {
        let mut progress = AgentProgress::default();
        assert_eq!(progress.recent_activities.len(), 0);

        progress.add_activity("Step 1".to_string());
        assert_eq!(progress.recent_activities.len(), 1);

        // Add 11 activities to test the limit
        for i in 2..=12 {
            progress.add_activity(format!("Step {}", i));
        }

        // Should only keep last 10
        assert_eq!(progress.recent_activities.len(), 10);
        assert_eq!(progress.recent_activities[0], "Step 3");
        assert_eq!(progress.recent_activities[9], "Step 12");
    }

    #[test]
    fn test_agent_progress_increment_tool_use() {
        let mut progress = AgentProgress::default();
        assert_eq!(progress.tool_use_count, 0);

        progress.increment_tool_use();
        assert_eq!(progress.tool_use_count, 1);

        progress.increment_tool_use();
        assert_eq!(progress.tool_use_count, 2);
    }

    #[test]
    fn test_agent_progress_add_tokens() {
        let mut progress = AgentProgress::default();
        assert_eq!(progress.token_count, 0);

        progress.add_tokens(100);
        assert_eq!(progress.token_count, 100);

        progress.add_tokens(50);
        assert_eq!(progress.token_count, 150);
    }

    #[test]
    fn test_agent_progress_next_step() {
        let mut progress = AgentProgress::default();
        assert_eq!(progress.current_step, 0);

        progress.next_step();
        assert_eq!(progress.current_step, 1);

        progress.next_step();
        assert_eq!(progress.current_step, 2);
    }

    #[test]
    fn test_agent_progress_serde() {
        let progress = AgentProgress {
            recent_activities: vec!["Reading files".to_string(), "Analyzing code".to_string()],
            token_count: 500,
            tool_use_count: 3,
            current_step: 2,
        };

        let json = serde_json::to_string(&progress).unwrap();
        let deserialized: AgentProgress = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized, progress);
    }

    #[test]
    fn test_execution_metadata_from_progress() {
        let mut progress = AgentProgress::default();
        progress.token_count = 1000;
        progress.tool_use_count = 5;
        progress.current_step = 3;

        let metadata = ExecutionMetadata::from_progress(&progress, 5000);
        assert_eq!(metadata.total_tokens, 1000);
        assert_eq!(metadata.total_tool_uses, 5);
        assert_eq!(metadata.execution_time_ms, 5000);
        assert_eq!(metadata.tools_used.len(), 0);
    }

    #[test]
    fn test_execution_metadata_add_tool() {
        let mut metadata = ExecutionMetadata::default();
        assert_eq!(metadata.tools_used.len(), 0);

        metadata.add_tool("Read".to_string());
        assert_eq!(metadata.tools_used.len(), 1);
        assert_eq!(metadata.tools_used[0], "Read");

        // Adding same tool should not duplicate
        metadata.add_tool("Read".to_string());
        assert_eq!(metadata.tools_used.len(), 1);

        metadata.add_tool("Write".to_string());
        assert_eq!(metadata.tools_used.len(), 2);
    }

    #[test]
    fn test_agent_status_is_terminal() {
        assert!(!AgentStatus::Pending.is_terminal());
        assert!(!AgentStatus::Running(AgentProgress::default()).is_terminal());
        assert!(
            AgentStatus::Completed(SubAgentResult {
                agent_id: "test".to_string(),
                content: "done".to_string(),
                metadata: ExecutionMetadata::default(),
            })
            .is_terminal()
        );
        assert!(AgentStatus::Failed("error".to_string()).is_terminal());
        assert!(AgentStatus::Killed.is_terminal());
    }

    #[test]
    fn test_agent_status_is_running() {
        assert!(!AgentStatus::Pending.is_running());
        assert!(AgentStatus::Running(AgentProgress::default()).is_running());
        assert!(
            !AgentStatus::Completed(SubAgentResult {
                agent_id: "test".to_string(),
                content: "done".to_string(),
                metadata: ExecutionMetadata::default(),
            })
            .is_running()
        );
        assert!(!AgentStatus::Failed("error".to_string()).is_running());
        assert!(!AgentStatus::Killed.is_running());
    }

    #[test]
    fn test_agent_status_progress() {
        let progress = AgentProgress {
            recent_activities: vec!["test".to_string()],
            token_count: 100,
            tool_use_count: 2,
            current_step: 1,
        };

        let status = AgentStatus::Running(progress.clone());
        assert!(status.progress().is_some());
        assert_eq!(status.progress().unwrap().token_count, 100);

        assert!(AgentStatus::Pending.progress().is_none());
        assert!(AgentStatus::Killed.progress().is_none());
    }

    #[test]
    fn test_agent_status_progress_mut() {
        let mut status = AgentStatus::Running(AgentProgress::default());

        if let Some(progress) = status.progress_mut() {
            progress.add_tokens(500);
        }

        assert_eq!(status.progress().unwrap().token_count, 500);
    }

    #[test]
    fn test_agent_status_result() {
        let result = SubAgentResult {
            agent_id: "test-123".to_string(),
            content: "Success!".to_string(),
            metadata: ExecutionMetadata::default(),
        };

        let status = AgentStatus::Completed(result.clone());
        assert!(status.result().is_some());
        assert_eq!(status.result().unwrap().agent_id, "test-123");

        assert!(AgentStatus::Pending.result().is_none());
        assert!(
            AgentStatus::Running(AgentProgress::default())
                .result()
                .is_none()
        );
        assert!(AgentStatus::Failed("error".to_string()).result().is_none());
    }

    #[test]
    fn test_running_agent_new() {
        let config = SubAgentConfig::new(AgentType::Explore, "Test prompt");

        let agent = RunningAgent::new("agent-123".to_string(), AgentType::Explore, config.clone());

        assert_eq!(agent.id, "agent-123");
        assert_eq!(agent.agent_type, AgentType::Explore);
        assert!(matches!(agent.status, AgentStatus::Pending));
        assert!(!agent.cancel_token.is_cancelled());
    }

    #[test]
    fn test_running_agent_elapsed_ms() {
        let config = SubAgentConfig::new(AgentType::GeneralPurpose, "Test");
        let agent = RunningAgent::new("test".to_string(), AgentType::GeneralPurpose, config);

        // Sleep a bit to ensure elapsed time is > 0
        std::thread::sleep(std::time::Duration::from_millis(10));

        assert!(agent.elapsed_ms() >= 10);
    }

    #[test]
    fn test_running_agent_is_active() {
        let config = SubAgentConfig::new(AgentType::GeneralPurpose, "Test");
        let mut agent = RunningAgent::new("test".to_string(), AgentType::GeneralPurpose, config);

        // Initially pending - active
        assert!(agent.is_active());

        // Running - active
        agent.status = AgentStatus::Running(AgentProgress::default());
        assert!(agent.is_active());

        // Completed - not active
        agent.status = AgentStatus::Completed(SubAgentResult {
            agent_id: "test".to_string(),
            content: "done".to_string(),
            metadata: ExecutionMetadata::default(),
        });
        assert!(!agent.is_active());

        // Failed - not active
        agent.status = AgentStatus::Failed("error".to_string());
        assert!(!agent.is_active());

        // Killed - not active
        agent.status = AgentStatus::Killed;
        assert!(!agent.is_active());
    }

    #[test]
    fn test_running_agent_cancel() {
        let config = SubAgentConfig::new(AgentType::GeneralPurpose, "Test");
        let agent = RunningAgent::new("test".to_string(), AgentType::GeneralPurpose, config);

        assert!(!agent.is_cancelled());

        agent.cancel();
        assert!(agent.is_cancelled());
    }

    #[test]
    fn test_subagent_config_serde() {
        let config = SubAgentConfig {
            agent_type: AgentType::Plan,
            prompt: "Design the system".to_string(),
            resume_id: Some("resume-123".to_string()),
            run_in_background: true,
            model_override: Some("gpt-4".to_string()),
            thoroughness: Thoroughness::VeryThorough,
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: SubAgentConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.agent_type, AgentType::Plan);
        assert_eq!(deserialized.prompt, "Design the system");
        assert_eq!(deserialized.resume_id, Some("resume-123".to_string()));
        assert!(deserialized.run_in_background);
        assert_eq!(deserialized.model_override, Some("gpt-4".to_string()));
        assert_eq!(deserialized.thoroughness, Thoroughness::VeryThorough);
    }

    #[test]
    fn test_thoroughness_levels() {
        assert_eq!(Thoroughness::Quick.suggested_max_steps(), 5);
        assert_eq!(Thoroughness::Medium.suggested_max_steps(), 15);
        assert_eq!(Thoroughness::VeryThorough.suggested_max_steps(), 30);

        assert_eq!(Thoroughness::Quick.as_str(), "quick");
        assert_eq!(Thoroughness::Medium.as_str(), "medium");
        assert_eq!(Thoroughness::VeryThorough.as_str(), "very_thorough");
    }

    #[test]
    fn test_subagent_result_serde() {
        let result = SubAgentResult {
            agent_id: "agent-456".to_string(),
            content: "Task completed successfully".to_string(),
            metadata: ExecutionMetadata {
                total_tokens: 2500,
                total_tool_uses: 8,
                execution_time_ms: 15000,
                tools_used: vec!["Read".to_string(), "Write".to_string()],
            },
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: SubAgentResult = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.agent_id, "agent-456");
        assert_eq!(deserialized.content, "Task completed successfully");
        assert_eq!(deserialized.metadata.total_tokens, 2500);
        assert_eq!(deserialized.metadata.total_tool_uses, 8);
        assert_eq!(deserialized.metadata.execution_time_ms, 15000);
        assert_eq!(deserialized.metadata.tools_used.len(), 2);
    }
}
