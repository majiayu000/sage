//! Core mode manager implementation

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::modes::types::{AgentMode, ModeState, ModeTransition, ToolFilter};

/// Mode manager for controlling agent modes
pub struct ModeManager {
    /// Current mode state
    pub(super) state: Arc<RwLock<ModeState>>,
    /// Transition history
    pub(super) transitions: Arc<RwLock<Vec<ModeTransition>>>,
    /// Plan file directory
    pub(super) plan_dir: PathBuf,
}

impl ModeManager {
    /// Create a new mode manager
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(ModeState::default())),
            transitions: Arc::new(RwLock::new(Vec::new())),
            plan_dir: Self::default_plan_dir(),
        }
    }

    /// Create with custom plan directory
    pub fn with_plan_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.plan_dir = dir.into();
        self
    }

    /// Get default plan directory
    fn default_plan_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("sage")
            .join("plans")
    }

    /// Get current mode
    pub async fn current_mode(&self) -> AgentMode {
        self.state.read().await.mode
    }

    /// Get current mode state
    pub async fn current_state(&self) -> ModeState {
        self.state.read().await.clone()
    }

    /// Check if current mode is read-only
    pub async fn is_read_only(&self) -> bool {
        self.state.read().await.mode.is_read_only()
    }

    /// Check if a tool is allowed in current mode
    pub async fn is_tool_allowed(&self, tool_name: &str) -> bool {
        self.state.read().await.is_tool_allowed(tool_name)
    }

    /// Get tool filter for current mode
    pub async fn tool_filter(&self) -> ToolFilter {
        self.state.read().await.mode.allowed_tools()
    }
}

impl Default for ModeManager {
    fn default() -> Self {
        Self::new()
    }
}
