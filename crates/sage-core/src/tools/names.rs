//! Tool name constants
//!
//! This module provides canonical tool names to avoid hardcoding strings
//! throughout the codebase. All tool name checks should use these constants.

/// File operation tools
pub mod file_ops {
    /// Read file contents
    pub const READ: &str = "Read";
    /// Write file contents
    pub const WRITE: &str = "Write";
    /// Edit file with string replacement
    pub const EDIT: &str = "Edit";
    /// Edit multiple locations in a file
    pub const MULTI_EDIT: &str = "MultiEdit";
    /// Search for files by pattern
    pub const GLOB: &str = "Glob";
    /// Search file contents
    pub const GREP: &str = "Grep";
    /// Edit JSON files
    pub const JSON_EDIT: &str = "json_edit_tool";
    /// Edit Jupyter notebooks
    pub const NOTEBOOK_EDIT: &str = "notebook_edit";
}

/// Process/execution tools
pub mod process {
    /// Execute bash commands
    pub const BASH: &str = "bash";
    /// Spawn background tasks
    pub const TASK: &str = "Task";
    /// Get task output
    pub const TASK_OUTPUT: &str = "task_output";
    /// Kill background shell
    pub const KILL_SHELL: &str = "kill_shell";
}

/// Task management tools
pub mod task_mgmt {
    /// Mark task as done
    pub const TASK_DONE: &str = "task_done";
    /// Write todo items
    pub const TODO_WRITE: &str = "TodoWrite";
    /// Read todo items
    pub const TODO_READ: &str = "TodoRead";
}

/// User interaction tools
pub mod interaction {
    /// Ask user a question
    pub const ASK_USER: &str = "ask_user_question";
}

/// Planning tools
pub mod planning {
    /// Enter plan mode
    pub const ENTER_PLAN_MODE: &str = "enter_plan_mode";
    /// Exit plan mode
    pub const EXIT_PLAN_MODE: &str = "exit_plan_mode";
}

/// Network tools
pub mod network {
    /// Fetch web content
    pub const WEB_FETCH: &str = "web-fetch";
    /// Search the web
    pub const WEB_SEARCH: &str = "web-search";
    /// Open browser
    pub const OPEN_BROWSER: &str = "open-browser";
}

/// Check if a tool name is a file-modifying tool
#[inline]
pub fn is_file_modifying_tool(name: &str) -> bool {
    matches!(
        name,
        file_ops::WRITE | file_ops::EDIT | file_ops::MULTI_EDIT
    )
}

/// Check if a tool name indicates task completion
#[inline]
pub fn is_completion_tool(name: &str) -> bool {
    name == task_mgmt::TASK_DONE
}

/// Check if a tool requires user interaction
#[inline]
pub fn is_interactive_tool(name: &str) -> bool {
    name == interaction::ASK_USER
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_modifying_tools() {
        assert!(is_file_modifying_tool("Write"));
        assert!(is_file_modifying_tool("Edit"));
        assert!(is_file_modifying_tool("MultiEdit"));
        assert!(!is_file_modifying_tool("Read"));
        assert!(!is_file_modifying_tool("bash"));
    }

    #[test]
    fn test_completion_tool() {
        assert!(is_completion_tool("task_done"));
        assert!(!is_completion_tool("Write"));
    }
}
