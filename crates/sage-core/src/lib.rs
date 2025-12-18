//! Sage Agent Core Library
//!
//! This crate provides the core functionality for the Sage Agent system,
//! including agent execution, LLM integration, tool management, and configuration.

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
    Agent, AgentExecution, AgentLifecycle, AgentState, AgentStep, ClaudeStyleAgent, HookResult,
    LifecycleContext, LifecycleError, LifecycleHook, LifecycleHookRegistry, LifecycleManager,
    LifecyclePhase, LifecycleResult, LoggingHook, MetricsHook, ReactiveAgent,
    ReactiveExecutionManager, ReactiveResponse,
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
pub use error::{SageError, SageResult};
pub use events::{Event, EventBus, SharedEventBus};
pub use hooks::{
    CallbackHook, CommandHook, HookConfig, HookEvent, HookExecutionResult, HookExecutor,
    HookImplementation, HookInput, HookMatcher, HookOutput, HookRegistry, HookType, HookVariant,
    HooksConfig, PermissionDecision, PromptHook,
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
    RateLimitError, RateLimitGuard, RateLimiter, RateLimiterConfig, RecoverableError, RecoveryError,
    RetryConfig, RetryPolicy, RetryResult, SlidingWindowRateLimiter, SupervisionPolicy,
    SupervisionResult, Supervisor, TaskSupervisor,
};
pub use sandbox::{
    DefaultSandbox, ResourceLimits, ResourceUsage, Sandbox, SandboxBuilder, SandboxConfig,
    SandboxError, SandboxMode, SandboxPolicy, SandboxResult, SandboxedExecution,
};
pub use tools::{
    BackgroundShellTask, BackgroundTaskRegistry, BackgroundTaskStatus, BackgroundTaskSummary,
    BatchStrategy, BatchToolExecutor, Tool, ToolCall, ToolExecutor, ToolResult, BACKGROUND_REGISTRY,
};
pub use trajectory::TrajectoryRecorder;
pub use types::*;
pub use session::{
    ConversationMessage, FileSessionStorage, MemorySessionStorage, MessageRole, Session,
    SessionConfig, SessionManager, SessionState, SessionStorage, SessionSummary,
    SessionToolCall, SessionToolResult, TokenUsage,
};
// Note: SessionId is re-exported from concurrency module
pub use settings::{
    HookDefinition as SettingsHookDefinition, HookDefinitionType as SettingsHookDefinitionType,
    HooksSettings, ModelSettings, ParsedPattern, PermissionBehavior, PermissionSettings, Settings,
    SettingsLoadInfo, SettingsLoader, SettingsLocations, SettingsSource, SettingsValidator,
    ToolSettings, UiSettings, ValidationResult as SettingsValidationResult, WorkspaceSettings,
};
pub use validation::{
    CommonRules, FieldError, FieldSchema, FieldType, InputSanitizer, RuleSet, SanitizeOptions,
    SchemaBuilder, ValidationError, ValidationResult, ValidationRule, ValidationSchema, Validator,
};
pub use checkpoints::{
    ChangeDetector, Checkpoint, CheckpointId, CheckpointManager, CheckpointManagerConfig,
    CheckpointStorage, CheckpointSummary, CheckpointType, ConversationSnapshot, DiffHunk, DiffLine,
    FileChange, FileCheckpointStorage, FileSnapshot, FileState, MemoryCheckpointStorage,
    RestoreOptions, RestorePreview, RestoreResult, TextDiff, TokenUsageSnapshot, ToolExecutionRecord,
};
pub use commands::{
    CommandArgument, CommandExecutor, CommandInvocation, CommandRegistry, CommandResult,
    CommandSource, SlashCommand,
};
pub use skills::{
    Skill, SkillActivation, SkillContext, SkillRegistry, SkillSource, SkillTrigger, TaskType,
    ToolAccess,
};
pub use output::{
    AssistantEvent, CostInfo, ErrorEvent, JsonFormatter, JsonOutput, OutputEvent, OutputFormat,
    OutputFormatter, OutputWriter, ResultEvent, StreamJsonFormatter, SystemEvent, TextFormatter,
    ToolCallResultEvent, ToolCallStartEvent, ToolCallSummary, UserPromptEvent, create_formatter,
};
pub use modes::{
    AgentMode, ModeExitResult, ModeManager, ModeState, ModeTransition, PlanModeConfig,
    PlanModeContext, ToolFilter,
};
pub use cost::{
    CostStatus, CostTracker, ModelPricing, ModelStats, PricingRegistry, ProviderStats, TokenPrice,
    TrackResult, UsageRecord, UsageStats,
};
pub use prompts::{BuiltinPrompts, PromptRegistry, PromptTemplate, PromptVariable, RenderError};
// New modular prompt system (Claude Code style)
pub use prompts::{
    SystemPromptBuilder, SystemPrompt, SystemReminder, PlanPhase,
    PromptVariables, TemplateRenderer, ToolDescriptions, AgentPrompts,
    GitPrompts, SecurityPolicy,
};
pub use telemetry::{
    Counter, Gauge, Histogram, HistogramData, HistogramTimer, LabeledCounter, Metric, MetricType,
    MetricValue, MetricsCollector, MetricsSnapshot, SharedMetricsCollector, create_metrics_collector,
};
pub use workspace::{
    AnalysisResult, BuildSystem, DependencyInfo, EntryPoint, FileStats, FrameworkType,
    ImportantFile, ImportantFileType, LanguageType, PatternMatcher, ProjectPattern, ProjectStructure,
    ProjectType, ProjectTypeDetector, RuntimeType, TestFramework, WorkspaceAnalyzer, WorkspaceConfig,
    WorkspaceError,
};
pub use memory::{
    FileMemoryStorage, Memory, MemoryCategory, MemoryConfig, MemoryId, MemoryManager,
    MemoryMetadata, MemoryQuery, MemoryScore, MemorySource, MemoryStats, MemoryStorage,
    MemoryStorageError, MemoryType, RelevanceScore, SharedMemoryManager, create_memory_manager,
};
pub use learning::{
    Confidence, CorrectionRecord, CorrectionStats, LearningConfig, LearningEngine, LearningError,
    LearningEvent, LearningEventType, LearningStats, Pattern, PatternDetector, PatternId,
    PatternSource, PatternType, PreferenceIndicator, SharedLearningEngine, StylePattern,
    analyze_user_message, create_learning_engine, create_learning_engine_with_memory,
};
pub use storage::{
    BackendType, ConnectionPool, ConnectionStatus, DatabaseBackend, DatabaseError, DatabaseRow,
    DatabaseValue, FallbackStrategy, Migration, MigrationRunner, PostgresBackend, PostgresConfig,
    QueryResult, SchemaVersion, SharedStorageManager, SqliteBackend, SqliteConfig,
    StorageConfig, StorageManager, StorageStats, create_storage_manager,
    RetryConfig as StorageRetryConfig,
};
