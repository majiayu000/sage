//! Agent lifecycle hooks
//!
//! Provides lifecycle management for agents with hooks at various points
//! in the agent execution flow.

use crate::agent::{AgentExecution, AgentState, AgentStep};
use crate::error::SageError;
use crate::types::TaskMetadata;
use async_trait::async_trait;
use std::fmt;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Result type for lifecycle operations
pub type LifecycleResult<T> = Result<T, LifecycleError>;

/// Errors that can occur during lifecycle operations
#[derive(Debug, Clone)]
pub enum LifecycleError {
    /// Initialization failed
    InitFailed(String),
    /// Hook execution failed
    HookFailed {
        hook: LifecyclePhase,
        message: String,
    },
    /// State transition not allowed
    InvalidTransition { from: AgentState, to: AgentState },
    /// Shutdown failed
    ShutdownFailed(String),
    /// Hook aborted the operation
    Aborted {
        phase: LifecyclePhase,
        reason: String,
    },
    /// Wrapped sage error
    Internal(String),
}

impl fmt::Display for LifecycleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InitFailed(msg) => write!(f, "Initialization failed: {}", msg),
            Self::HookFailed { hook, message } => {
                write!(f, "Hook {} failed: {}", hook, message)
            }
            Self::InvalidTransition { from, to } => {
                write!(f, "Invalid state transition from {} to {}", from, to)
            }
            Self::ShutdownFailed(msg) => write!(f, "Shutdown failed: {}", msg),
            Self::Aborted { phase, reason } => {
                write!(f, "Aborted at {}: {}", phase, reason)
            }
            Self::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for LifecycleError {}

impl From<SageError> for LifecycleError {
    fn from(err: SageError) -> Self {
        Self::Internal(err.to_string())
    }
}

/// Lifecycle phases where hooks can be registered
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LifecyclePhase {
    /// Agent initialization (before first task)
    Init,
    /// Before task execution starts
    TaskStart,
    /// Before each step in the execution
    StepStart,
    /// After each step completes
    StepComplete,
    /// After task execution completes (success or failure)
    TaskComplete,
    /// Agent shutdown
    Shutdown,
    /// State transition
    StateTransition,
    /// Error occurred
    Error,
}

impl fmt::Display for LifecyclePhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Init => write!(f, "init"),
            Self::TaskStart => write!(f, "task_start"),
            Self::StepStart => write!(f, "step_start"),
            Self::StepComplete => write!(f, "step_complete"),
            Self::TaskComplete => write!(f, "task_complete"),
            Self::Shutdown => write!(f, "shutdown"),
            Self::StateTransition => write!(f, "state_transition"),
            Self::Error => write!(f, "error"),
        }
    }
}

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

/// Registry for managing lifecycle hooks
pub struct LifecycleHookRegistry {
    hooks: RwLock<Vec<Arc<dyn LifecycleHook>>>,
}

impl LifecycleHookRegistry {
    /// Create a new hook registry
    pub fn new() -> Self {
        Self {
            hooks: RwLock::new(Vec::new()),
        }
    }

    /// Register a hook
    pub async fn register(&self, hook: Arc<dyn LifecycleHook>) {
        let mut hooks = self.hooks.write().await;
        hooks.push(hook);
        // Sort by priority (higher first)
        hooks.sort_by(|a, b| b.priority().cmp(&a.priority()));
    }

    /// Unregister a hook by name
    pub async fn unregister(&self, name: &str) {
        let mut hooks = self.hooks.write().await;
        hooks.retain(|h| h.name() != name);
    }

    /// Execute all hooks for a phase
    pub async fn execute_hooks(
        &self,
        phase: LifecyclePhase,
        mut context: LifecycleContext,
    ) -> LifecycleResult<LifecycleContext> {
        let hooks = self.hooks.read().await;

        for hook in hooks.iter() {
            if !hook.phases().contains(&phase) {
                continue;
            }

            match hook.execute(&context).await? {
                HookResult::Continue => continue,
                HookResult::Skip => break,
                HookResult::Abort(reason) => {
                    return Err(LifecycleError::Aborted { phase, reason });
                }
                HookResult::ModifyContext(new_context) => {
                    context = *new_context;
                }
            }
        }

        Ok(context)
    }

    /// Get all registered hooks
    pub async fn hooks(&self) -> Vec<Arc<dyn LifecycleHook>> {
        self.hooks.read().await.clone()
    }

    /// Get hooks count
    pub async fn count(&self) -> usize {
        self.hooks.read().await.len()
    }
}

impl Default for LifecycleHookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

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
                        total_tokens = execution.total_usage.total_tokens,
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
            return Err(LifecycleError::InvalidTransition { from, to });
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
    use std::sync::atomic::{AtomicU32, Ordering};

    struct CountingHook {
        name: String,
        phases: Vec<LifecyclePhase>,
        count: Arc<AtomicU32>,
    }

    impl CountingHook {
        fn new(name: &str, phases: Vec<LifecyclePhase>) -> Self {
            Self {
                name: name.to_string(),
                phases,
                count: Arc::new(AtomicU32::new(0)),
            }
        }

        fn count(&self) -> u32 {
            self.count.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl LifecycleHook for CountingHook {
        fn name(&self) -> &str {
            &self.name
        }

        fn phases(&self) -> Vec<LifecyclePhase> {
            self.phases.clone()
        }

        async fn execute(&self, _context: &LifecycleContext) -> LifecycleResult<HookResult> {
            self.count.fetch_add(1, Ordering::SeqCst);
            Ok(HookResult::Continue)
        }
    }

    #[tokio::test]
    async fn test_hook_registry() {
        let registry = LifecycleHookRegistry::new();

        let hook = Arc::new(CountingHook::new(
            "test",
            vec![LifecyclePhase::Init, LifecyclePhase::TaskStart],
        ));
        registry.register(hook.clone()).await;

        assert_eq!(registry.count().await, 1);

        // Execute init hook
        let context = LifecycleContext::new(LifecyclePhase::Init, AgentState::Initializing);
        registry
            .execute_hooks(LifecyclePhase::Init, context)
            .await
            .unwrap();

        assert_eq!(hook.count(), 1);

        // Execute task start hook
        let context = LifecycleContext::new(LifecyclePhase::TaskStart, AgentState::Thinking);
        registry
            .execute_hooks(LifecyclePhase::TaskStart, context)
            .await
            .unwrap();

        assert_eq!(hook.count(), 2);

        // Execute hook for different phase (should not increment)
        let context = LifecycleContext::new(LifecyclePhase::Shutdown, AgentState::Completed);
        registry
            .execute_hooks(LifecyclePhase::Shutdown, context)
            .await
            .unwrap();

        assert_eq!(hook.count(), 2);
    }

    #[tokio::test]
    async fn test_hook_priority() {
        let registry = LifecycleHookRegistry::new();

        struct PriorityHook {
            name: String,
            priority: i32,
            order: Arc<RwLock<Vec<String>>>,
        }

        #[async_trait]
        impl LifecycleHook for PriorityHook {
            fn name(&self) -> &str {
                &self.name
            }

            fn phases(&self) -> Vec<LifecyclePhase> {
                vec![LifecyclePhase::Init]
            }

            fn priority(&self) -> i32 {
                self.priority
            }

            async fn execute(&self, _context: &LifecycleContext) -> LifecycleResult<HookResult> {
                self.order.write().await.push(self.name.clone());
                Ok(HookResult::Continue)
            }
        }

        let order = Arc::new(RwLock::new(Vec::new()));

        let hook1 = Arc::new(PriorityHook {
            name: "low".to_string(),
            priority: 0,
            order: order.clone(),
        });
        let hook2 = Arc::new(PriorityHook {
            name: "high".to_string(),
            priority: 100,
            order: order.clone(),
        });
        let hook3 = Arc::new(PriorityHook {
            name: "medium".to_string(),
            priority: 50,
            order: order.clone(),
        });

        registry.register(hook1).await;
        registry.register(hook2).await;
        registry.register(hook3).await;

        let context = LifecycleContext::new(LifecyclePhase::Init, AgentState::Initializing);
        registry
            .execute_hooks(LifecyclePhase::Init, context)
            .await
            .unwrap();

        let execution_order = order.read().await;
        assert_eq!(*execution_order, vec!["high", "medium", "low"]);
    }

    #[tokio::test]
    async fn test_hook_abort() {
        let registry = LifecycleHookRegistry::new();

        struct AbortHook;

        #[async_trait]
        impl LifecycleHook for AbortHook {
            fn name(&self) -> &str {
                "abort"
            }

            fn phases(&self) -> Vec<LifecyclePhase> {
                vec![LifecyclePhase::Init]
            }

            async fn execute(&self, _context: &LifecycleContext) -> LifecycleResult<HookResult> {
                Ok(HookResult::Abort("Test abort".to_string()))
            }
        }

        registry.register(Arc::new(AbortHook)).await;

        let context = LifecycleContext::new(LifecyclePhase::Init, AgentState::Initializing);
        let result = registry.execute_hooks(LifecyclePhase::Init, context).await;

        assert!(result.is_err());
        match result {
            Err(LifecycleError::Aborted { phase, reason }) => {
                assert_eq!(phase, LifecyclePhase::Init);
                assert_eq!(reason, "Test abort");
            }
            _ => panic!("Expected Aborted error"),
        }
    }

    #[tokio::test]
    async fn test_lifecycle_manager() {
        let manager = LifecycleManager::new();
        let agent_id = uuid::Uuid::new_v4();

        // Register a counting hook
        let hook = Arc::new(CountingHook::new(
            "test",
            vec![
                LifecyclePhase::Init,
                LifecyclePhase::TaskStart,
                LifecyclePhase::StepStart,
                LifecyclePhase::StepComplete,
                LifecyclePhase::TaskComplete,
            ],
        ));
        manager.registry().register(hook.clone()).await;

        // Initialize
        manager.initialize(agent_id).await.unwrap();
        assert!(manager.is_initialized().await);
        assert_eq!(hook.count(), 1);

        // Task start
        let task = TaskMetadata::new("Test task", "/tmp");
        manager.notify_task_start(agent_id, &task).await.unwrap();
        assert_eq!(manager.state().await, AgentState::Thinking);
        assert_eq!(hook.count(), 2);

        // Step start
        manager.notify_step_start(agent_id, 1).await.unwrap();
        assert_eq!(hook.count(), 3);

        // Step complete
        let step = AgentStep::new(1, AgentState::ToolExecution);
        manager.notify_step_complete(agent_id, &step).await.unwrap();
        assert_eq!(hook.count(), 4);

        // Shutdown
        manager.shutdown(agent_id).await.unwrap();
        assert!(!manager.is_initialized().await);
    }

    #[tokio::test]
    async fn test_lifecycle_context_builder() {
        let context = LifecycleContext::new(LifecyclePhase::TaskStart, AgentState::Thinking)
            .with_agent_id(uuid::Uuid::new_v4())
            .with_task(TaskMetadata::new("Test", "/tmp"))
            .with_step_number(1)
            .with_metadata("custom", serde_json::json!({"key": "value"}));

        assert_eq!(context.phase, LifecyclePhase::TaskStart);
        assert_eq!(context.state, AgentState::Thinking);
        assert!(context.agent_id.is_some());
        assert!(context.task.is_some());
        assert_eq!(context.step_number, Some(1));
        assert!(context.metadata.contains_key("custom"));
    }

    #[tokio::test]
    async fn test_logging_hook() {
        let hook = LoggingHook::all_phases();
        assert_eq!(hook.name(), "logging");
        assert_eq!(hook.phases().len(), 8);

        let context = LifecycleContext::new(LifecyclePhase::Init, AgentState::Initializing);
        let result = hook.execute(&context).await.unwrap();
        assert!(matches!(result, HookResult::Continue));
    }

    #[tokio::test]
    async fn test_metrics_hook() {
        let hook = MetricsHook::new();
        assert_eq!(hook.name(), "metrics");
        assert_eq!(hook.priority(), -100);

        let context = LifecycleContext::new(LifecyclePhase::TaskStart, AgentState::Thinking)
            .with_task(TaskMetadata::new("Test", "/tmp"));
        let result = hook.execute(&context).await.unwrap();
        assert!(matches!(result, HookResult::Continue));
    }

    #[tokio::test]
    async fn test_unregister_hook() {
        let registry = LifecycleHookRegistry::new();

        let hook1 = Arc::new(CountingHook::new("hook1", vec![LifecyclePhase::Init]));
        let hook2 = Arc::new(CountingHook::new("hook2", vec![LifecyclePhase::Init]));

        registry.register(hook1).await;
        registry.register(hook2.clone()).await;
        assert_eq!(registry.count().await, 2);

        registry.unregister("hook1").await;
        assert_eq!(registry.count().await, 1);

        let context = LifecycleContext::new(LifecyclePhase::Init, AgentState::Initializing);
        registry
            .execute_hooks(LifecyclePhase::Init, context)
            .await
            .unwrap();

        assert_eq!(hook2.count(), 1);
    }

    #[test]
    fn test_lifecycle_error_display() {
        let err = LifecycleError::InitFailed("test".to_string());
        assert_eq!(err.to_string(), "Initialization failed: test");

        let err = LifecycleError::HookFailed {
            hook: LifecyclePhase::TaskStart,
            message: "failed".to_string(),
        };
        assert!(err.to_string().contains("task_start"));

        let err = LifecycleError::InvalidTransition {
            from: AgentState::Completed,
            to: AgentState::Thinking,
        };
        assert!(err.to_string().contains("Invalid state transition"));
    }

    #[test]
    fn test_lifecycle_phase_display() {
        assert_eq!(format!("{}", LifecyclePhase::Init), "init");
        assert_eq!(format!("{}", LifecyclePhase::TaskStart), "task_start");
        assert_eq!(format!("{}", LifecyclePhase::StepStart), "step_start");
        assert_eq!(format!("{}", LifecyclePhase::StepComplete), "step_complete");
        assert_eq!(format!("{}", LifecyclePhase::TaskComplete), "task_complete");
        assert_eq!(format!("{}", LifecyclePhase::Shutdown), "shutdown");
        assert_eq!(
            format!("{}", LifecyclePhase::StateTransition),
            "state_transition"
        );
        assert_eq!(format!("{}", LifecyclePhase::Error), "error");
    }
}
