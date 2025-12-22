//! Lifecycle hook traits

use async_trait::async_trait;

use crate::agent::{AgentState, AgentStep};
use crate::error::SageError;
use crate::types::TaskMetadata;

use super::super::context::{HookResult, LifecycleContext};
use super::super::error::LifecycleResult;
use super::super::phase::LifecyclePhase;

/// Async lifecycle hook trait
#[async_trait]
pub trait LifecycleHook: Send + Sync {
    /// Name of the hook for logging
    fn name(&self) -> &str;

    /// Phases this hook should run for
    fn phases(&self) -> Vec<LifecyclePhase>;

    /// Priority (higher runs first)
    fn priority(&self) -> i32 {
        0
    }

    /// Execute the hook
    async fn execute(&self, context: &LifecycleContext) -> LifecycleResult<HookResult>;
}

/// Agent lifecycle trait for agents that support lifecycle hooks
#[async_trait]
pub trait AgentLifecycle: Send + Sync {
    /// Called when the agent is initialized
    async fn on_init(&mut self) -> LifecycleResult<()> {
        Ok(())
    }

    /// Called before task execution starts
    async fn on_task_start(&mut self, task: &TaskMetadata) -> LifecycleResult<()> {
        let _ = task;
        Ok(())
    }

    /// Called before each step execution
    async fn on_step_start(&mut self, step_number: u32) -> LifecycleResult<()> {
        let _ = step_number;
        Ok(())
    }

    /// Called after each step completes
    async fn on_step_complete(&mut self, step: &AgentStep) -> LifecycleResult<()> {
        let _ = step;
        Ok(())
    }

    /// Called after task execution completes
    async fn on_task_complete(
        &mut self,
        task: &TaskMetadata,
        success: bool,
        result: Option<&str>,
    ) -> LifecycleResult<()> {
        let _ = (task, success, result);
        Ok(())
    }

    /// Called when the agent is shut down
    async fn on_shutdown(&mut self) -> LifecycleResult<()> {
        Ok(())
    }

    /// Called on state transitions
    async fn on_state_change(&mut self, from: AgentState, to: AgentState) -> LifecycleResult<()> {
        let _ = (from, to);
        Ok(())
    }

    /// Called when an error occurs
    async fn on_error(&mut self, error: &SageError) -> LifecycleResult<()> {
        let _ = error;
        Ok(())
    }
}
