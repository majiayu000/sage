//! Mode manager
//!
//! This module provides the mode manager for controlling agent operational modes.

use crate::error::{SageError, SageResult};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::RwLock;

use super::types::{AgentMode, ModeState, ModeTransition, PlanModeConfig, ToolFilter};

/// Mode manager for controlling agent modes
pub struct ModeManager {
    /// Current mode state
    state: Arc<RwLock<ModeState>>,
    /// Transition history
    transitions: Arc<RwLock<Vec<ModeTransition>>>,
    /// Plan file directory
    plan_dir: PathBuf,
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

    /// Enter plan mode
    pub async fn enter_plan_mode(&self, plan_name: Option<&str>) -> SageResult<PlanModeContext> {
        let current = self.current_mode().await;

        if current == AgentMode::Plan {
            return Err(SageError::InvalidInput("Already in plan mode".to_string()));
        }

        // Generate plan file path
        let plan_file = self.generate_plan_path(plan_name);

        // Ensure plan directory exists
        if let Some(parent) = plan_file.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                SageError::Storage(format!("Failed to create plan directory: {}", e))
            })?;
        }

        // Create plan config
        let config = PlanModeConfig::new().with_plan_file(&plan_file);

        // Record transition
        let transition = ModeTransition::new(
            current,
            AgentMode::Plan,
            "Entering plan mode for architecture/design exploration",
        );

        {
            let mut transitions = self.transitions.write().await;
            transitions.push(transition);
        }

        // Update state
        {
            let mut state = self.state.write().await;
            state.mode = AgentMode::Plan;
            state.entered_at = chrono::Utc::now();
            state.plan_config = Some(config.clone());
            state.blocked_tool_calls = 0;
        }

        Ok(PlanModeContext {
            plan_file,
            config,
            previous_mode: current,
        })
    }

    /// Exit plan mode (requires approval for normal exit)
    pub async fn exit_plan_mode(&self, approved: bool) -> SageResult<ModeExitResult> {
        let state = self.state.read().await;

        if state.mode != AgentMode::Plan {
            return Err(SageError::InvalidInput("Not in plan mode".to_string()));
        }

        let plan_config = state.plan_config.clone();
        let blocked_count = state.blocked_tool_calls;
        let duration = state.duration();
        drop(state);

        // Check approval
        if !approved {
            return Ok(ModeExitResult {
                exited: false,
                plan_file: plan_config.and_then(|c| c.plan_file),
                blocked_tool_calls: blocked_count,
                duration_secs: duration.num_seconds() as u64,
                message: "Plan mode exit requires approval".to_string(),
            });
        }

        // Record transition
        let transition = ModeTransition::new(
            AgentMode::Plan,
            AgentMode::Normal,
            "Exiting plan mode with approval",
        );

        {
            let mut transitions = self.transitions.write().await;
            transitions.push(transition);
        }

        // Update state
        let plan_file = {
            let mut state = self.state.write().await;
            let pf = state.plan_config.as_ref().and_then(|c| c.plan_file.clone());
            state.mode = AgentMode::Normal;
            state.entered_at = chrono::Utc::now();
            state.plan_config = None;
            pf
        };

        Ok(ModeExitResult {
            exited: true,
            plan_file,
            blocked_tool_calls: blocked_count,
            duration_secs: duration.num_seconds() as u64,
            message: "Exited plan mode successfully".to_string(),
        })
    }

    /// Transition to a different mode
    pub async fn transition_to(&self, mode: AgentMode, reason: &str) -> SageResult<()> {
        let current = self.current_mode().await;

        if current == mode {
            return Ok(()); // Already in target mode
        }

        let transition = ModeTransition::new(current, mode, reason);

        // Check if approval is needed
        if transition.requires_approval {
            return Err(SageError::InvalidInput(format!(
                "Transition from {} to {} requires approval",
                current, mode
            )));
        }

        // Record transition
        {
            let mut transitions = self.transitions.write().await;
            transitions.push(transition);
        }

        // Update state
        {
            let mut state = self.state.write().await;
            state.mode = mode;
            state.entered_at = chrono::Utc::now();
            state.blocked_tool_calls = 0;

            if mode == AgentMode::Plan {
                state.plan_config = Some(PlanModeConfig::new());
            } else {
                state.plan_config = None;
            }
        }

        Ok(())
    }

    /// Record a blocked tool call
    pub async fn record_blocked_tool(&self, tool_name: &str) {
        let mut state = self.state.write().await;
        state.record_blocked();
        tracing::warn!("Tool '{}' blocked in {} mode", tool_name, state.mode);
    }

    /// Generate a plan file path
    fn generate_plan_path(&self, name: Option<&str>) -> PathBuf {
        let generated_name;
        let name = match name {
            Some(n) => n,
            None => {
                // Generate a unique name
                generated_name = uuid::Uuid::new_v4().to_string();
                &generated_name[..8]
            }
        };

        // Create a descriptive name
        let adjectives = ["ancient", "bright", "cosmic", "dancing", "elegant"];
        let nouns = ["river", "mountain", "forest", "ocean", "meadow"];

        let idx1 = name.bytes().next().unwrap_or(0) as usize % adjectives.len();
        let idx2 = name.bytes().last().unwrap_or(0) as usize % nouns.len();

        let descriptive = format!(
            "{}-{}-{}",
            adjectives[idx1],
            nouns[idx2],
            &name[..4.min(name.len())]
        );

        self.plan_dir.join(format!("{}.md", descriptive))
    }

    /// Save plan content
    pub async fn save_plan(&self, content: &str) -> SageResult<PathBuf> {
        let state = self.state.read().await;

        let plan_file = state
            .plan_config
            .as_ref()
            .and_then(|c| c.plan_file.clone())
            .ok_or_else(|| SageError::InvalidInput("No plan file configured".to_string()))?;

        drop(state);

        // Ensure directory exists
        if let Some(parent) = plan_file.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                SageError::Storage(format!("Failed to create plan directory: {}", e))
            })?;
        }

        // Write content
        let mut file = fs::File::create(&plan_file)
            .await
            .map_err(|e| SageError::Storage(format!("Failed to create plan file: {}", e)))?;

        file.write_all(content.as_bytes())
            .await
            .map_err(|e| SageError::Storage(format!("Failed to write plan file: {}", e)))?;

        tracing::info!("Saved plan to {:?}", plan_file);
        Ok(plan_file)
    }

    /// Load plan content
    pub async fn load_plan(&self) -> SageResult<Option<String>> {
        let state = self.state.read().await;

        let plan_file = match state.plan_config.as_ref().and_then(|c| c.plan_file.clone()) {
            Some(f) => f,
            None => return Ok(None),
        };

        drop(state);

        if !plan_file.exists() {
            return Ok(None);
        }

        let mut file = fs::File::open(&plan_file)
            .await
            .map_err(|e| SageError::Storage(format!("Failed to open plan file: {}", e)))?;

        let mut content = String::new();
        file.read_to_string(&mut content)
            .await
            .map_err(|e| SageError::Storage(format!("Failed to read plan file: {}", e)))?;

        Ok(Some(content))
    }

    /// Get transition history
    pub async fn get_transitions(&self) -> Vec<ModeTransition> {
        self.transitions.read().await.clone()
    }

    /// Clear transition history
    pub async fn clear_transitions(&self) {
        self.transitions.write().await.clear();
    }
}

impl Default for ModeManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Context returned when entering plan mode
#[derive(Debug, Clone)]
pub struct PlanModeContext {
    /// Path to the plan file
    pub plan_file: PathBuf,
    /// Plan mode configuration
    pub config: PlanModeConfig,
    /// Previous mode before entering plan mode
    pub previous_mode: AgentMode,
}

/// Result of exiting a mode
#[derive(Debug, Clone)]
pub struct ModeExitResult {
    /// Whether the mode was actually exited
    pub exited: bool,
    /// Plan file path (if was in plan mode)
    pub plan_file: Option<PathBuf>,
    /// Number of tool calls blocked during mode
    pub blocked_tool_calls: usize,
    /// Duration in the mode (seconds)
    pub duration_secs: u64,
    /// Status message
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mode_manager_creation() {
        let manager = ModeManager::new();
        assert_eq!(manager.current_mode().await, AgentMode::Normal);
    }

    #[tokio::test]
    async fn test_enter_plan_mode() {
        let manager = ModeManager::new();

        let context = manager.enter_plan_mode(Some("test")).await.unwrap();

        assert_eq!(manager.current_mode().await, AgentMode::Plan);
        assert!(context.plan_file.to_string_lossy().contains(".md"));
    }

    #[tokio::test]
    async fn test_exit_plan_mode_without_approval() {
        let manager = ModeManager::new();
        manager.enter_plan_mode(None).await.unwrap();

        let result = manager.exit_plan_mode(false).await.unwrap();

        assert!(!result.exited);
        assert_eq!(manager.current_mode().await, AgentMode::Plan);
    }

    #[tokio::test]
    async fn test_exit_plan_mode_with_approval() {
        let manager = ModeManager::new();
        manager.enter_plan_mode(None).await.unwrap();

        let result = manager.exit_plan_mode(true).await.unwrap();

        assert!(result.exited);
        assert_eq!(manager.current_mode().await, AgentMode::Normal);
    }

    #[tokio::test]
    async fn test_is_tool_allowed_normal_mode() {
        let manager = ModeManager::new();

        assert!(manager.is_tool_allowed("Read").await);
        assert!(manager.is_tool_allowed("Write").await);
        assert!(manager.is_tool_allowed("Bash").await);
    }

    #[tokio::test]
    async fn test_is_tool_allowed_plan_mode() {
        let manager = ModeManager::new();
        manager.enter_plan_mode(None).await.unwrap();

        assert!(manager.is_tool_allowed("Read").await);
        assert!(manager.is_tool_allowed("Glob").await);
        assert!(!manager.is_tool_allowed("Write").await);
        assert!(!manager.is_tool_allowed("Bash").await);
    }

    #[tokio::test]
    async fn test_record_blocked_tool() {
        let manager = ModeManager::new();
        manager.enter_plan_mode(None).await.unwrap();

        manager.record_blocked_tool("Write").await;

        let state = manager.current_state().await;
        assert_eq!(state.blocked_tool_calls, 1);
    }

    #[tokio::test]
    async fn test_transition_to() {
        let manager = ModeManager::new();

        manager
            .transition_to(AgentMode::Debug, "Testing")
            .await
            .unwrap();
        assert_eq!(manager.current_mode().await, AgentMode::Debug);

        manager
            .transition_to(AgentMode::Review, "Review")
            .await
            .unwrap();
        assert_eq!(manager.current_mode().await, AgentMode::Review);
    }

    #[tokio::test]
    async fn test_transition_requires_approval() {
        let manager = ModeManager::new();
        manager.enter_plan_mode(None).await.unwrap();

        // Should fail without approval
        let result = manager.transition_to(AgentMode::Normal, "Exit").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_enter_plan_mode_twice_fails() {
        let manager = ModeManager::new();
        manager.enter_plan_mode(None).await.unwrap();

        let result = manager.enter_plan_mode(None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_transition_history() {
        let manager = ModeManager::new();

        manager.enter_plan_mode(None).await.unwrap();
        manager.exit_plan_mode(true).await.unwrap();

        let transitions = manager.get_transitions().await;
        assert_eq!(transitions.len(), 2);
    }

    #[tokio::test]
    async fn test_save_and_load_plan() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let manager = ModeManager::new().with_plan_dir(temp_dir.path());

        manager.enter_plan_mode(Some("test")).await.unwrap();

        let content = "# Test Plan\n\n1. Step one\n2. Step two";
        manager.save_plan(content).await.unwrap();

        let loaded = manager.load_plan().await.unwrap();
        assert_eq!(loaded, Some(content.to_string()));
    }

    #[tokio::test]
    async fn test_generate_plan_path() {
        let manager = ModeManager::new();
        let path = manager.generate_plan_path(Some("test"));

        assert!(path.to_string_lossy().contains(".md"));
    }

    #[tokio::test]
    async fn test_is_read_only() {
        let manager = ModeManager::new();

        assert!(!manager.is_read_only().await);

        manager.enter_plan_mode(None).await.unwrap();
        assert!(manager.is_read_only().await);
    }
}
