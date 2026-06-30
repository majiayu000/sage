//! Process and terminal tools

pub mod agent_lifecycle;
pub mod agent_messaging;
pub mod bash;
pub mod kill_shell;
pub mod task;
pub mod task_output;

// Re-export tools
pub use agent_lifecycle::AgentLifecycleTool;
pub use agent_messaging::AgentMessagingTool;
pub use bash::{
    BashTool, requires_user_confirmation, validate_command_comprehensive,
    validate_command_security, validate_command_with_strictness,
};
pub use kill_shell::KillShellTool;
pub use task::{
    GLOBAL_TASK_REGISTRY, TaskRegistry, TaskRequest, TaskStatus, TaskTool, get_pending_tasks,
    get_task, update_task_status,
};
pub use task_output::TaskOutputTool;
