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
    // File operations
    CodebaseRetrievalTool, EditTool, GlobTool, GrepTool, NotebookEditTool, ReadTool, WriteTool,
    // Process
    BashTool, KillShellTool, TaskOutputTool, TaskRequest, TaskStatus, TaskTool,
    get_pending_tasks, get_task, update_task_status,
    // Task management
    AddTasksTool, ReorganizeTasklistTool, TaskDoneTool, TodoItem, TodoReadTool, TodoStatus,
    TodoWriteTool, UpdateTasksTool, ViewTasklistTool,
    get_current_task, get_current_todos, get_todo_display,
    // Planning
    EnterPlanModeTool, ExitPlanModeTool,
    // Interaction
    AskUserQuestionTool,
    // Extensions
    DeferredToolInfo, DeferredToolRegistry, PlatformToolProxy, SkillTool, SlashCommandTool,
    ToolSearchResult, ToolSearchTool,
    // Network
    BrowserTool, HttpClientTool, WebFetchTool, WebSearchTool,
    // Diagnostics
    DiagnosticsTool, LearnTool, LearningPatternsTool, RememberTool, RenderMermaidTool,
    SearchUntruncatedTool, SessionNotesTool, ViewRangeUntruncatedTool,
    get_global_learning_engine, get_global_memory_manager, get_learning_patterns_for_context,
    get_memories_for_context, init_global_learning_engine, init_global_memory_manager,
    // Infrastructure
    CloudTool, KubernetesTool, TerraformTool,
    // Monitoring
    LogAnalyzerTool, TestGeneratorTool,
    // Code intelligence
    LspTool,
    // VCS
    GitTool,
    // Team
    SendMessageTool, TeamConfig, TeamManager, TeamMember, TeammateTool,
    // Utilities
    SequentialThinkingTool, TelemetryStatsTool,
    // Factory functions
    get_default_tools, get_file_ops_tools, get_process_tools, get_task_mgmt_tools,
    get_planning_tools, get_interaction_tools, get_extension_tools, get_network_tools,
    get_diagnostics_tools, get_vcs_tools, get_monitoring_tools, get_infrastructure_tools,
    get_code_intelligence_tools, get_team_tools,
};
// Re-export MCP tools
pub use tools::{
    McpServersTool, McpToolAdapter, McpToolRegistry, SharedMcpToolRegistry,
    create_mcp_registry, get_global_mcp_registry, get_mcp_tools, init_global_mcp_registry,
};
