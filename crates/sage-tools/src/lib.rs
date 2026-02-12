//! Tool implementations for Sage Agent
//!
//! This crate provides a comprehensive collection of tools for the Sage Agent system,
//! organized into functional categories for code manipulation, process execution,
//! file operations, and more.
//!
//! # Tool Categories
//!
//! ## File Operations ([`tools::file_ops`])
//! - **Read** - Read file contents with line numbers
//! - **Write** - Create or overwrite files
//! - **Edit** - Precise text replacement in files
//! - **Glob** - Pattern-based file discovery
//! - **Grep** - Content search with regex support
//! - **MultiEdit** - Batch file editing operations
//! - **JsonEdit** - JSON file manipulation with JSONPath
//!
//! ## Process Execution ([`tools::process`])
//! - **Bash** - Shell command execution with sandboxing
//! - **TaskOutput** - Retrieve background task results
//! - **KillShell** - Terminate running shell processes
//!
//! ## Code Intelligence
//! - **LSP** - Language Server Protocol integration (in [`tools::file_ops`])
//! - **TestGenerator** - Automated test generation (in [`tools::file_ops`])
//! - **CodebaseRetrieval** - Semantic code search (in [`tools::file_ops`])
//!
//! ## Network Operations ([`tools::network`])
//! - **WebFetch** - HTTP content fetching
//! - **WebSearch** - Web search integration
//! - **HttpClient** - Full-featured HTTP client
//!
//! ## Diagnostics ([`tools::diagnostics`])
//! - **Learning** - Pattern learning from user interactions
//! - **Memory** - Long-term memory management
//!
//! ## Extensions ([`tools::extensions`])
//! - **Skill** - Skill invocation
//! - **SlashCommand** - Slash command handling
//!
//! # Usage
//!
//! ```rust,ignore
//! use sage_tools::get_default_tools;
//!
//! // Get all default tools
//! let tools = get_default_tools();
//! ```

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

pub mod config;
pub mod mcp_tools;
pub mod tools;

// Re-export commonly used tools and functions
pub use tools::{
    // Task management
    AddTasksTool,
    // Interaction
    AskUserQuestionTool,
    // Process
    BashTool,
    // Network
    BrowserTool,
    // Infrastructure
    CloudTool,
    // File operations
    CodebaseRetrievalTool,
    // Extensions
    DeferredToolInfo,
    DeferredToolRegistry,
    // Diagnostics
    DiagnosticsTool,
    EditTool,
    // Planning
    EnterPlanModeTool,
    ExitPlanModeTool,
    // VCS
    GitTool,
    GlobTool,
    GrepTool,
    HttpClientTool,
    KillShellTool,
    KubernetesTool,
    LearnTool,
    LearningPatternsTool,
    // Monitoring
    LogAnalyzerTool,
    // Code intelligence
    LspTool,
    NotebookEditTool,
    PlatformToolProxy,
    ReadTool,
    RememberTool,
    RenderMermaidTool,
    ReorganizeTasklistTool,
    SearchUntruncatedTool,
    // Team
    SendMessageTool,
    // Utilities
    SequentialThinkingTool,
    SessionNotesTool,
    SkillTool,
    SlashCommandTool,
    TaskDoneTool,
    TaskOutputTool,
    TaskRequest,
    TaskStatus,
    TaskTool,
    TeamConfig,
    TeamManager,
    TeamMember,
    TeammateTool,
    TelemetryStatsTool,
    TerraformTool,
    TestGeneratorTool,
    TodoItem,
    TodoReadTool,
    TodoStatus,
    TodoWriteTool,
    ToolSearchResult,
    ToolSearchTool,
    UpdateTasksTool,
    ViewRangeUntruncatedTool,
    ViewTasklistTool,
    WebFetchTool,
    WebSearchTool,
    WriteTool,
    get_code_intelligence_tools,
    get_current_task,
    get_current_todos,
    // Factory functions
    get_default_tools,
    get_diagnostics_tools,
    get_extension_tools,
    get_file_ops_tools,
    get_global_learning_engine,
    get_global_memory_manager,
    get_infrastructure_tools,
    get_interaction_tools,
    get_learning_patterns_for_context,
    get_memories_for_context,
    get_monitoring_tools,
    get_network_tools,
    get_pending_tasks,
    get_planning_tools,
    get_process_tools,
    get_task,
    get_task_mgmt_tools,
    get_team_tools,
    get_todo_display,
    get_vcs_tools,
    init_global_learning_engine,
    init_global_memory_manager,
    update_task_status,
};
// Re-export MCP tools
pub use tools::{
    McpServersTool, McpToolAdapter, McpToolRegistry, SharedMcpToolRegistry, create_mcp_registry,
    get_global_mcp_registry, get_mcp_tools, init_global_mcp_registry,
};
