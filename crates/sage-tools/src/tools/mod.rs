//! Sage Agent Tools
//! 
//! This module contains all the tools available to the Sage Agent, organized by category:
//! 
//! - `file_ops`: File and code operations (edit, json_edit, codebase_retrieval)
//! - `process`: Process and terminal tools (bash)
//! - `task_mgmt`: Task management tools (task_management, reorganize_tasklist, task_done)
//! - `utils`: Utility tools (sequential_thinking, monitoring, enhanced_errors)
//! - `network`: Network and browser tools (web_search, web_fetch, browser, http_client)
//! - `diagnostics`: Diagnostics and content processing tools
//! - `vcs`: Version control system tools (git)
//! - `database`: Database tools (sql, mongodb)
//! - `container`: Container management tools (docker)
//! - `security`: Security tools (scanner)
//! - `data`: Data processing tools (csv_processor, email)

pub mod file_ops;
pub mod process;
pub mod task_mgmt;
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
pub use file_ops::{EditTool, JsonEditTool, CodebaseRetrievalTool};
pub use process::BashTool;
pub use task_mgmt::{ViewTasklistTool, AddTasksTool, UpdateTasksTool, ReorganizeTasklistTool, TaskDoneTool};
pub use utils::SequentialThinkingTool;
pub use network::{WebSearchTool, WebFetchTool, BrowserTool};
pub use diagnostics::{DiagnosticsTool, ViewRangeUntruncatedTool, SearchUntruncatedTool, RememberTool, RenderMermaidTool};

// New tools with updated interfaces
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
        Arc::new(JsonEditTool::new()),
        Arc::new(CodebaseRetrievalTool::new()),
        
        // Process tools
        Arc::new(BashTool::new()),
        
        // Task management
        Arc::new(ViewTasklistTool::new()),
        Arc::new(AddTasksTool::new()),
        Arc::new(UpdateTasksTool::new()),
        Arc::new(ReorganizeTasklistTool::new()),
        Arc::new(TaskDoneTool::new()),
        
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
        
        // New tools with updated interfaces
        Arc::new(GitTool::new()),
        Arc::new(LogAnalyzerTool::new()),
        Arc::new(TestGeneratorTool::new()),
        Arc::new(KubernetesTool::new()),
        Arc::new(TerraformTool::new()),
        Arc::new(CloudTool::new()),
    ]
}

/// Get tools by category
pub fn get_file_ops_tools() -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(EditTool::new()),
        Arc::new(JsonEditTool::new()),
        Arc::new(CodebaseRetrievalTool::new()),
    ]
}

pub fn get_process_tools() -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(BashTool::new()),
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