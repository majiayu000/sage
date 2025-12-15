//! Sage Agent Core Library
//!
//! This crate provides the core functionality for the Sage Agent system,
//! including agent execution, LLM integration, tool management, and configuration.

pub mod agent;
pub mod builder;
pub mod cache;
pub mod concurrency;
pub mod config;
pub mod error;
pub mod events;
pub mod interrupt;
pub mod llm;
pub mod mcp;
pub mod plugins;
pub mod recovery;
pub mod sandbox;
pub mod tools;
pub mod trajectory;
pub mod types;
pub mod ui;
pub mod validation;

// Plugin system is implemented in plugins module
// TODO: Add plugin marketplace integration

// Re-export commonly used types
pub use agent::{
    Agent, AgentExecution, AgentLifecycle, AgentState, AgentStep, ClaudeStyleAgent, HookResult,
    LifecycleContext, LifecycleError, LifecycleHook, LifecycleHookRegistry, LifecycleManager,
    LifecyclePhase, LifecycleResult, LoggingHook, MetricsHook, ReactiveAgent,
    ReactiveExecutionManager, ReactiveResponse,
};
pub use builder::{BuilderError, SageBuilder, SageComponents};
pub use cache::{CacheManager, LLMCache, CacheKey, CacheEntry, CacheConfig};
pub use concurrency::{CancellationHierarchy, SessionId, AgentId, ToolCallId, SharedCancellationHierarchy};
pub use config::{Config, ModelParameters, LakeviewConfig};
pub use error::{SageError, SageResult};
pub use events::{Event, EventBus, SharedEventBus};
pub use interrupt::{InterruptManager, InterruptReason, TaskScope};
pub use llm::{LLMClient, LLMMessage, LLMResponse, LLMProvider};
pub use mcp::{McpClient, McpError, McpRegistry, McpTool, McpResource, StdioTransport};
pub use recovery::{
    BackoffConfig, BackoffStrategy, CircuitBreaker, CircuitBreakerConfig, CircuitState,
    ErrorClass, RecoverableError, RecoveryError, RetryConfig, RetryPolicy, RetryResult,
    SupervisionPolicy, SupervisionResult, Supervisor, TaskSupervisor,
};
pub use tools::{Tool, ToolCall, ToolExecutor, ToolResult, BatchToolExecutor, BatchStrategy};
pub use sandbox::{
    DefaultSandbox, ResourceLimits, ResourceUsage, Sandbox, SandboxBuilder, SandboxConfig,
    SandboxError, SandboxMode, SandboxPolicy, SandboxResult, SandboxedExecution,
};
pub use trajectory::TrajectoryRecorder;
pub use types::*;
pub use validation::{
    CommonRules, FieldError, FieldSchema, FieldType, InputSanitizer, RuleSet, SanitizeOptions,
    SchemaBuilder, ValidationError, ValidationResult, ValidationRule, ValidationSchema, Validator,
};
pub use plugins::{
    Plugin, PluginCapability, PluginContext, PluginEntry, PluginError, PluginInfo,
    PluginLifecycle, PluginManifest, PluginPermission, PluginRegistry, PluginResult, PluginState,
};
