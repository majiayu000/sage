//! Agent system for Sage Agent

pub mod base;
pub mod completion;
pub mod execution;
pub mod lifecycle;
pub mod outcome;
pub mod reactive_agent;
pub mod state;
pub mod step;
pub mod subagent;

pub use base::Agent;
pub use completion::{
    CompletionChecker, CompletionStatus, FileOperationTracker, LimitType, TaskType,
};
pub use execution::AgentExecution;
pub use lifecycle::{
    AgentLifecycle, HookResult, LifecycleContext, LifecycleError, LifecycleHook,
    LifecycleHookRegistry, LifecycleManager, LifecyclePhase, LifecycleResult, LoggingHook,
    MetricsHook,
};
pub use outcome::{ExecutionError, ExecutionErrorKind, ExecutionOutcome};
pub use reactive_agent::{
    ClaudeStyleAgent, ReactiveAgent, ReactiveExecutionManager, ReactiveResponse,
};
pub use state::AgentState;
pub use step::AgentStep;
pub use subagent::{
    AgentDefinition, AgentRegistry, AgentType, ToolAccessControl, get_builtin_agents,
    register_builtin_agents,
};
