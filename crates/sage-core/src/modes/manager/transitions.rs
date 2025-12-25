//! Mode transition operations

use tokio::fs;

use crate::error::{SageError, SageResult};
use crate::modes::types::{AgentMode, ModeTransition, PlanModeConfig};

use super::core::ModeManager;
use super::types::{ModeExitResult, PlanModeContext};

impl ModeManager {
    /// Enter plan mode
    pub async fn enter_plan_mode(&self, plan_name: Option<&str>) -> SageResult<PlanModeContext> {
        let current = self.current_mode().await;

        if current == AgentMode::Plan {
            return Err(SageError::invalid_input("Already in plan mode".to_string()));
        }

        // Generate plan file path
        let plan_file = self.generate_plan_path(plan_name);

        // Ensure plan directory exists
        if let Some(parent) = plan_file.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                SageError::storage(format!("Failed to create plan directory: {}", e))
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
            return Err(SageError::invalid_input("Not in plan mode".to_string()));
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
            return Err(SageError::invalid_input(format!(
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

    /// Get transition history
    pub async fn get_transitions(&self) -> Vec<ModeTransition> {
        self.transitions.read().await.clone()
    }

    /// Clear transition history
    pub async fn clear_transitions(&self) {
        self.transitions.write().await.clear();
    }
}
