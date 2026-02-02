//! Sage Agent Tools
//!
//! This module contains all the tools available to the Sage Agent, organized by category:
//!
//! - `file_ops`: File and code operations (edit, json_edit, codebase_retrieval, notebook_edit)
//! - `process`: Process and terminal tools (bash, kill_shell)
//! - `task_mgmt`: Task management tools (task_management, reorganize_tasklist, task_done)
//! - `planning`: Planning mode tools (enter_plan_mode, exit_plan_mode)
//! - `interaction`: User interaction tools (ask_user_question)
//! - `extensions`: Extension tools (skill, slash_command, tool_search)
//! - `utils`: Utility tools (sequential_thinking, monitoring, enhanced_errors)
//! - `network`: Network and browser tools (web_search, web_fetch, browser)
//! - `diagnostics`: Diagnostics and content processing tools
//! - `vcs`: Version control system tools (git)
//! - `monitoring`: Monitoring tools (log_analyzer, test_generator)
//! - `infrastructure`: Infrastructure tools (kubernetes, terraform, cloud)
//! - `code_intelligence`: Code intelligence tools (lsp)
//! - `team`: Team collaboration tools (teammate, send_message)

pub mod code_intelligence;
pub mod diagnostics;
pub mod extensions;
pub mod file_ops;
pub mod infrastructure;
pub mod interaction;
pub mod monitoring;
pub mod planning;
pub mod process;
pub mod task_mgmt;
pub mod team;
pub mod utils;

// VCS module with only updated git_simple
pub mod vcs {
    pub mod git_simple;
    pub use git_simple::GitTool;
}

// Network module with only working tools
pub mod network {
    pub mod browser;
    pub mod http_client;
    pub mod validation;
    pub mod web_fetch;
    pub mod web_search;

    pub use browser::BrowserTool;
    pub use http_client::HttpClientTool;
    pub use validation::validate_url_security;
    pub use web_fetch::WebFetchTool;
    pub use web_search::WebSearchTool;
}

// Re-export all tools for easy access
// Note: JsonEditTool, MultiEditTool are Sage-specific and currently disabled
pub use code_intelligence::LspTool;
pub use diagnostics::{
    DiagnosticsTool, LearnTool, LearningPatternsTool, RememberTool, RenderMermaidTool,
    SearchUntruncatedTool, SessionNotesTool, ViewRangeUntruncatedTool, get_global_learning_engine,
    get_global_memory_manager, get_learning_patterns_for_context, get_memories_for_context,
    init_global_learning_engine, init_global_memory_manager,
};
pub use extensions::{
    DeferredToolInfo, DeferredToolRegistry, PlatformToolProxy, SkillTool, SlashCommandTool,
    ToolSearchResult, ToolSearchTool,
};
pub use file_ops::{
    CodebaseRetrievalTool, EditTool, GlobTool, GrepTool, NotebookEditTool, ReadTool, WriteTool,
};
pub use infrastructure::{CloudTool, KubernetesTool, TerraformTool};
pub use interaction::AskUserQuestionTool;
pub use monitoring::{LogAnalyzerTool, TestGeneratorTool};
pub use network::{BrowserTool, HttpClientTool, WebFetchTool, WebSearchTool};
pub use planning::{EnterPlanModeTool, ExitPlanModeTool};
pub use process::{
    BashTool, KillShellTool, TaskOutputTool, TaskRequest, TaskStatus, TaskTool, get_pending_tasks,
    get_task, update_task_status,
};
pub use task_mgmt::{
    AddTasksTool, ReorganizeTasklistTool, TaskDoneTool, TodoItem, TodoReadTool, TodoStatus,
    TodoWriteTool, UpdateTasksTool, ViewTasklistTool, get_current_task, get_current_todos,
    get_todo_display,
};
pub use team::{SendMessageTool, TeamConfig, TeamManager, TeamMember, TeammateTool};
pub use utils::{SequentialThinkingTool, TelemetryStatsTool};
pub use vcs::GitTool;

// Re-export MCP tools
pub use crate::mcp_tools::{
    McpServersTool, McpToolAdapter, McpToolRegistry, SharedMcpToolRegistry, create_mcp_registry,
    get_global_mcp_registry, get_mcp_tools, init_global_mcp_registry,
};

use sage_core::tools::Tool;
use std::sync::Arc;

/// Get all default tools organized by category
pub fn get_default_tools() -> Vec<Arc<dyn Tool>> {
    vec![
        // File operations
        Arc::new(EditTool::new()),
        Arc::new(ReadTool::new()),
        Arc::new(WriteTool::new()),
        Arc::new(GlobTool::new()),
        Arc::new(GrepTool::new()),
        Arc::new(NotebookEditTool::new()),
        Arc::new(CodebaseRetrievalTool::new()), // Smart codebase search
        // Disabled Sage-specific tools: JsonEditTool, MultiEditTool

        // Process tools
        Arc::new(BashTool::new()),
        Arc::new(KillShellTool::new()),
        Arc::new(TaskTool::new()), // Claude Code compatible subagent spawning
        Arc::new(TaskOutputTool::new()),
        // Task management
        Arc::new(TodoWriteTool::new()), // Claude Code compatible
        Arc::new(TodoReadTool::new()),  // Read current todo list status
        Arc::new(ViewTasklistTool::new()),
        Arc::new(AddTasksTool::new()),
        Arc::new(UpdateTasksTool::new()),
        Arc::new(ReorganizeTasklistTool::new()),
        Arc::new(TaskDoneTool::new()),
        // Planning mode
        Arc::new(EnterPlanModeTool::new()),
        Arc::new(ExitPlanModeTool::new()),
        // User interaction
        Arc::new(AskUserQuestionTool::new()),
        // Extensions
        Arc::new(SkillTool::new()),
        Arc::new(SlashCommandTool::new()),
        Arc::new(ToolSearchTool::new()), // Claude Code compatible deferred tool search
        // Platform tool proxies (for LLM platform built-in tools)
        Arc::new(PlatformToolProxy::glm_claim_coupon()),
        // Utilities
        Arc::new(SequentialThinkingTool::new()),
        Arc::new(TelemetryStatsTool::new()), // View tool usage statistics
        // Network tools
        Arc::new(WebSearchTool::new()),
        Arc::new(WebFetchTool::new()),
        Arc::new(BrowserTool::new()),
        Arc::new(HttpClientTool::new()),
        // Diagnostics
        Arc::new(DiagnosticsTool::new()),
        Arc::new(ViewRangeUntruncatedTool::new()),
        Arc::new(SearchUntruncatedTool::new()),
        Arc::new(RememberTool::new()),
        Arc::new(SessionNotesTool::new()),
        Arc::new(RenderMermaidTool::new()),
        // Learning mode
        Arc::new(LearnTool::new()),
        Arc::new(LearningPatternsTool::new()),
        // MCP server management
        Arc::new(McpServersTool::new()),
        // VCS
        Arc::new(GitTool::new()),
        // Monitoring
        Arc::new(LogAnalyzerTool::new()),
        Arc::new(TestGeneratorTool::new()),
        // Infrastructure
        Arc::new(KubernetesTool::new()),
        Arc::new(TerraformTool::new()),
        Arc::new(CloudTool::new()),
        // Code intelligence
        Arc::new(LspTool::new()),
        // Team collaboration
        Arc::new(TeammateTool::new()),
        Arc::new(SendMessageTool::new()),
    ]
}

/// Get tools by category
pub fn get_file_ops_tools() -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(EditTool::new()),
        Arc::new(ReadTool::new()),
        Arc::new(WriteTool::new()),
        Arc::new(GlobTool::new()),
        Arc::new(GrepTool::new()),
        Arc::new(NotebookEditTool::new()),
        Arc::new(CodebaseRetrievalTool::new()),
        // Disabled Sage-specific tools: JsonEditTool, MultiEditTool
    ]
}

pub fn get_process_tools() -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(BashTool::new()),
        Arc::new(KillShellTool::new()),
        Arc::new(TaskOutputTool::new()),
    ]
}

pub fn get_task_mgmt_tools() -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(ViewTasklistTool::new()),
        Arc::new(AddTasksTool::new()),
        Arc::new(UpdateTasksTool::new()),
        Arc::new(ReorganizeTasklistTool::new()),
        Arc::new(TaskDoneTool::new()),
    ]
}

pub fn get_planning_tools() -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(EnterPlanModeTool::new()),
        Arc::new(ExitPlanModeTool::new()),
    ]
}

pub fn get_interaction_tools() -> Vec<Arc<dyn Tool>> {
    vec![Arc::new(AskUserQuestionTool::new())]
}

pub fn get_extension_tools() -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(SkillTool::new()),
        Arc::new(SlashCommandTool::new()),
    ]
}

pub fn get_network_tools() -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(WebSearchTool::new()),
        Arc::new(WebFetchTool::new()),
        Arc::new(BrowserTool::new()),
        Arc::new(HttpClientTool::new()),
    ]
}

pub fn get_diagnostics_tools() -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(DiagnosticsTool::new()),
        Arc::new(ViewRangeUntruncatedTool::new()),
        Arc::new(SearchUntruncatedTool::new()),
        Arc::new(RememberTool::new()),
        Arc::new(SessionNotesTool::new()),
        Arc::new(RenderMermaidTool::new()),
        Arc::new(LearnTool::new()),
        Arc::new(LearningPatternsTool::new()),
    ]
}

pub fn get_vcs_tools() -> Vec<Arc<dyn Tool>> {
    vec![Arc::new(GitTool::new())]
}

pub fn get_monitoring_tools() -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(LogAnalyzerTool::new()),
        Arc::new(TestGeneratorTool::new()),
    ]
}

pub fn get_infrastructure_tools() -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(KubernetesTool::new()),
        Arc::new(TerraformTool::new()),
        Arc::new(CloudTool::new()),
    ]
}

pub fn get_code_intelligence_tools() -> Vec<Arc<dyn Tool>> {
    vec![Arc::new(LspTool::new())]
}

pub fn get_team_tools() -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(TeammateTool::new()),
        Arc::new(SendMessageTool::new()),
    ]
}

#[cfg(test)]
pub mod tests;
