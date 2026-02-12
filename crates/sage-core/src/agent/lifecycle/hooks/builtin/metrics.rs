//! Metrics collection hook

use async_trait::async_trait;

use crate::agent::lifecycle::context::{HookResult, LifecycleContext};
use crate::agent::lifecycle::error::LifecycleResult;
use crate::agent::lifecycle::hooks::traits::LifecycleHook;
use crate::agent::lifecycle::phase::LifecyclePhase;

/// Metrics collection hook
pub struct MetricsHook {
    name: String,
}

impl MetricsHook {
    /// Create a new metrics hook
    pub fn new() -> Self {
        Self {
            name: "metrics".to_string(),
        }
    }
}

impl Default for MetricsHook {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LifecycleHook for MetricsHook {
    fn name(&self) -> &str {
        &self.name
    }

    fn phases(&self) -> Vec<LifecyclePhase> {
        vec![
            LifecyclePhase::TaskStart,
            LifecyclePhase::StepComplete,
            LifecyclePhase::TaskComplete,
            LifecyclePhase::Error,
        ]
    }

    fn priority(&self) -> i32 {
        -100 // Run after other hooks
    }

    async fn execute(&self, context: &LifecycleContext) -> LifecycleResult<HookResult> {
        match context.phase {
            LifecyclePhase::TaskStart => {
                tracing::info!(
                    task = ?context.task.as_ref().map(|t| &t.description),
                    "Task started"
                );
            }
            LifecyclePhase::StepComplete => {
                if let Some(step) = &context.step {
                    tracing::info!(
                        step_number = step.step_number,
                        state = %step.state,
                        tool_calls = step.tool_calls.len(),
                        "Step completed"
                    );
                }
            }
            LifecyclePhase::TaskComplete => {
                if let Some(execution) = &context.execution {
                    tracing::info!(
                        success = execution.success,
                        steps = execution.steps.len(),
                        total_tokens = execution.total_usage.total_tokens(),
                        "Task completed"
                    );
                }
            }
            LifecyclePhase::Error => {
                tracing::error!(
                    error = ?context.error,
                    state = %context.state,
                    "Error occurred"
                );
            }
            _ => {}
        }
        Ok(HookResult::Continue)
    }
}
