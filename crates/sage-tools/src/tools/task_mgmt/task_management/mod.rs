//! Task management tools for organizing complex work

mod add_tool;
mod task_list;
mod types;
mod update_tool;
mod view_tool;

#[cfg(test)]
mod tests;

// Re-export public APIs
pub use add_tool::AddTasksTool;
pub use task_list::{GLOBAL_TASK_LIST, TaskList};
pub use types::{Task, TaskState};
pub use update_tool::UpdateTasksTool;
pub use view_tool::ViewTasklistTool;
