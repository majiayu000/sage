//! Sage Agent Core Library
//!
//! This crate provides the core functionality for the Sage Agent system,
//! including agent execution, LLM integration, tool management, and configuration.
//!
//! # Overview
//!
//! `sage-core` is the foundational library for the Sage Agent ecosystem. It provides
//! low-level components for building AI agents that can interact with LLMs, execute
//! tools, manage state, and handle complex workflows.
//!
//! # Key Components
//!
//! ## Agent Execution
//!
//! The [`agent`] module provides the core agent execution engine:
//!
//! - [`Agent`] - Main trait for agent implementations
//! - [`AgentExecution`] - Tracks execution state across multiple steps
//! - [`ExecutionMode`] - Controls interactive vs. non-interactive execution
//! - [`UnifiedExecutor`] - Unified execution loop (Claude Code style)
//!
//! ## LLM Integration
//!
//! The [`llm`] module provides multi-provider LLM support:
//!
//! - [`LLMClient`] - Async interface for LLM interactions
//! - [`LLMProvider`] - Abstraction over different providers (Anthropic, OpenAI, Google)
//! - [`LLMMessage`] - Unified message format
//!
//! ## Tool System
//!
//! The [`tools`] module provides a flexible tool execution framework:
//!
//! - [`Tool`] - Trait for defining custom tools
//! - [`ToolExecutor`] - Manages tool execution with safety and sandboxing
//! - [`ToolCall`] and [`ToolResult`] - Tool invocation types
//!
//! ## Configuration
//!
//! The [`config`] module handles configuration loading and validation:
//!
//! - [`Config`] - Main configuration structure
//! - [`ModelParameters`] - LLM model configuration
//! - Environment variable substitution
//!
//! ## Additional Features
//!
//! - **Memory Management** ([`memory`]) - Long-term memory and context management
//! - **Trajectory Recording** ([`trajectory`]) - Execution history tracking
//! - **Plugin System** ([`plugins`]) - Extensibility through plugins
//! - **Skills** ([`skills`]) - High-level task-specific capabilities
//! - **MCP Integration** ([`mcp`]) - Model Context Protocol support
//! - **Checkpoints** ([`checkpoints`]) - State snapshots and restoration
//! - **Recovery** ([`recovery`]) - Error handling and retry policies
//! - **Telemetry** ([`telemetry`]) - Metrics collection and monitoring
//!
//! # Examples
//!
//! For high-level usage, prefer using the [`sage-sdk`](https://docs.rs/sage-sdk) crate.
//! This core library is intended for advanced use cases and custom integrations.
//!
//! ## Basic Agent Creation
//!
//! ```no_run
//! use sage_core::{agent::base::BaseAgent, config::Config};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = Config::default();
//! let agent = BaseAgent::new(config)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Custom Tool Implementation
//!
//! ```no_run
//! use sage_core::tools::{Tool, ToolCall, ToolResult};
//! use async_trait::async_trait;
//!
//! struct CustomTool;
//!
//! #[async_trait]
//! impl Tool for CustomTool {
//!     fn name(&self) -> &str { "custom_tool" }
//!     fn description(&self) -> &str { "A custom tool" }
//!
//!     async fn execute(&self, call: &ToolCall) -> ToolResult {
//!         // Implementation
//!         ToolResult::success("Result")
//!     }
//! }
//! ```
//!
//! # Feature Flags
//!
//! This crate does not currently use feature flags, but future versions may add
//! optional features for specific providers or advanced capabilities.

// Allow common clippy lints that are stylistic preferences
#![allow(clippy::collapsible_if)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::unnecessary_map_or)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::manual_range_patterns)]
#![allow(clippy::ptr_arg)]
#![allow(clippy::single_char_add_str)]
#![allow(clippy::option_map_or_none)]
#![allow(clippy::match_like_matches_macro)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::filter_map_identity)]

pub mod agent;
pub mod builder;
pub mod cache;
pub mod checkpoints;
pub mod commands;
pub mod concurrency;
pub mod config;
pub mod context;
pub mod cost;
pub mod error;
pub mod events;
pub mod hooks;
pub mod input;
pub mod interrupt;
pub mod learning;
pub mod llm;
pub mod mcp;
pub mod memory;
pub mod modes;
pub mod output;
pub mod plugins;
pub mod prompts;
pub mod recovery;
pub mod sandbox;
pub mod session;
pub mod settings;
pub mod skills;
pub mod storage;
pub mod telemetry;
pub mod tools;
pub mod trajectory;
pub mod types;
pub mod ui;
pub mod validation;
pub mod workspace;

// Plugin system is implemented in plugins module
// TODO: Add plugin marketplace integration

// Re-export commonly used types
pub use agent::{
    Agent, AgentExecution, AgentLifecycle, AgentState, AgentStep, AutoResponse, ClaudeStyleAgent,
    ExecutionMode, ExecutionOptions, HookResult, LifecycleContext, LifecycleError, LifecycleHook,
    LifecycleHookRegistry, LifecycleManager, LifecyclePhase, LifecycleResult, LoggingHook,
    MetricsHook, ReactiveAgent, ReactiveExecutionManager, ReactiveResponse, UnifiedExecutor,
    UnifiedExecutorBuilder,
};
pub use builder::{BuilderError, SageBuilder, SageComponents};
pub use cache::{CacheConfig, CacheEntry, CacheKey, CacheManager, LLMCache};
pub use concurrency::{
    AgentId, CancellationHierarchy, SessionId, SharedCancellationHierarchy, ToolCallId,
};
pub use config::{Config, LakeviewConfig, ModelParameters};
pub use context::{
    AggregatedStats, ContextConfig, ContextManager, ContextUsageStats, ConversationSummarizer,
    MessagePruner, OverflowStrategy, PrepareResult, PruneResult, SharedStreamingMetrics,
    StreamingMetrics, StreamingStats, StreamingTokenCounter, TokenEstimator,
};
pub use error::{OptionExt, ResultExt, SageError, SageResult};
pub use events::{Event, EventBus, SharedEventBus};
pub use hooks::{
    CallbackHook, CommandHook, HookConfig, HookEvent, HookExecutionResult, HookExecutor,
    HookImplementation, HookInput, HookMatcher, HookOutput, HookRegistry, HookType, HookVariant,
    HooksConfig, PermissionDecision, PromptHook,
};
pub use input::{
    InputChannel, InputChannelHandle, InputContext, InputOption, InputRequest, InputResponse,
};
pub use interrupt::{InterruptManager, InterruptReason, TaskScope};
pub use llm::{LLMClient, LLMMessage, LLMProvider, LLMResponse};
pub use mcp::{McpClient, McpError, McpRegistry, McpResource, McpTool, StdioTransport};
pub use plugins::{
    Plugin, PluginCapability, PluginContext, PluginEntry, PluginError, PluginInfo, PluginLifecycle,
    PluginManifest, PluginPermission, PluginRegistry, PluginResult, PluginState,
};
pub use recovery::{
    BackoffConfig, BackoffStrategy, CircuitBreaker, CircuitBreakerConfig, CircuitState, ErrorClass,
    RateLimitError, RateLimitGuard, RateLimiter, RateLimiterConfig, RecoverableError,
    RecoveryError, RetryConfig, RetryPolicy, RetryResult, SlidingWindowRateLimiter,
    SupervisionPolicy, SupervisionResult, Supervisor, TaskSupervisor,
};
pub use sandbox::{
    DefaultSandbox, ResourceLimits, ResourceUsage, Sandbox, SandboxBuilder, SandboxConfig,
    SandboxError, SandboxMode, SandboxPolicy, SandboxResult, SandboxedExecution,
};
pub use session::{
    ConversationMessage, FileSessionStorage, MemorySessionStorage, MessageRole, Session,
    SessionConfig, SessionManager, SessionState, SessionStorage, SessionSummary, SessionToolCall,
    SessionToolResult, TokenUsage,
};
pub use tools::{
    BACKGROUND_REGISTRY, BackgroundShellTask, BackgroundTaskRegistry, BackgroundTaskStatus,
    BackgroundTaskSummary, BatchStrategy, BatchToolExecutor, Tool, ToolCall, ToolExecutor,
    ToolResult,
};
pub use trajectory::TrajectoryRecorder;
pub use types::*;
// Note: SessionId is re-exported from concurrency module
pub use checkpoints::{
    ChangeDetector, Checkpoint, CheckpointId, CheckpointManager, CheckpointManagerConfig,
    CheckpointStorage, CheckpointSummary, CheckpointType, ConversationSnapshot, DiffHunk, DiffLine,
    FileChange, FileCheckpointStorage, FileSnapshot, FileState, MemoryCheckpointStorage,
    RestoreOptions, RestorePreview, RestoreResult, TextDiff, TokenUsageSnapshot,
    ToolExecutionRecord,
};
pub use commands::{
    CommandArgument, CommandExecutor, CommandInvocation, CommandRegistry, CommandResult,
    CommandSource, SlashCommand,
};
pub use cost::{
    CostStatus, CostTracker, ModelPricing, ModelStats, PricingRegistry, ProviderStats, TokenPrice,
    TrackResult, UsageRecord, UsageStats,
};
pub use modes::{
    AgentMode, ModeExitResult, ModeManager, ModeState, ModeTransition, PlanModeConfig,
    PlanModeContext, ToolFilter,
};
pub use output::{
    AssistantEvent, CostInfo, ErrorEvent, JsonFormatter, JsonOutput, OutputEvent, OutputFormat,
    OutputFormatter, OutputWriter, ResultEvent, StreamJsonFormatter, SystemEvent, TextFormatter,
    ToolCallResultEvent, ToolCallStartEvent, ToolCallSummary, UserPromptEvent, create_formatter,
};
pub use prompts::{BuiltinPrompts, PromptRegistry, PromptTemplate, PromptVariable, RenderError};
pub use settings::{
    HookDefinition as SettingsHookDefinition, HookDefinitionType as SettingsHookDefinitionType,
    HooksSettings, ModelSettings, ParsedPattern, PermissionBehavior, PermissionSettings, Settings,
    SettingsLoadInfo, SettingsLoader, SettingsLocations, SettingsSource, SettingsValidator,
    ToolSettings, UiSettings, ValidationResult as SettingsValidationResult, WorkspaceSettings,
};
pub use skills::{
    Skill, SkillActivation, SkillContext, SkillRegistry, SkillSource, SkillTrigger, TaskType,
    ToolAccess,
};
pub use validation::{
    CommonRules, FieldError, FieldSchema, FieldType, InputSanitizer, RuleSet, SanitizeOptions,
    SchemaBuilder, ValidationError, ValidationResult, ValidationRule, ValidationSchema, Validator,
};
// New modular prompt system (Claude Code style)
pub use learning::{
    Confidence, CorrectionRecord, CorrectionStats, LearningConfig, LearningEngine, LearningError,
    LearningEvent, LearningEventType, LearningStats, Pattern, PatternDetector, PatternId,
    PatternSource, PatternType, PreferenceIndicator, SharedLearningEngine, StylePattern,
    analyze_user_message, create_learning_engine, create_learning_engine_with_memory,
};
pub use memory::{
    FileMemoryStorage, Memory, MemoryCategory, MemoryConfig, MemoryId, MemoryManager,
    MemoryMetadata, MemoryQuery, MemoryScore, MemorySource, MemoryStats, MemoryStorage,
    MemoryStorageError, MemoryType, RelevanceScore, SharedMemoryManager, create_memory_manager,
};
pub use prompts::{
    AgentPrompts, GitPrompts, PlanPhase, PromptVariables, SecurityPolicy, SystemPrompt,
    SystemPromptBuilder, SystemReminder, TemplateRenderer, ToolDescriptions,
};
pub use storage::{
    BackendType, ConnectionPool, ConnectionStatus, DatabaseBackend, DatabaseError, DatabaseRow,
    DatabaseValue, FallbackStrategy, Migration, MigrationRunner, PostgresBackend, PostgresConfig,
    QueryResult, RetryConfig as StorageRetryConfig, SchemaVersion, SharedStorageManager,
    SqliteBackend, SqliteConfig, StorageConfig, StorageManager, StorageStats,
    create_storage_manager,
};
pub use telemetry::{
    Counter, Gauge, Histogram, HistogramData, HistogramTimer, LabeledCounter, Metric, MetricType,
    MetricValue, MetricsCollector, MetricsSnapshot, SharedMetricsCollector, TelemetryCollector,
    TelemetrySummary, ToolStats, ToolUsageEvent, create_metrics_collector, global_telemetry,
};
pub use workspace::{
    AnalysisResult, BuildSystem, DependencyInfo, EntryPoint, FileStats, FrameworkType, GitInfo,
    ImportantFile, ImportantFileType, LanguageType, PatternMatcher, ProjectPattern,
    ProjectStructure, ProjectType, ProjectTypeDetector, RuntimeType, TestFramework,
    WorkspaceAnalyzer, WorkspaceConfig, WorkspaceError,
};
