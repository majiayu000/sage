//! Tool implementations for Sage Agent

pub mod bash;
pub mod edit;
pub mod json_edit;
pub mod sequential_thinking;
pub mod task_done;
pub mod augment_tools;
pub mod utils;

// TODO: Add MCP-compatible tools
// pub mod mcp_tools;  // MCP protocol compatible tools

// TODO: Add development tools
// pub mod git;         // Advanced Git operations
// pub mod docker;      // Container management
// pub mod kubernetes;  // K8s orchestration
// pub mod terraform;   // Infrastructure as Code

// TODO: Add data processing tools
// pub mod database;    // SQL database operations
// pub mod csv_processor; // CSV/Excel processing
// pub mod json_processor; // JSON manipulation
// pub mod xml_processor;  // XML processing

// TODO: Add communication tools
// pub mod http_client; // HTTP/REST API client
// pub mod websocket;   // WebSocket client
// pub mod email;       // Email operations
// pub mod slack;       // Slack integration

// TODO: Add security tools
// pub mod security_scanner; // Vulnerability scanning
// pub mod secret_manager;   // Secret management
// pub mod cert_manager;     // Certificate operations

// TODO: Add monitoring tools
// pub mod log_analyzer;     // Log analysis
// pub mod metrics_collector; // Metrics collection
// pub mod health_checker;   // Health monitoring

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
