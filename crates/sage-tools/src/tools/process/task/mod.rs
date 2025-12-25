//! Task tool - Claude Code compatible subagent spawning
//!
//! Launches specialized sub-agents to handle complex tasks autonomously.
//! Now with actual execution support via SubAgentRunner.

mod executor;
mod schema;
mod tool;
mod types;

#[cfg(test)]
mod tests;

// Re-export public items
pub use tool::TaskTool;
pub use types::{
    TaskRegistry, TaskRequest, TaskStatus, get_pending_tasks, get_task, update_task_status,
    GLOBAL_TASK_REGISTRY,
};
