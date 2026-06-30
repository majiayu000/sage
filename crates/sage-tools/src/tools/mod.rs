//! Sage Agent Tools
//!
//! This module contains all the tools available to the Sage Agent, organized by category:
//!
//! - `file_ops`: File and code operations (edit, notebook_edit)
//! - `process`: Process and terminal tools (bash, kill_shell)
//! - `task_mgmt`: Task management tools (TodoWrite, TodoRead, TaskDone)
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
    pub(super) mod redirect;
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
    EditTool, FileAccessTracker, GlobTool, GrepTool, NotebookEditTool, ReadTool, WriteTool,
};
pub use infrastructure::{CloudTool, KubernetesTool, TerraformTool};
pub use interaction::AskUserQuestionTool;
pub use monitoring::{LogAnalyzerTool, TestGeneratorTool};
pub use network::{BrowserTool, HttpClientTool, WebFetchTool, WebSearchTool};
pub use planning::{EnterPlanModeTool, ExitPlanModeTool};
pub use process::{
    AgentLifecycleTool, BashTool, KillShellTool, TaskOutputTool, TaskRequest, TaskStatus, TaskTool,
    get_pending_tasks, get_task, update_task_status,
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

use once_cell::sync::Lazy;
use parking_lot::Mutex;
use sage_core::agent::subagent::SubAgentGraph;
use sage_core::skills::SkillRegistry;
use sage_core::thread_store::ThreadStore;
use sage_core::tools::Tool;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

const DEFAULT_TOOL_NAMES: &[&str] = &[
    "Edit",
    "Read",
    "Write",
    "Glob",
    "Grep",
    "NotebookEdit",
    "Bash",
    "KillShell",
    "Task",
    "TaskOutput",
    "TodoWrite",
    "TodoRead",
    "TaskDone",
    "EnterPlanMode",
    "ExitPlanMode",
    "AskUserQuestion",
    "Skill",
    "SlashCommand",
    "ToolSearch",
    "claim_glm_camp_coupon",
    "SequentialThinking",
    "TelemetryStats",
    "WebSearch",
    "WebFetch",
    "OpenBrowser",
    "http_client",
    "Diagnostics",
    "ViewRangeUntruncated",
    "SearchUntruncated",
    "Remember",
    "SessionNotes",
    "RenderMermaid",
    "Learn",
    "LearningPatterns",
    "McpServers",
    "git",
    "log_analyzer",
    "test_generator",
    "kubernetes",
    "terraform",
    "cloud",
    "LSP",
    "TeammateTool",
    "SendMessageTool",
];

static GRAPH_TASK_REGISTRIES: Lazy<Mutex<HashMap<String, Arc<process::task::TaskRegistry>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

struct DefaultToolConfig {
    working_directory: PathBuf,
    skill_registry: Arc<RwLock<SkillRegistry>>,
    file_access_tracker: Arc<FileAccessTracker>,
    thread_store: Option<Arc<dyn ThreadStore>>,
}

impl DefaultToolConfig {
    fn new(
        working_directory: impl Into<PathBuf>,
        skill_registry: Arc<RwLock<SkillRegistry>>,
    ) -> Self {
        Self {
            working_directory: working_directory.into(),
            skill_registry,
            file_access_tracker: Arc::new(FileAccessTracker::new()),
            thread_store: None,
        }
    }

    fn with_thread_store(mut self, thread_store: Arc<dyn ThreadStore>) -> Self {
        self.thread_store = Some(thread_store);
        self
    }

    fn with_new_skill_registry(working_directory: impl Into<PathBuf>) -> Self {
        let working_directory = working_directory.into();
        let mut registry = SkillRegistry::new(&working_directory);
        registry.register_builtins();
        Self::new(working_directory, Arc::new(RwLock::new(registry)))
    }
}

fn build_default_tools(config: DefaultToolConfig) -> Vec<Arc<dyn Tool>> {
    let working_directory = config.working_directory;
    let file_access_tracker = config.file_access_tracker;
    let thread_store = config.thread_store;
    let task_registry = thread_store
        .as_ref()
        .map(graph_task_registry)
        .unwrap_or_else(|| process::task::GLOBAL_TASK_REGISTRY.clone());
    let subagent_graph =
        thread_store.map(|thread_store| Arc::new(SubAgentGraph::new(thread_store)));
    let mut tools: Vec<Arc<dyn Tool>> = vec![
        // File operations
        Arc::new(EditTool::with_working_directory_and_tracker(
            working_directory.clone(),
            Arc::clone(&file_access_tracker),
        )),
        Arc::new(ReadTool::with_working_directory_and_tracker(
            working_directory.clone(),
            Arc::clone(&file_access_tracker),
        )),
        Arc::new(WriteTool::with_working_directory_and_tracker(
            working_directory.clone(),
            Arc::clone(&file_access_tracker),
        )),
        Arc::new(GlobTool::with_working_directory(working_directory.clone())),
        Arc::new(GrepTool::with_working_directory(working_directory.clone())),
        Arc::new(NotebookEditTool::with_working_directory_and_tracker(
            working_directory.clone(),
            Arc::clone(&file_access_tracker),
        )),
        // Process tools
        Arc::new(BashTool::with_working_directory(working_directory.clone())),
        Arc::new(KillShellTool::new()),
        match &subagent_graph {
            Some(graph) => Arc::new(TaskTool::with_registry_and_graph(
                Arc::clone(&task_registry),
                Arc::clone(graph),
            )),
            None => Arc::new(TaskTool::with_registry(Arc::clone(&task_registry))),
        },
        match &subagent_graph {
            Some(graph) => Arc::new(TaskOutputTool::with_task_registry_and_graph(
                Arc::clone(&task_registry),
                Arc::clone(graph),
            )),
            None => Arc::new(TaskOutputTool::with_task_registry(Arc::clone(
                &task_registry,
            ))),
        },
        // Task management
        Arc::new(TodoWriteTool::new()), // Claude Code compatible
        Arc::new(TodoReadTool::new()),  // Read current todo list status
        Arc::new(TaskDoneTool::new()),
        // Planning mode
        Arc::new(EnterPlanModeTool::new()),
        Arc::new(ExitPlanModeTool::new()),
        // User interaction
        Arc::new(AskUserQuestionTool::new()),
        // Extensions
        Arc::new(SkillTool::with_registry_and_working_directory(
            config.skill_registry,
            working_directory.clone(),
        )),
        Arc::new(SlashCommandTool::with_working_directory(
            working_directory.clone(),
        )),
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
        Arc::new(HttpClientTool::with_working_directory(
            working_directory.clone(),
        )),
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
        Arc::new(LspTool::with_working_directory(working_directory)),
        // Team collaboration
        Arc::new(TeammateTool::new()),
        Arc::new(SendMessageTool::new()),
    ];

    if let Some(graph) = &subagent_graph {
        let insert_at = tools
            .iter()
            .position(|tool| tool.name() == "TaskOutput")
            .map(|index| index + 1)
            .unwrap_or(tools.len());
        tools.insert(
            insert_at,
            Arc::new(AgentLifecycleTool::with_task_registry_and_graph(
                Arc::clone(&task_registry),
                Arc::clone(graph),
            )),
        );
    }

    tools
}

fn graph_task_registry(thread_store: &Arc<dyn ThreadStore>) -> Arc<process::task::TaskRegistry> {
    let key = thread_store.registry_key().unwrap_or_else(|| {
        let raw: *const dyn ThreadStore = Arc::as_ptr(thread_store);
        format!("thread-store:{:p}", raw as *const ())
    });
    GRAPH_TASK_REGISTRIES
        .lock()
        .entry(key)
        .or_insert_with(|| Arc::new(process::task::TaskRegistry::new()))
        .clone()
}

/// Get all default tools organized by category
pub fn get_default_tools() -> Vec<Arc<dyn Tool>> {
    let working_directory = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    build_default_tools(DefaultToolConfig::with_new_skill_registry(
        working_directory,
    ))
}

pub fn get_default_tool_names() -> &'static [&'static str] {
    DEFAULT_TOOL_NAMES
}

pub const fn get_default_tool_count() -> usize {
    DEFAULT_TOOL_NAMES.len()
}

/// Get default tools bound to a specific working directory and shared skill registry.
///
/// Use this in the main executor path so that:
/// - System prompt skill listing
/// - Skill tool execution
///
/// operate on the same registry instance.
pub fn get_default_tools_with_context(
    working_directory: impl Into<PathBuf>,
    skill_registry: Arc<RwLock<SkillRegistry>>,
) -> Vec<Arc<dyn Tool>> {
    build_default_tools(DefaultToolConfig::new(working_directory, skill_registry))
}

/// Get default tools bound to a working directory, skill registry, and ThreadStore.
///
/// When a ThreadStore is provided, Task and TaskOutput share one in-memory task
/// registry plus a SubAgentGraph backed by that store.
pub fn get_default_tools_with_context_and_thread_store(
    working_directory: impl Into<PathBuf>,
    skill_registry: Arc<RwLock<SkillRegistry>>,
    thread_store: Arc<dyn ThreadStore>,
) -> Vec<Arc<dyn Tool>> {
    build_default_tools(
        DefaultToolConfig::new(working_directory, skill_registry).with_thread_store(thread_store),
    )
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
        Arc::new(TodoWriteTool::new()),
        Arc::new(TodoReadTool::new()),
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
