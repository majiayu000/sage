//! Tool descriptions module
//!
//! Contains detailed descriptions for each tool, following Claude Code's pattern
//! of having separate, comprehensive descriptions for each tool.

/// Tool descriptions that provide detailed usage guidance
pub struct ToolDescriptions;

impl ToolDescriptions {
    /// Read tool description
    pub const READ: &'static str = r#"Reads a file from the local filesystem.

Usage:
- The file_path parameter must be an absolute path
- By default, reads up to 2000 lines from the beginning
- You can specify offset and limit for long files
- This tool can read images (PNG, JPG) as Claude is multimodal
- Can read PDF files and Jupyter notebooks

Important:
- Use this tool instead of cat/head/tail commands
- Read files before editing them
- Read multiple files in parallel when possible"#;

    /// Edit tool description
    pub const EDIT: &'static str = r#"Performs exact string replacements in files.

Usage:
- You must Read the file first before editing
- The edit will FAIL if old_string is not unique
- Use replace_all for renaming variables across the file

Important:
- ALWAYS prefer editing existing files over creating new ones
- Preserve exact indentation from the file
- Only use emojis if the user explicitly requests it"#;

    /// Write tool description
    pub const WRITE: &'static str = r#"Writes a file to the local filesystem.

Usage:
- Will overwrite existing files
- You MUST Read existing files first before overwriting
- Use absolute paths

Important:
- NEVER create files unless absolutely necessary
- ALWAYS prefer editing existing files
- NEVER proactively create documentation files unless requested"#;

    /// Bash tool description
    pub const BASH: &'static str = r#"Executes bash commands in a persistent shell session.

Usage:
- For terminal operations like git, npm, docker
- Quote file paths with spaces using double quotes
- Commands timeout after 2 minutes by default

Important:
- DO NOT use for file operations - use Read/Edit/Write instead
- Avoid using grep/cat/head/tail - use specialized tools
- Can run commands in background with run_in_background parameter"#;

    /// Task done tool description
    pub const TASK_DONE: &'static str = r#"Marks the current task as complete.

CRITICAL: Only use this when ALL conditions are met:
1. You have CREATED or MODIFIED actual code files
2. The implementation is functional
3. All requested features are implemented
4. Tests pass (if applicable)

NEVER call task_done if you have only:
- Written plans or documentation
- Generated a list of tasks
- Described what you would do
- Not created any code files"#;

    /// Enter plan mode tool description
    pub const ENTER_PLAN_MODE: &'static str = r#"Enter QUICK plan mode for brief analysis before coding.

When to use:
- Complex multi-component tasks
- Multiple valid implementation approaches
- Architectural decisions needed
- Multi-file changes (3+ files)

When NOT to use:
- Single-line fixes
- Simple, clear requirements
- Bug fixes with obvious solutions

IMPORTANT:
- Keep planning under 2 minutes
- Then exit and START WRITING CODE
- Plans without code are worthless"#;

    /// Exit plan mode tool description
    pub const EXIT_PLAN_MODE: &'static str = r#"Exit plan mode and start implementation.

After calling this:
- You MUST start writing code immediately
- Do NOT call task_done without creating files
- User will review your plan first

Requirements before exiting:
- Plan written to plan file
- Key decisions documented
- Ready to implement"#;

    /// Get description for a tool by name
    pub fn for_tool(name: &str) -> Option<&'static str> {
        match name {
            "Read" => Some(Self::READ),
            "Edit" => Some(Self::EDIT),
            "Write" => Some(Self::WRITE),
            "Bash" => Some(Self::BASH),
            "task_done" => Some(Self::TASK_DONE),
            "enter_plan_mode" => Some(Self::ENTER_PLAN_MODE),
            "exit_plan_mode" => Some(Self::EXIT_PLAN_MODE),
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
    }

    #[test]
    fn test_for_tool() {
        assert!(ToolDescriptions::for_tool("Read").is_some());
        assert!(ToolDescriptions::for_tool("Unknown").is_none());
    }

    #[test]
    fn test_task_done_description() {
        let desc = ToolDescriptions::TASK_DONE;
        assert!(desc.contains("CRITICAL"));
        assert!(desc.contains("code files"));
        assert!(desc.contains("NEVER"));
    }
}
