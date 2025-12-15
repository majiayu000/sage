//! Agent system for Sage Agent

pub mod base;
pub mod execution;
pub mod lifecycle;
pub mod outcome;
pub mod reactive_agent;
pub mod state;
pub mod step;

pub use base::Agent;
pub use execution::AgentExecution;
pub use outcome::{ExecutionError, ExecutionErrorKind, ExecutionOutcome};
pub use lifecycle::{
    AgentLifecycle, HookResult, LifecycleContext, LifecycleError, LifecycleHook,
    LifecycleHookRegistry, LifecycleManager, LifecyclePhase, LifecycleResult, LoggingHook,
    MetricsHook,
};
pub use reactive_agent::{ReactiveAgent, ReactiveResponse, ClaudeStyleAgent, ReactiveExecutionManager};
pub use state::AgentState;
pub use step::AgentStep;
