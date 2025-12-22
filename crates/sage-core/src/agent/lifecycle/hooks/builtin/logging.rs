//! Logging hook for debugging lifecycle events

use async_trait::async_trait;

use crate::agent::lifecycle::context::{HookResult, LifecycleContext};
use crate::agent::lifecycle::error::LifecycleResult;
use crate::agent::lifecycle::hooks::traits::LifecycleHook;
use crate::agent::lifecycle::phase::LifecyclePhase;

/// A simple logging hook for debugging
pub struct LoggingHook {
    name: String,
    phases: Vec<LifecyclePhase>,
}

impl LoggingHook {
    /// Create a logging hook for all phases
    pub fn all_phases() -> Self {
        Self {
            name: "logging".to_string(),
            phases: vec![
                LifecyclePhase::Init,
                LifecyclePhase::TaskStart,
                LifecyclePhase::StepStart,
                LifecyclePhase::StepComplete,
                LifecyclePhase::TaskComplete,
                LifecyclePhase::Shutdown,
                LifecyclePhase::StateTransition,
                LifecyclePhase::Error,
            ],
        }
    }

    /// Create a logging hook for specific phases
    pub fn for_phases(phases: Vec<LifecyclePhase>) -> Self {
        Self {
            name: "logging".to_string(),
            phases,
        }
    }
}

#[async_trait]
impl LifecycleHook for LoggingHook {
    fn name(&self) -> &str {
        &self.name
    }

    fn phases(&self) -> Vec<LifecyclePhase> {
        self.phases.clone()
    }

    async fn execute(&self, context: &LifecycleContext) -> LifecycleResult<HookResult> {
        tracing::debug!(
            phase = %context.phase,
            state = %context.state,
            agent_id = ?context.agent_id,
            step_number = ?context.step_number,
            "Lifecycle hook triggered"
        );
        Ok(HookResult::Continue)
    }
}
