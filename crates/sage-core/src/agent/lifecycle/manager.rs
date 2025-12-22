//! Lifecycle manager for coordinating hooks and state

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::agent::{AgentExecution, AgentState, AgentStep};
use crate::error::SageError;
use crate::types::TaskMetadata;

use super::context::LifecycleContext;
use super::error::LifecycleResult;
use super::hooks::LifecycleHookRegistry;
use super::phase::LifecyclePhase;

/// Lifecycle manager that coordinates hooks and state
pub struct LifecycleManager {
    /// Hook registry
    registry: Arc<LifecycleHookRegistry>,
    /// Current state
    state: RwLock<AgentState>,
    /// Whether initialized
    initialized: RwLock<bool>,
}

impl LifecycleManager {
    /// Create a new lifecycle manager
    pub fn new() -> Self {
        Self {
            registry: Arc::new(LifecycleHookRegistry::new()),
            state: RwLock::new(AgentState::Initializing),
            initialized: RwLock::new(false),
        }
    }

    /// Create with existing registry
    pub fn with_registry(registry: Arc<LifecycleHookRegistry>) -> Self {
        Self {
            registry,
            state: RwLock::new(AgentState::Initializing),
            initialized: RwLock::new(false),
        }
    }

    /// Get the hook registry
    pub fn registry(&self) -> Arc<LifecycleHookRegistry> {
        self.registry.clone()
    }

    /// Get current state
    pub async fn state(&self) -> AgentState {
        *self.state.read().await
    }

    /// Initialize the lifecycle manager
    pub async fn initialize(&self, agent_id: crate::types::Id) -> LifecycleResult<()> {
        let context = LifecycleContext::new(LifecyclePhase::Init, AgentState::Initializing)
            .with_agent_id(agent_id);

        self.registry
            .execute_hooks(LifecyclePhase::Init, context)
            .await?;

        *self.initialized.write().await = true;
        Ok(())
    }

    /// Notify task start
    pub async fn notify_task_start(
        &self,
        agent_id: crate::types::Id,
        task: &TaskMetadata,
    ) -> LifecycleResult<()> {
        let context = LifecycleContext::new(LifecyclePhase::TaskStart, AgentState::Thinking)
            .with_agent_id(agent_id)
            .with_task(task.clone());

        self.registry
            .execute_hooks(LifecyclePhase::TaskStart, context)
            .await?;

        *self.state.write().await = AgentState::Thinking;
        Ok(())
    }

    /// Notify step start
    pub async fn notify_step_start(
        &self,
        agent_id: crate::types::Id,
        step_number: u32,
    ) -> LifecycleResult<()> {
        let current_state = *self.state.read().await;
        let context = LifecycleContext::new(LifecyclePhase::StepStart, current_state)
            .with_agent_id(agent_id)
            .with_step_number(step_number);

        self.registry
            .execute_hooks(LifecyclePhase::StepStart, context)
            .await?;

        Ok(())
    }

    /// Notify step complete
    pub async fn notify_step_complete(
        &self,
        agent_id: crate::types::Id,
        step: &AgentStep,
    ) -> LifecycleResult<()> {
        let context = LifecycleContext::new(LifecyclePhase::StepComplete, step.state)
            .with_agent_id(agent_id)
            .with_step_number(step.step_number)
            .with_step(step.clone());

        self.registry
            .execute_hooks(LifecyclePhase::StepComplete, context)
            .await?;

        *self.state.write().await = step.state;
        Ok(())
    }

    /// Notify task complete
    pub async fn notify_task_complete(
        &self,
        agent_id: crate::types::Id,
        execution: &AgentExecution,
    ) -> LifecycleResult<()> {
        let state = if execution.success {
            AgentState::Completed
        } else {
            AgentState::Error
        };

        let context = LifecycleContext::new(LifecyclePhase::TaskComplete, state)
            .with_agent_id(agent_id)
            .with_task(execution.task.clone())
            .with_execution(execution.clone());

        self.registry
            .execute_hooks(LifecyclePhase::TaskComplete, context)
            .await?;

        *self.state.write().await = state;
        Ok(())
    }

    /// Notify state transition
    pub async fn notify_state_change(
        &self,
        agent_id: crate::types::Id,
        from: AgentState,
        to: AgentState,
    ) -> LifecycleResult<()> {
        // Validate transition
        if !from.can_transition_to(&to) {
            return Err(super::error::LifecycleError::InvalidTransition { from, to });
        }

        let context = LifecycleContext::new(LifecyclePhase::StateTransition, to)
            .with_agent_id(agent_id)
            .with_previous_state(from);

        self.registry
            .execute_hooks(LifecyclePhase::StateTransition, context)
            .await?;

        *self.state.write().await = to;
        Ok(())
    }

    /// Notify error
    pub async fn notify_error(
        &self,
        agent_id: crate::types::Id,
        error: &SageError,
    ) -> LifecycleResult<()> {
        let current_state = *self.state.read().await;
        let context = LifecycleContext::new(LifecyclePhase::Error, current_state)
            .with_agent_id(agent_id)
            .with_error(error.to_string());

        self.registry
            .execute_hooks(LifecyclePhase::Error, context)
            .await?;

        Ok(())
    }

    /// Shutdown the lifecycle manager
    pub async fn shutdown(&self, agent_id: crate::types::Id) -> LifecycleResult<()> {
        let current_state = *self.state.read().await;
        let context =
            LifecycleContext::new(LifecyclePhase::Shutdown, current_state).with_agent_id(agent_id);

        self.registry
            .execute_hooks(LifecyclePhase::Shutdown, context)
            .await?;

        *self.initialized.write().await = false;
        Ok(())
    }

    /// Check if initialized
    pub async fn is_initialized(&self) -> bool {
        *self.initialized.read().await
    }
}

impl Default for LifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}
