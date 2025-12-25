//! Process and terminal tools

pub mod bash;
pub mod kill_shell;
pub mod task;
pub mod task_output;

// Re-export tools
pub use bash::{BashTool, requires_user_confirmation, validate_command_security};
pub use kill_shell::KillShellTool;
pub use task::{
    TaskRegistry, TaskRequest, TaskStatus, TaskTool, get_pending_tasks, get_task,
    update_task_status, GLOBAL_TASK_REGISTRY,
};
pub use task_output::TaskOutputTool;
