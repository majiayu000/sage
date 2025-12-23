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
            return Err(super::error::LifecycleError::invalid_transition(from, to));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_new_manager() {
        let manager = LifecycleManager::new();
        assert_eq!(manager.state().await, AgentState::Initializing);
        assert!(!manager.is_initialized().await);
    }

    #[tokio::test]
    async fn test_initialize() {
        let manager = LifecycleManager::new();
        let agent_id = uuid::Uuid::new_v4();

        let result = manager.initialize(agent_id).await;
        assert!(result.is_ok());
        assert!(manager.is_initialized().await);
    }

    #[tokio::test]
    async fn test_notify_task_start() {
        let manager = LifecycleManager::new();
        let agent_id = uuid::Uuid::new_v4();

        manager.initialize(agent_id).await.unwrap();

        let task = TaskMetadata::new("Test task", ".");

        let result = manager.notify_task_start(agent_id, &task).await;
        assert!(result.is_ok());
        assert_eq!(manager.state().await, AgentState::Thinking);
    }

    #[tokio::test]
    async fn test_notify_step_start() {
        let manager = LifecycleManager::new();
        let agent_id = uuid::Uuid::new_v4();

        manager.initialize(agent_id).await.unwrap();

        let result = manager.notify_step_start(agent_id, 1).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_notify_step_complete() {
        let manager = LifecycleManager::new();
        let agent_id = uuid::Uuid::new_v4();

        manager.initialize(agent_id).await.unwrap();

        let step = AgentStep::new(1, AgentState::Completed);
        let result = manager.notify_step_complete(agent_id, &step).await;

        assert!(result.is_ok());
        assert_eq!(manager.state().await, AgentState::Completed);
    }

    #[tokio::test]
    async fn test_notify_task_complete() {
        let manager = LifecycleManager::new();
        let agent_id = uuid::Uuid::new_v4();

        manager.initialize(agent_id).await.unwrap();

        let mut execution = AgentExecution::new(TaskMetadata::new("Test task", "."));
        execution.complete(true, Some("Success".to_string()));

        let result = manager.notify_task_complete(agent_id, &execution).await;
        assert!(result.is_ok());
        assert_eq!(manager.state().await, AgentState::Completed);
    }

    #[tokio::test]
    async fn test_notify_state_change_valid() {
        let manager = LifecycleManager::new();
        let agent_id = uuid::Uuid::new_v4();

        manager.initialize(agent_id).await.unwrap();

        // Valid transition: Initializing -> Thinking
        let result = manager
            .notify_state_change(agent_id, AgentState::Initializing, AgentState::Thinking)
            .await;

        assert!(result.is_ok());
        assert_eq!(manager.state().await, AgentState::Thinking);
    }

    #[tokio::test]
    async fn test_notify_state_change_invalid() {
        let manager = LifecycleManager::new();
        let agent_id = uuid::Uuid::new_v4();

        manager.initialize(agent_id).await.unwrap();

        // Invalid transition: Initializing -> Completed
        let result = manager
            .notify_state_change(agent_id, AgentState::Initializing, AgentState::Completed)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_notify_error() {
        let manager = LifecycleManager::new();
        let agent_id = uuid::Uuid::new_v4();

        manager.initialize(agent_id).await.unwrap();

        let error = crate::error::SageError::agent("Test error");
        let result = manager.notify_error(agent_id, &error).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_shutdown() {
        let manager = LifecycleManager::new();
        let agent_id = uuid::Uuid::new_v4();

        manager.initialize(agent_id).await.unwrap();
        assert!(manager.is_initialized().await);

        let result = manager.shutdown(agent_id).await;
        assert!(result.is_ok());
        assert!(!manager.is_initialized().await);
    }

    #[tokio::test]
    async fn test_with_registry() {
        let registry = Arc::new(LifecycleHookRegistry::new());
        let _manager = LifecycleManager::with_registry(registry.clone());

        assert_eq!(Arc::strong_count(&registry), 2); // manager + our reference
    }
}
