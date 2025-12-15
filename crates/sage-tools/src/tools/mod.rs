//! Sage Agent Tools
//!
//! This module contains all the tools available to the Sage Agent, organized by category:
//!
//! - `file_ops`: File and code operations (edit, json_edit, codebase_retrieval, notebook_edit)
//! - `process`: Process and terminal tools (bash, kill_shell)
//! - `task_mgmt`: Task management tools (task_management, reorganize_tasklist, task_done)
//! - `planning`: Planning mode tools (enter_plan_mode, exit_plan_mode)
//! - `interaction`: User interaction tools (ask_user_question)
//! - `extensions`: Extension tools (skill, slash_command)
//! - `utils`: Utility tools (sequential_thinking, monitoring, enhanced_errors)
//! - `network`: Network and browser tools (web_search, web_fetch, browser)
//! - `diagnostics`: Diagnostics and content processing tools
//! - `vcs`: Version control system tools (git)
//! - `monitoring`: Monitoring tools (log_analyzer, test_generator)
//! - `infrastructure`: Infrastructure tools (kubernetes, terraform, cloud)

pub mod file_ops;
pub mod process;
pub mod task_mgmt;
pub mod planning;
pub mod interaction;
pub mod extensions;
pub mod utils;
pub mod diagnostics;
pub mod monitoring;
pub mod infrastructure;

// VCS module with only updated git_simple
pub mod vcs {
    pub mod git_simple;
    pub use git_simple::GitTool;
}

// Network module with only working tools
pub mod network {
    pub mod web_search;
    pub mod web_fetch;
    pub mod browser;

    pub use web_search::WebSearchTool;
    pub use web_fetch::WebFetchTool;
    pub use browser::BrowserTool;
}

// Re-export all tools for easy access
// Note: JsonEditTool, CodebaseRetrievalTool, MultiEditTool are Sage-specific and currently disabled
pub use file_ops::{EditTool, ReadTool, WriteTool, GlobTool, GrepTool, NotebookEditTool};
pub use process::{BashTool, KillShellTool};
pub use task_mgmt::{ViewTasklistTool, AddTasksTool, UpdateTasksTool, ReorganizeTasklistTool, TaskDoneTool};
pub use planning::{EnterPlanModeTool, ExitPlanModeTool};
pub use interaction::AskUserQuestionTool;
pub use extensions::{SkillTool, SlashCommandTool};
pub use utils::SequentialThinkingTool;
pub use network::{WebSearchTool, WebFetchTool, BrowserTool};
pub use diagnostics::{DiagnosticsTool, ViewRangeUntruncatedTool, SearchUntruncatedTool, RememberTool, RenderMermaidTool};
pub use vcs::GitTool;
pub use monitoring::{LogAnalyzerTool, TestGeneratorTool};
pub use infrastructure::{KubernetesTool, TerraformTool, CloudTool};

use std::sync::Arc;
use sage_core::tools::Tool;

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
        // Disabled Sage-specific tools: JsonEditTool, CodebaseRetrievalTool, MultiEditTool

        // Process tools
        Arc::new(BashTool::new()),
        Arc::new(KillShellTool::new()),

        // Task management
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

        // Utilities
        Arc::new(SequentialThinkingTool::new()),

        // Network tools
        Arc::new(WebSearchTool::new()),
        Arc::new(WebFetchTool::new()),
        Arc::new(BrowserTool::new()),

        // Diagnostics
        Arc::new(DiagnosticsTool::new()),
        Arc::new(ViewRangeUntruncatedTool::new()),
        Arc::new(SearchUntruncatedTool::new()),
        Arc::new(RememberTool::new()),
        Arc::new(RenderMermaidTool::new()),

        // VCS
        Arc::new(GitTool::new()),

        // Monitoring
        Arc::new(LogAnalyzerTool::new()),
        Arc::new(TestGeneratorTool::new()),

        // Infrastructure
        Arc::new(KubernetesTool::new()),
        Arc::new(TerraformTool::new()),
        Arc::new(CloudTool::new()),
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
        // Disabled Sage-specific tools: JsonEditTool, CodebaseRetrievalTool, MultiEditTool
    ]
}

pub fn get_process_tools() -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(BashTool::new()),
        Arc::new(KillShellTool::new()),
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
    vec![
        Arc::new(AskUserQuestionTool::new()),
    ]
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
    ]
}

pub fn get_diagnostics_tools() -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(DiagnosticsTool::new()),
        Arc::new(ViewRangeUntruncatedTool::new()),
        Arc::new(SearchUntruncatedTool::new()),
        Arc::new(RememberTool::new()),
        Arc::new(RenderMermaidTool::new()),
    ]
}

pub fn get_vcs_tools() -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(GitTool::new()),
    ]
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

#[cfg(test)]
pub mod tests;
