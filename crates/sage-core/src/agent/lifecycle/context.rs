//! Lifecycle context and hook results

use crate::agent::{AgentExecution, AgentState, AgentStep};
use crate::types::TaskMetadata;

use super::phase::LifecyclePhase;

/// Context passed to lifecycle hooks
#[derive(Debug, Clone)]
pub struct LifecycleContext {
    /// Current phase
    pub phase: LifecyclePhase,
    /// Agent ID
    pub agent_id: Option<crate::types::Id>,
    /// Current state
    pub state: AgentState,
    /// Previous state (for transitions)
    pub previous_state: Option<AgentState>,
    /// Task metadata (if in a task)
    pub task: Option<TaskMetadata>,
    /// Current step number
    pub step_number: Option<u32>,
    /// Current step (for step hooks)
    pub step: Option<AgentStep>,
    /// Execution so far (for task hooks)
    pub execution: Option<AgentExecution>,
    /// Error message (for error hooks)
    pub error: Option<String>,
    /// Additional metadata
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

impl LifecycleContext {
    /// Create a new lifecycle context
    pub fn new(phase: LifecyclePhase, state: AgentState) -> Self {
        Self {
            phase,
            agent_id: None,
            state,
            previous_state: None,
            task: None,
            step_number: None,
            step: None,
            execution: None,
            error: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Set agent ID
    pub fn with_agent_id(mut self, id: crate::types::Id) -> Self {
        self.agent_id = Some(id);
        self
    }

    /// Set task
    pub fn with_task(mut self, task: TaskMetadata) -> Self {
        self.task = Some(task);
        self
    }

    /// Set step number
    pub fn with_step_number(mut self, step: u32) -> Self {
        self.step_number = Some(step);
        self
    }

    /// Set current step
    pub fn with_step(mut self, step: AgentStep) -> Self {
        self.step = Some(step);
        self
    }

    /// Set execution
    pub fn with_execution(mut self, execution: AgentExecution) -> Self {
        self.execution = Some(execution);
        self
    }

    /// Set previous state
    pub fn with_previous_state(mut self, state: AgentState) -> Self {
        self.previous_state = Some(state);
        self
    }

    /// Set error
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Result from a hook execution
#[derive(Debug, Clone)]
pub enum HookResult {
    /// Continue normal execution
    Continue,
    /// Skip remaining hooks for this phase
    Skip,
    /// Abort the operation
    Abort(String),
    /// Modify context and continue
    ModifyContext(Box<LifecycleContext>),
}

impl Default for HookResult {
    fn default() -> Self {
        Self::Continue
    }
}
