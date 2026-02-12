//! Agent system for Sage Agent

pub mod completion;
pub mod execution;
pub mod lifecycle;
pub mod options;
pub mod outcome;
pub mod state;
pub mod step;
pub mod subagent;
pub mod trait_impls;
pub mod traits;
pub mod unified;

pub use completion::{
    CompletionChecker, CompletionStatus, CompletionTaskType, FileOperationTracker, LimitType,
};
pub use execution::AgentExecution;
pub use lifecycle::{
    AgentLifecycle, HookResult, LifecycleContext, LifecycleError, LifecycleHook,
    LifecycleHookRegistry, LifecycleManager, LifecyclePhase, LifecycleResult, LoggingHook,
    MetricsHook,
};
pub use options::{AutoResponseConfig, ExecutionMode, ExecutionOptions};
pub use outcome::{ExecutionError, ExecutionErrorKind, ExecutionOutcome};
pub use state::AgentState;
pub use step::AgentStep;
pub use subagent::{
    AgentDefinition, AgentRegistry, AgentType, ToolAccessControl, get_builtin_agents,
    register_builtin_agents,
};
pub use unified::{
    ContextBuilder, ContextGitInfo, ProjectContext, UnifiedExecutor, UnifiedExecutorBuilder,
};

// Core trait abstractions for dependency injection and testability
pub use traits::{
    LlmService, NoopProgressReporter, NoopSessionRecorder, ProgressReporter,
    SessionRecorderService, ToolService, UserInteractionService,
};
