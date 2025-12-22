//! Tool usage validation

use crate::tools::Tool;
use sage_core::error::{SageError, SageResult};
use std::collections::HashSet;

/// Tool usage policy validator
pub struct ToolUsageValidator {
    /// Commands that should not be used in Bash
    forbidden_bash_commands: HashSet<&'static str>,
}

impl ToolUsageValidator {
    /// Create a new validator
    pub fn new() -> Self {
        let mut forbidden = HashSet::new();

        // File reading commands
        forbidden.insert("cat");
        forbidden.insert("head");
        forbidden.insert("tail");
        forbidden.insert("less");
        forbidden.insert("more");

        // File writing commands
        forbidden.insert("tee");

        // File editing commands
        forbidden.insert("sed");
        forbidden.insert("awk");
        forbidden.insert("perl");

        // File finding commands
        forbidden.insert("find");
        forbidden.insert("locate");

        // Content search commands
        forbidden.insert("grep");
        forbidden.insert("rg");
        forbidden.insert("ag");
        forbidden.insert("ack");

        Self {
            forbidden_bash_commands: forbidden,
        }
    }

    /// Validate a Bash command
    pub fn validate_bash_command(&self, command: &str) -> SageResult<()> {
        let command_lower = command.to_lowercase();

        // Check for file reading
        if command_lower.contains("cat ") || command_lower.starts_with("cat ") {
            return Err(SageError::tool(
                "Bash",
                "Use Read tool instead of 'cat' for reading files",
            ));
        }

        if command_lower.contains("head ") || command_lower.contains("tail ") {
            return Err(SageError::tool(
                "Bash",
                "Use Read tool with offset/limit instead of 'head'/'tail'",
            ));
        }

        // Check for file writing
        if (command_lower.contains("echo ") && command_lower.contains(">"))
            || command_lower.contains("cat <<")
        {
            return Err(SageError::tool(
                "Bash",
                "Use Write or Edit tool instead of shell redirection",
            ));
        }

        // Check for file editing
        if command_lower.contains("sed ") || command_lower.contains("awk ") {
            return Err(SageError::tool(
                "Bash",
                "Use Edit tool instead of 'sed'/'awk' for file editing",
            ));
        }

        // Check for file finding
        if command_lower.contains("find ") && !command_lower.contains("git") {
            return Err(SageError::tool(
                "Bash",
                "Use Glob tool instead of 'find' for file pattern matching",
            ));
        }

        // Check for content search
        if (command_lower.contains("grep ") || command_lower.contains("rg "))
            && !command_lower.contains("git")
        {
            return Err(SageError::tool(
                "Bash",
                "Use Grep tool instead of 'grep'/'rg' for content search",
            ));
        }

        // Check for echo used for user communication
        if command_lower.starts_with("echo ") && !command_lower.contains(">") {
            return Err(SageError::tool(
                "Bash",
                "Output text directly instead of using 'echo' for user communication",
            ));
        }

        Ok(())
    }

    /// Validate tool selection for a given task
    pub fn validate_tool_selection(
        &self,
        tool_name: &str,
        task_description: &str,
    ) -> SageResult<()> {
        let task_lower = task_description.to_lowercase();

        // Check if Bash is being used for file operations
        if tool_name == "Bash" {
            if task_lower.contains("read file")
                || task_lower.contains("view file")
                || task_lower.contains("show file")
            {
                return Err(SageError::tool(
                    "Bash",
                    "Use Read tool instead of Bash for reading files",
                ));
            }

            if task_lower.contains("write file")
                || task_lower.contains("create file")
                || task_lower.contains("save file")
            {
                return Err(SageError::tool(
                    "Bash",
                    "Use Write tool instead of Bash for creating files",
                ));
            }

            if task_lower.contains("edit file")
                || task_lower.contains("modify file")
                || task_lower.contains("update file")
            {
                return Err(SageError::tool(
                    "Bash",
                    "Use Edit tool instead of Bash for modifying files",
                ));
            }

            if task_lower.contains("find file") || task_lower.contains("search file") {
                return Err(SageError::tool(
                    "Bash",
                    "Use Glob tool instead of Bash for finding files",
                ));
            }

            if task_lower.contains("search content") || task_lower.contains("grep") {
                return Err(SageError::tool(
                    "Bash",
                    "Use Grep tool instead of Bash for searching content",
                ));
            }
        }

        Ok(())
    }
}

impl Default for ToolUsageValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_bash_cat() {
        let validator = ToolUsageValidator::new();
        assert!(validator.validate_bash_command("cat file.txt").is_err());
        assert!(validator.validate_bash_command("cat /path/to/file").is_err());
    }

    #[test]
    fn test_validate_bash_echo_redirect() {
        let validator = ToolUsageValidator::new();
        assert!(validator
            .validate_bash_command("echo 'content' > file.txt")
            .is_err());
    }

    #[test]
    fn test_validate_bash_sed() {
        let validator = ToolUsageValidator::new();
        assert!(validator
            .validate_bash_command("sed -i 's/old/new/' file.txt")
            .is_err());
    }

    #[test]
    fn test_validate_bash_find() {
        let validator = ToolUsageValidator::new();
        assert!(validator
            .validate_bash_command("find . -name '*.rs'")
            .is_err());
    }

    #[test]
    fn test_validate_bash_grep() {
        let validator = ToolUsageValidator::new();
        assert!(validator
            .validate_bash_command("grep 'pattern' file.txt")
            .is_err());
    }

    #[test]
    fn test_validate_bash_allowed_commands() {
        let validator = ToolUsageValidator::new();
        assert!(validator.validate_bash_command("git status").is_ok());
        assert!(validator.validate_bash_command("cargo build").is_ok());
        assert!(validator.validate_bash_command("npm install").is_ok());
        assert!(validator.validate_bash_command("docker ps").is_ok());
    }

    #[test]
    fn test_validate_tool_selection() {
        let validator = ToolUsageValidator::new();

        assert!(validator
            .validate_tool_selection("Bash", "read file contents")
            .is_err());
        assert!(validator
            .validate_tool_selection("Bash", "write file data")
            .is_err());
        assert!(validator
            .validate_tool_selection("Bash", "edit file content")
            .is_err());

        assert!(validator
            .validate_tool_selection("Bash", "run git command")
            .is_ok());
    }
}
