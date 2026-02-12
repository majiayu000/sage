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
    pub const JSON_EDIT: &str = "JsonEdit";
    /// Edit Jupyter notebooks
    pub const NOTEBOOK_EDIT: &str = "NotebookEdit";
}

/// Process/execution tools
pub mod process {
    /// Execute bash commands
    pub const BASH: &str = "Bash";
    /// Spawn background tasks
    pub const TASK: &str = "Task";
    /// Get task output
    pub const TASK_OUTPUT: &str = "TaskOutput";
    /// Kill background shell
    pub const KILL_SHELL: &str = "KillShell";
}

/// Task management tools
pub mod task_mgmt {
    /// Mark task as done
    pub const TASK_DONE: &str = "TaskDone";
    /// Write todo items
    pub const TODO_WRITE: &str = "TodoWrite";
    /// Read todo items
    pub const TODO_READ: &str = "TodoRead";
}

/// User interaction tools
pub mod interaction {
    /// Ask user a question
    pub const ASK_USER: &str = "AskUserQuestion";
}

/// Planning tools
pub mod planning {
    /// Enter plan mode
    pub const ENTER_PLAN_MODE: &str = "EnterPlanMode";
    /// Exit plan mode
    pub const EXIT_PLAN_MODE: &str = "ExitPlanMode";
}

/// Network tools
pub mod network {
    /// Fetch web content
    pub const WEB_FETCH: &str = "WebFetch";
    /// Search the web
    pub const WEB_SEARCH: &str = "WebSearch";
    /// Open browser
    pub const OPEN_BROWSER: &str = "OpenBrowser";
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
        assert!(!is_file_modifying_tool("Bash"));
    }

    #[test]
    fn test_completion_tool() {
        assert!(is_completion_tool("TaskDone"));
        assert!(!is_completion_tool("Write"));
    }
}
