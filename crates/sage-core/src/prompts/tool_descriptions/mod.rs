//! Tool descriptions module
//!
//! Contains detailed descriptions for each tool, following Claude Code's pattern
//! of having separate, comprehensive descriptions for each tool with examples.

mod execution_tools;
mod file_tools;
mod planning_tools;
mod search_tools;
mod web_tools;

/// Tool descriptions that provide detailed usage guidance
pub struct ToolDescriptions;

impl ToolDescriptions {
    /// Read tool description - comprehensive file reading guidance
    pub const READ: &'static str = file_tools::READ;

    /// Edit tool description - string replacement guidance
    pub const EDIT: &'static str = file_tools::EDIT;

    /// Write tool description - file creation guidance
    pub const WRITE: &'static str = file_tools::WRITE;

    /// Bash tool description - comprehensive shell command guidance
    pub const BASH: &'static str = execution_tools::BASH;

    /// Glob tool description
    pub const GLOB: &'static str = search_tools::GLOB;

    /// Grep tool description
    pub const GREP: &'static str = search_tools::GREP;

    /// Task tool description
    pub const TASK: &'static str = execution_tools::TASK;

    /// TodoWrite tool description - comprehensive task management guidance
    pub const TODO_WRITE: &'static str = planning_tools::TODO_WRITE;

    /// Enter plan mode tool description
    pub const ENTER_PLAN_MODE: &'static str = planning_tools::ENTER_PLAN_MODE;

    /// Exit plan mode tool description
    pub const EXIT_PLAN_MODE: &'static str = planning_tools::EXIT_PLAN_MODE;

    /// AskUserQuestion tool description
    pub const ASK_USER_QUESTION: &'static str = planning_tools::ASK_USER_QUESTION;

    /// WebFetch tool description
    pub const WEB_FETCH: &'static str = web_tools::WEB_FETCH;

    /// WebSearch tool description
    pub const WEB_SEARCH: &'static str = web_tools::WEB_SEARCH;

    /// Get description for a tool by name
    pub fn for_tool(name: &str) -> Option<&'static str> {
        match name {
            "Read" => Some(Self::READ),
            "Edit" => Some(Self::EDIT),
            "Write" => Some(Self::WRITE),
            "Bash" => Some(Self::BASH),
            "Glob" => Some(Self::GLOB),
            "Grep" => Some(Self::GREP),
            "Task" => Some(Self::TASK),
            "TodoWrite" => Some(Self::TODO_WRITE),
            "EnterPlanMode" => Some(Self::ENTER_PLAN_MODE),
            "ExitPlanMode" => Some(Self::EXIT_PLAN_MODE),
            "AskUserQuestion" => Some(Self::ASK_USER_QUESTION),
            "WebFetch" => Some(Self::WEB_FETCH),
            "WebSearch" => Some(Self::WEB_SEARCH),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_descriptions_exist() {
        assert!(!ToolDescriptions::READ.is_empty());
        assert!(!ToolDescriptions::EDIT.is_empty());
        assert!(!ToolDescriptions::WRITE.is_empty());
        assert!(!ToolDescriptions::BASH.is_empty());
        assert!(!ToolDescriptions::GLOB.is_empty());
        assert!(!ToolDescriptions::GREP.is_empty());
        assert!(!ToolDescriptions::TASK.is_empty());
        assert!(!ToolDescriptions::TODO_WRITE.is_empty());
    }

    #[test]
    fn test_for_tool() {
        assert!(ToolDescriptions::for_tool("Read").is_some());
        assert!(ToolDescriptions::for_tool("Bash").is_some());
        assert!(ToolDescriptions::for_tool("TodoWrite").is_some());
        assert!(ToolDescriptions::for_tool("Unknown").is_none());
    }

    #[test]
    fn test_bash_contains_examples() {
        let desc = ToolDescriptions::BASH;
        assert!(desc.contains("<good-example>"));
        assert!(desc.contains("<bad-example>"));
    }

    #[test]
    fn test_todo_write_contains_examples() {
        let desc = ToolDescriptions::TODO_WRITE;
        assert!(desc.contains("<example>"));
        assert!(desc.contains("<reasoning>"));
    }

    #[test]
    fn test_enter_plan_mode_contains_examples() {
        let desc = ToolDescriptions::ENTER_PLAN_MODE;
        assert!(desc.contains("GOOD"));
        assert!(desc.contains("BAD"));
    }

    #[test]
    fn test_variable_placeholders() {
        assert!(ToolDescriptions::READ.contains("${BASH_TOOL_NAME}"));
        assert!(ToolDescriptions::BASH.contains("${GLOB_TOOL_NAME}"));
        assert!(ToolDescriptions::BASH.contains("${GREP_TOOL_NAME}"));
        assert!(ToolDescriptions::TASK.contains("${TASK_TOOL_NAME}"));
    }
}
