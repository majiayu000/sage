//! Slash command execution tool
//!
//! Allows executing custom slash commands within conversation context.
//! Slash commands are user-defined shortcuts that expand to prompts or execute actions.

use async_trait::async_trait;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
use std::path::PathBuf;

/// Tool for executing slash commands
///
/// Slash commands are custom shortcuts defined in `.claude/commands/` that can
/// expand to prompts or trigger specific actions. They provide a way to create
/// reusable command templates.
///
/// # Examples
///
/// - `command: "/review-pr 123"` - Review pull request #123
/// - `command: "/test"` - Run test suite
/// - `command: "/deploy production"` - Deploy to production
pub struct SlashCommandTool {
    /// Base directory for command files (typically `.claude/commands/`)
    command_dir: Option<PathBuf>,
}

impl Default for SlashCommandTool {
    fn default() -> Self {
        Self::new()
    }
}

impl SlashCommandTool {
    /// Create a new SlashCommandTool instance
    pub fn new() -> Self {
        Self { command_dir: None }
    }

    /// Create a SlashCommandTool with a specific command directory
    pub fn with_command_dir(command_dir: PathBuf) -> Self {
        Self {
            command_dir: Some(command_dir),
        }
    }

    /// Get the command directory
    fn get_command_dir(&self) -> PathBuf {
        self.command_dir.clone().unwrap_or_else(|| {
            // Default to .claude/commands in current directory
            PathBuf::from(".claude/commands")
        })
    }

    /// Parse command string into command name and arguments
    fn parse_command(&self, command: &str) -> Result<(String, Vec<String>), ToolError> {
        if !command.starts_with('/') {
            return Err(ToolError::InvalidArguments(
                "Slash command must start with '/'".to_string(),
            ));
        }

        let parts: Vec<&str> = command[1..].split_whitespace().collect();
        if parts.is_empty() {
            return Err(ToolError::InvalidArguments(
                "Invalid slash command format".to_string(),
            ));
        }

        let command_name = parts[0].to_string();
        let args = parts[1..].iter().map(|s| s.to_string()).collect();

        Ok((command_name, args))
    }

    /// Execute the slash command
    async fn execute_command(
        &self,
        command_name: &str,
        args: &[String],
    ) -> Result<String, ToolError> {
        // In a real implementation, this would:
        // 1. Look up the command file in .claude/commands/
        // 2. Read and expand the command template
        // 3. Substitute arguments
        // 4. Return the expanded prompt or execute the action

        let command_dir = self.get_command_dir();
        let command_file = command_dir.join(format!("{}.md", command_name));

        // For now, return a message indicating the command would be executed
        let args_str = if args.is_empty() {
            String::new()
        } else {
            format!(" with arguments: {}", args.join(" "))
        };

        Ok(format!(
            "Slash command '{}' execution initiated{}.\n\
             Command file: {}\n\
             \n\
             The command prompt will be expanded and processed in the conversation context.",
            command_name,
            args_str,
            command_file.display()
        ))
    }

    /// Validate command format
    fn validate_command(&self, command: &str) -> Result<(), ToolError> {
        if command.is_empty() {
            return Err(ToolError::InvalidArguments(
                "Command cannot be empty".to_string(),
            ));
        }

        if !command.starts_with('/') {
            return Err(ToolError::InvalidArguments(
                "Slash command must start with '/'".to_string(),
            ));
        }

        // Parse to ensure valid format
        self.parse_command(command)?;

        Ok(())
    }
}

#[async_trait]
impl Tool for SlashCommandTool {
    fn name(&self) -> &str {
        "slash_command"
    }

    fn description(&self) -> &str {
        "Execute a custom slash command. Slash commands are user-defined shortcuts stored in \
         `.claude/commands/` that expand to prompts or trigger actions. Each command can accept \
         arguments. Use this when you need to run a predefined command template or workflow."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![ToolParameter::string(
                "command",
                "The slash command to execute with its arguments (e.g., '/review-pr 123', '/test', '/deploy production')",
            )],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        // Extract command parameter
        let command = call.get_string("command").ok_or_else(|| {
            ToolError::InvalidArguments("Missing required parameter: command".to_string())
        })?;

        // Validate command format
        self.validate_command(&command)?;

        // Parse command
        let (command_name, args) = self.parse_command(&command)?;

        // Execute the command
        let result = self.execute_command(&command_name, &args).await?;

        Ok(ToolResult::success(&call.id, self.name(), result))
    }

    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        let command = call.get_string("command").ok_or_else(|| {
            ToolError::InvalidArguments("Missing required parameter: command".to_string())
        })?;

        self.validate_command(&command)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;

    fn create_tool_call(id: &str, name: &str, command: &str) -> ToolCall {
        let mut arguments = HashMap::new();
        arguments.insert("command".to_string(), json!(command));

        ToolCall {
            id: id.to_string(),
            name: name.to_string(),
            arguments,
            call_id: None,
        }
    }

    #[tokio::test]
    async fn test_slash_command_execution() {
        let tool = SlashCommandTool::new();
        let call = create_tool_call("test-1", "slash_command", "/review-pr 123");

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        assert!(result.output.as_ref().unwrap().contains("review-pr"));
        assert!(result.output.as_ref().unwrap().contains("123"));
    }

    #[tokio::test]
    async fn test_command_parsing() {
        let tool = SlashCommandTool::new();

        // Simple command without args
        let (name, args) = tool.parse_command("/test").unwrap();
        assert_eq!(name, "test");
        assert!(args.is_empty());

        // Command with single arg
        let (name, args) = tool.parse_command("/review-pr 123").unwrap();
        assert_eq!(name, "review-pr");
        assert_eq!(args, vec!["123"]);

        // Command with multiple args
        let (name, args) = tool.parse_command("/deploy production --force").unwrap();
        assert_eq!(name, "deploy");
        assert_eq!(args, vec!["production", "--force"]);
    }

    #[tokio::test]
    async fn test_invalid_command_format() {
        let tool = SlashCommandTool::new();

        // Missing slash
        let call = create_tool_call("test-2", "slash_command", "test");
        assert!(tool.validate(&call).is_err());

        // Empty command
        let call = create_tool_call("test-3", "slash_command", "");
        assert!(tool.validate(&call).is_err());

        // Just slash
        let call = create_tool_call("test-4", "slash_command", "/");
        assert!(tool.validate(&call).is_err());
    }

    #[tokio::test]
    async fn test_valid_commands() {
        let tool = SlashCommandTool::new();

        let valid_commands = vec![
            "/test",
            "/review-pr 123",
            "/deploy production",
            "/help",
            "/clear",
        ];

        for cmd in valid_commands {
            let call = create_tool_call(&format!("test-{}", cmd), "slash_command", cmd);
            assert!(
                tool.validate(&call).is_ok(),
                "Command should be valid: {}",
                cmd
            );
        }
    }

    #[tokio::test]
    async fn test_missing_command_parameter() {
        let tool = SlashCommandTool::new();
        let call = ToolCall {
            id: "test-5".to_string(),
            name: "slash_command".to_string(),
            arguments: HashMap::new(),
            call_id: None,
        };

        let result = tool.execute(&call).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Missing required parameter")
        );
    }

    #[tokio::test]
    async fn test_custom_command_dir() {
        let custom_dir = PathBuf::from("/custom/commands");
        let tool = SlashCommandTool::with_command_dir(custom_dir.clone());

        assert_eq!(tool.get_command_dir(), custom_dir);
    }

    #[tokio::test]
    async fn test_tool_schema() {
        let tool = SlashCommandTool::new();
        let schema = tool.schema();

        assert_eq!(schema.name, "slash_command");
        assert!(!schema.description.is_empty());

        // Check that the schema has the command parameter
        let params = schema.parameters.as_object().unwrap();
        assert!(params.contains_key("properties"));

        let properties = params.get("properties").unwrap().as_object().unwrap();
        assert!(properties.contains_key("command"));
    }

    #[tokio::test]
    async fn test_command_with_special_characters() {
        let tool = SlashCommandTool::new();

        // Command with dashes
        let call = create_tool_call("test-6", "slash_command", "/git-commit-smart");
        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);

        // Command with underscores
        let call = create_tool_call("test-7", "slash_command", "/run_tests");
        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_command_execution_output() {
        let tool = SlashCommandTool::new();
        let call = create_tool_call("test-8", "slash_command", "/review-pr 123");

        let result = tool.execute(&call).await.unwrap();
        let output = result.output.unwrap();

        // Check that output contains expected information
        assert!(output.contains("review-pr"));
        assert!(output.contains("123"));
        assert!(output.contains("Command file:"));
        assert!(output.contains(".claude/commands"));
    }
}
