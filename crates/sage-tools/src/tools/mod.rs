//! Sage Agent Tools
//! 
//! This module contains all the tools available to the Sage Agent, organized by category:
//! 
//! - `file_ops`: File and code operations (edit, json_edit, codebase_retrieval)
//! - `process`: Process and terminal tools (bash)
//! - `task_mgmt`: Task management tools (task_management, reorganize_tasklist, task_done)
//! - `utils`: Utility tools (sequential_thinking, monitoring, enhanced_errors)
//! - `network`: Network and browser tools (web_search, web_fetch, browser)
//! - `diagnostics`: Diagnostics and content processing tools

pub mod file_ops;
pub mod process;
pub mod task_mgmt;
pub mod utils;
pub mod network;
pub mod diagnostics;

// Re-export all tools for easy access
pub use file_ops::{EditTool, JsonEditTool, CodebaseRetrievalTool};
pub use process::BashTool;
pub use task_mgmt::{ViewTasklistTool, AddTasksTool, UpdateTasksTool, ReorganizeTasklistTool, TaskDoneTool};
pub use utils::SequentialThinkingTool;
pub use network::{WebSearchTool, WebFetchTool, BrowserTool};
pub use diagnostics::{DiagnosticsTool, ViewRangeUntruncatedTool, SearchUntruncatedTool, RememberTool, RenderMermaidTool};

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
