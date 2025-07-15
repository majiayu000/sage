//! Tool implementations for Sage Agent

pub mod bash;
pub mod edit;
pub mod json_edit;
pub mod sequential_thinking;
pub mod task_done;
pub mod augment_tools;
pub mod utils;

// Re-export tools
pub use bash::BashTool;
pub use edit::EditTool;
pub use json_edit::JsonEditTool;
pub use sequential_thinking::SequentialThinkingTool;
pub use task_done::TaskDoneTool;
pub use augment_tools::{
    CodebaseRetrievalTool, ViewTasklistTool, AddTasksTool,
    UpdateTasksTool, ReorganizeTasklistTool
};

use std::sync::Arc;
use sage_core::tools::Tool;

/// Get all default tools
pub fn get_default_tools() -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(BashTool::new()),
        Arc::new(EditTool::new()),
        Arc::new(JsonEditTool::new()),
        Arc::new(SequentialThinkingTool::new()),
        Arc::new(TaskDoneTool::new()),
        Arc::new(CodebaseRetrievalTool::new()),
        Arc::new(ViewTasklistTool::new()),
        Arc::new(AddTasksTool::new()),
        Arc::new(UpdateTasksTool::new()),
        Arc::new(ReorganizeTasklistTool::new()),
    ]
}
