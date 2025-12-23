//! Agent mode type definitions
//!
//! This module defines the different operational modes for the agent,
//! including Plan Mode for safe exploration.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

/// Agent operational mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AgentMode {
    /// Normal mode - all tools available
    #[default]
    Normal,
    /// Plan mode - read-only exploration
    Plan,
    /// Review mode - focused on code review
    Review,
    /// Debug mode - focused on debugging
    Debug,
}

impl AgentMode {
    /// Parse from string
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "normal" => Some(Self::Normal),
            "plan" => Some(Self::Plan),
            "review" => Some(Self::Review),
            "debug" => Some(Self::Debug),
            _ => None,
        }
    }

    /// Check if this mode is read-only
    pub fn is_read_only(&self) -> bool {
        matches!(self, Self::Plan | Self::Review)
    }

    /// Check if this mode allows modifications
    pub fn allows_modifications(&self) -> bool {
        matches!(self, Self::Normal | Self::Debug)
    }

    /// Get the tools allowed in this mode
    pub fn allowed_tools(&self) -> ToolFilter {
        match self {
            Self::Normal => ToolFilter::All,
            Self::Plan => ToolFilter::ReadOnly,
            Self::Review => ToolFilter::ReadOnly,
            Self::Debug => ToolFilter::All,
        }
    }

    /// Get the mode description
    pub fn description(&self) -> &'static str {
        match self {
            Self::Normal => "Normal mode - all tools available",
            Self::Plan => "Plan mode - read-only exploration, no file modifications",
            Self::Review => "Review mode - focused on code review, read-only",
            Self::Debug => "Debug mode - focused on debugging with all tools",
        }
    }
}

impl std::fmt::Display for AgentMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Normal => write!(f, "normal"),
            Self::Plan => write!(f, "plan"),
            Self::Review => write!(f, "review"),
            Self::Debug => write!(f, "debug"),
        }
    }
}

/// Tool filter for modes
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolFilter {
    /// All tools allowed
    All,
    /// Only read-only tools
    ReadOnly,
    /// Specific tools only
    Specific(HashSet<String>),
    /// All except specific tools
    Except(HashSet<String>),
}

impl ToolFilter {
    /// Read-only tools list
    pub const READ_ONLY_TOOLS: &'static [&'static str] = &[
        "Read",
        "Glob",
        "Grep",
        "WebFetch",
        "WebSearch",
        "AskUserQuestion",
    ];

    /// Write/modify tools list
    pub const WRITE_TOOLS: &'static [&'static str] = &["Write", "Edit", "Bash", "NotebookEdit"];

    /// Check if a tool is allowed
    pub fn allows(&self, tool_name: &str) -> bool {
        match self {
            Self::All => true,
            Self::ReadOnly => Self::READ_ONLY_TOOLS
                .iter()
                .any(|t| t.eq_ignore_ascii_case(tool_name)),
            Self::Specific(allowed) => allowed.iter().any(|t| t.eq_ignore_ascii_case(tool_name)),
            Self::Except(blocked) => !blocked.iter().any(|t| t.eq_ignore_ascii_case(tool_name)),
        }
    }

    /// Get all allowed tool names
    pub fn allowed_tools(&self) -> Vec<&'static str> {
        match self {
            Self::All => {
                let mut all = Self::READ_ONLY_TOOLS.to_vec();
                all.extend(Self::WRITE_TOOLS);
                all
            }
            Self::ReadOnly => Self::READ_ONLY_TOOLS.to_vec(),
            Self::Specific(_) | Self::Except(_) => {
                // Would need full tool list to compute
                Vec::new()
            }
        }
    }
}

impl Default for ToolFilter {
    fn default() -> Self {
        Self::All
    }
}

/// Plan mode configuration
#[derive(Debug, Clone, Default)]
pub struct PlanModeConfig {
    /// Plan file path
    pub plan_file: Option<PathBuf>,
    /// Whether to auto-exit plan mode after plan is approved
    pub auto_exit_on_approval: bool,
    /// Whether to show plan diff when exiting
    pub show_plan_diff: bool,
    /// Maximum plan file size
    pub max_plan_size: usize,
}

impl PlanModeConfig {
    /// Create new config
    pub fn new() -> Self {
        Self {
            plan_file: None,
            auto_exit_on_approval: true,
            show_plan_diff: true,
            max_plan_size: 100_000, // 100KB
        }
    }

    /// Set plan file
    pub fn with_plan_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.plan_file = Some(path.into());
        self
    }

    /// Disable auto-exit
    pub fn without_auto_exit(mut self) -> Self {
        self.auto_exit_on_approval = false;
        self
    }
}

/// Mode transition request
#[derive(Debug, Clone)]
pub struct ModeTransition {
    /// From mode
    pub from: AgentMode,
    /// To mode
    pub to: AgentMode,
    /// Reason for transition
    pub reason: String,
    /// Whether user approval is required
    pub requires_approval: bool,
}

impl ModeTransition {
    /// Create a transition
    pub fn new(from: AgentMode, to: AgentMode, reason: impl Into<String>) -> Self {
        let requires_approval = Self::needs_approval(from, to);
        Self {
            from,
            to,
            reason: reason.into(),
            requires_approval,
        }
    }

    /// Check if transition needs user approval
    fn needs_approval(from: AgentMode, to: AgentMode) -> bool {
        // Entering plan mode doesn't need approval
        // Exiting plan mode to normal needs approval
        match (from, to) {
            (AgentMode::Plan, AgentMode::Normal) => true,
            (AgentMode::Review, AgentMode::Normal) => true,
            _ => false,
        }
    }

    /// Check if this is entering a restricted mode
    pub fn is_entering_restricted(&self) -> bool {
        !self.from.is_read_only() && self.to.is_read_only()
    }

    /// Check if this is exiting a restricted mode
    pub fn is_exiting_restricted(&self) -> bool {
        self.from.is_read_only() && !self.to.is_read_only()
    }
}

/// Mode state for tracking current mode
#[derive(Debug, Clone)]
pub struct ModeState {
    /// Current mode
    pub mode: AgentMode,
    /// When mode was entered
    pub entered_at: chrono::DateTime<chrono::Utc>,
    /// Plan mode config (if in plan mode)
    pub plan_config: Option<PlanModeConfig>,
    /// Tools blocked count (for metrics)
    pub blocked_tool_calls: usize,
}

impl ModeState {
    /// Create new state
    pub fn new(mode: AgentMode) -> Self {
        Self {
            mode,
            entered_at: chrono::Utc::now(),
            plan_config: if mode == AgentMode::Plan {
                Some(PlanModeConfig::new())
            } else {
                None
            },
            blocked_tool_calls: 0,
        }
    }

    /// Check if a tool is allowed
    pub fn is_tool_allowed(&self, tool_name: &str) -> bool {
        self.mode.allowed_tools().allows(tool_name)
    }

    /// Record a blocked tool call
    pub fn record_blocked(&mut self) {
        self.blocked_tool_calls += 1;
    }

    /// Get duration in current mode
    pub fn duration(&self) -> chrono::Duration {
        chrono::Utc::now() - self.entered_at
    }
}

impl Default for ModeState {
    fn default() -> Self {
        Self::new(AgentMode::Normal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_mode_parsing() {
        assert_eq!(AgentMode::from_str("normal"), Some(AgentMode::Normal));
        assert_eq!(AgentMode::from_str("plan"), Some(AgentMode::Plan));
        assert_eq!(AgentMode::from_str("PLAN"), Some(AgentMode::Plan));
        assert_eq!(AgentMode::from_str("invalid"), None);
    }

    #[test]
    fn test_agent_mode_read_only() {
        assert!(!AgentMode::Normal.is_read_only());
        assert!(AgentMode::Plan.is_read_only());
        assert!(AgentMode::Review.is_read_only());
        assert!(!AgentMode::Debug.is_read_only());
    }

    #[test]
    fn test_agent_mode_display() {
        assert_eq!(AgentMode::Normal.to_string(), "normal");
        assert_eq!(AgentMode::Plan.to_string(), "plan");
    }

    #[test]
    fn test_tool_filter_all() {
        let filter = ToolFilter::All;
        assert!(filter.allows("Read"));
        assert!(filter.allows("Write"));
        assert!(filter.allows("Bash"));
    }

    #[test]
    fn test_tool_filter_read_only() {
        let filter = ToolFilter::ReadOnly;
        assert!(filter.allows("Read"));
        assert!(filter.allows("Glob"));
        assert!(filter.allows("Grep"));
        assert!(!filter.allows("Write"));
        assert!(!filter.allows("Edit"));
        assert!(!filter.allows("Bash"));
    }

    #[test]
    fn test_tool_filter_specific() {
        let mut allowed = HashSet::new();
        allowed.insert("Read".to_string());
        allowed.insert("Grep".to_string());

        let filter = ToolFilter::Specific(allowed);
        assert!(filter.allows("Read"));
        assert!(filter.allows("Grep"));
        assert!(!filter.allows("Write"));
    }

    #[test]
    fn test_tool_filter_except() {
        let mut blocked = HashSet::new();
        blocked.insert("Bash".to_string());

        let filter = ToolFilter::Except(blocked);
        assert!(filter.allows("Read"));
        assert!(filter.allows("Write"));
        assert!(!filter.allows("Bash"));
    }

    #[test]
    fn test_plan_mode_config() {
        let config = PlanModeConfig::new()
            .with_plan_file("/tmp/plan.md")
            .without_auto_exit();

        assert_eq!(config.plan_file, Some(PathBuf::from("/tmp/plan.md")));
        assert!(!config.auto_exit_on_approval);
    }

    #[test]
    fn test_mode_transition() {
        let transition =
            ModeTransition::new(AgentMode::Normal, AgentMode::Plan, "Starting plan mode");

        assert!(!transition.requires_approval);
        assert!(transition.is_entering_restricted());
        assert!(!transition.is_exiting_restricted());
    }

    #[test]
    fn test_mode_transition_exit_requires_approval() {
        let transition =
            ModeTransition::new(AgentMode::Plan, AgentMode::Normal, "Exiting plan mode");

        assert!(transition.requires_approval);
        assert!(!transition.is_entering_restricted());
        assert!(transition.is_exiting_restricted());
    }

    #[test]
    fn test_mode_state() {
        let mut state = ModeState::new(AgentMode::Plan);

        assert!(state.is_tool_allowed("Read"));
        assert!(!state.is_tool_allowed("Write"));

        state.record_blocked();
        assert_eq!(state.blocked_tool_calls, 1);
    }

    #[test]
    fn test_mode_state_duration() {
        let state = ModeState::new(AgentMode::Normal);
        let duration = state.duration();
        assert!(duration.num_milliseconds() >= 0);
    }

    #[test]
    fn test_allowed_tools_list() {
        let read_only = ToolFilter::ReadOnly.allowed_tools();
        assert!(read_only.contains(&"Read"));
        assert!(read_only.contains(&"Glob"));
        assert!(!read_only.contains(&"Write"));
    }
}
