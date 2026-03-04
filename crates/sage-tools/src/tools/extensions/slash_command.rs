//! Slash command execution tool
//!
//! Executes custom slash commands from `.sage/commands/` and `~/.config/sage/commands/`.
//! Built-in commands (for example `/help`, `/clear`) are intentionally excluded from
//! this tool and must be handled by the CLI command pipeline.

use async_trait::async_trait;
use sage_core::commands::{CommandExecutor, CommandInvocation, CommandRegistry, CommandSource};
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Tool for executing custom slash commands.
pub struct SlashCommandTool {
    /// Working directory used to resolve `.sage/commands/`.
    working_directory: PathBuf,
}

impl Default for SlashCommandTool {
    fn default() -> Self {
        Self::new()
    }
}

impl SlashCommandTool {
    /// Create a new SlashCommandTool instance.
    pub fn new() -> Self {
        let working_directory = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self { working_directory }
    }

    /// Create a SlashCommandTool bound to a specific working directory.
    pub fn with_working_directory(working_directory: impl Into<PathBuf>) -> Self {
        Self {
            working_directory: working_directory.into(),
        }
    }

    /// Parse command string into command invocation.
    fn parse_command(&self, command: &str) -> Result<CommandInvocation, ToolError> {
        CommandInvocation::parse(command).ok_or_else(|| {
            ToolError::InvalidArguments(format!("Invalid slash command format: {}", command))
        })
    }

    /// Validate command format.
    fn validate_command(&self, command: &str) -> Result<(), ToolError> {
        if command.trim().is_empty() {
            return Err(ToolError::InvalidArguments(
                "Command cannot be empty".to_string(),
            ));
        }

        let _ = self.parse_command(command)?;
        Ok(())
    }

    async fn load_registry(&self) -> Result<Arc<RwLock<CommandRegistry>>, ToolError> {
        let mut registry = CommandRegistry::new(&self.working_directory);
        registry.register_builtins();
        registry.discover().await.map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to discover slash commands: {}", e))
        })?;
        Ok(Arc::new(RwLock::new(registry)))
    }

    async fn ensure_custom_command(
        &self,
        invocation: &CommandInvocation,
        registry: &Arc<RwLock<CommandRegistry>>,
    ) -> Result<(), ToolError> {
        let registry = registry.read().await;

        let (_, source) = registry
            .get_with_source(&invocation.command_name)
            .ok_or_else(|| {
                let mut available_custom: Vec<String> = registry
                    .list()
                    .iter()
                    .filter(|(_, src)| **src != CommandSource::Builtin)
                    .map(|(cmd, _)| format!("/{}", cmd.name))
                    .collect();
                available_custom.sort();

                if available_custom.is_empty() {
                    ToolError::ExecutionFailed(format!(
                        "Unknown slash command: /{}. No custom slash commands are available.",
                        invocation.command_name
                    ))
                } else {
                    ToolError::ExecutionFailed(format!(
                        "Unknown slash command: /{}. Available custom commands: {}",
                        invocation.command_name,
                        available_custom.join(", ")
                    ))
                }
            })?;

        if *source == CommandSource::Builtin {
            return Err(ToolError::ExecutionFailed(format!(
                "Built-in command '/{}' is not supported by SlashCommand tool",
                invocation.command_name
            )));
        }

        Ok(())
    }

    /// Execute the slash command via command registry/executor.
    async fn execute_command(
        &self,
        invocation: &CommandInvocation,
    ) -> Result<sage_core::commands::CommandResult, ToolError> {
        let registry = self.load_registry().await?;
        self.ensure_custom_command(invocation, &registry).await?;

        let executor = CommandExecutor::new(Arc::clone(&registry));
        let result = executor
            .process(&invocation.raw_input)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to execute command: {}", e)))?
            .ok_or_else(|| {
                ToolError::ExecutionFailed(
                    "Command did not produce an execution result".to_string(),
                )
            })?;

        if result.is_local {
            return Err(ToolError::ExecutionFailed(format!(
                "Slash command '/{}' resolved to local execution, which is not supported by this tool",
                invocation.command_name
            )));
        }

        if result.interactive.is_some() {
            return Err(ToolError::ExecutionFailed(format!(
                "Slash command '/{}' requires interactive handling and cannot run in this tool",
                invocation.command_name
            )));
        }

        Ok(result)
    }
}

#[async_trait]
impl Tool for SlashCommandTool {
    fn name(&self) -> &str {
        "SlashCommand"
    }

    fn description(&self) -> &str {
        "Execute a custom slash command from .sage/commands. Only project/user custom commands \
         are allowed here; built-in CLI commands are intentionally excluded."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![ToolParameter::string(
                "command",
                "The slash command to execute with arguments, for example '/review-pr 123'",
            )],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let command = call.get_string("command").ok_or_else(|| {
            ToolError::InvalidArguments("Missing required parameter: command".to_string())
        })?;

        self.validate_command(&command)?;
        let invocation = self.parse_command(&command)?;
        let result = self.execute_command(&invocation).await?;

        let mut sections = Vec::new();
        if !result.context_messages.is_empty() {
            sections.push(result.context_messages.join("\n"));
        }
        if !result.expanded_prompt.trim().is_empty() {
            sections.push(result.expanded_prompt.clone());
        }

        if sections.is_empty() {
            return Err(ToolError::ExecutionFailed(format!(
                "Slash command '/{}' produced empty prompt output",
                invocation.command_name
            )));
        }

        let mut tool_result = ToolResult::success(&call.id, self.name(), sections.join("\n\n"))
            .with_metadata("command_name", serde_json::json!(invocation.command_name));

        if let Some(status) = result.status_message {
            tool_result = tool_result.with_metadata("status_message", serde_json::json!(status));
        }

        if let Some(tools) = result.tool_restrictions {
            tool_result = tool_result.with_metadata("allowed_tools", serde_json::json!(tools));
        }

        if let Some(model) = result.model_override {
            tool_result = tool_result.with_metadata("model_override", serde_json::json!(model));
        }

        Ok(tool_result)
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
    use std::fs;
    use tempfile::TempDir;

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

    fn write_custom_command(
        temp_dir: &TempDir,
        command_name: &str,
        prompt_template: &str,
    ) -> PathBuf {
        let commands_dir = temp_dir.path().join(".sage").join("commands");
        fs::create_dir_all(&commands_dir).unwrap();
        let command_file = commands_dir.join(format!("{}.md", command_name));
        fs::write(
            &command_file,
            format!(
                "---\ndescription: test command {}\n---\n{}",
                command_name, prompt_template
            ),
        )
        .unwrap();
        command_file
    }

    #[tokio::test]
    async fn test_custom_slash_command_execution() {
        let temp_dir = TempDir::new().unwrap();
        write_custom_command(&temp_dir, "review-pr", "Review PR #$ARG1");

        let tool = SlashCommandTool::with_working_directory(temp_dir.path());
        let call = create_tool_call("test-1", "SlashCommand", "/review-pr 123");

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        assert!(result.output.as_ref().unwrap().contains("Review PR #123"));
        assert_eq!(
            result.metadata.get("command_name"),
            Some(&serde_json::json!("review-pr"))
        );
    }

    #[tokio::test]
    async fn test_command_parsing_with_quotes() {
        let temp_dir = TempDir::new().unwrap();
        write_custom_command(&temp_dir, "greet", "Hello $ARG1 from $ARG2");

        let tool = SlashCommandTool::with_working_directory(temp_dir.path());
        let call = create_tool_call("test-2", "SlashCommand", "/greet \"Jane Doe\" team");

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.as_ref().unwrap();
        assert!(output.contains("Hello Jane Doe from team"));
    }

    #[tokio::test]
    async fn test_reject_builtin_command() {
        let temp_dir = TempDir::new().unwrap();
        let tool = SlashCommandTool::with_working_directory(temp_dir.path());
        let call = create_tool_call("test-builtin", "SlashCommand", "/help");

        let result = tool.execute(&call).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Built-in command"));
    }

    #[tokio::test]
    async fn test_unknown_command_error() {
        let temp_dir = TempDir::new().unwrap();
        let tool = SlashCommandTool::with_working_directory(temp_dir.path());
        let call = create_tool_call("test-unknown", "SlashCommand", "/does-not-exist");

        let result = tool.execute(&call).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unknown slash command")
        );
    }

    #[tokio::test]
    async fn test_invalid_command_format() {
        let tool = SlashCommandTool::new();
        let invalid = vec!["", "test", "/"];
        for cmd in invalid {
            let call = create_tool_call("invalid", "SlashCommand", cmd);
            assert!(
                tool.validate(&call).is_err(),
                "command should be invalid: {cmd}"
            );
        }
    }

    #[tokio::test]
    async fn test_tool_schema() {
        let tool = SlashCommandTool::new();
        let schema = tool.schema();

        assert_eq!(schema.name, "SlashCommand");
        assert!(!schema.description.is_empty());

        let params = schema.parameters.as_object().unwrap();
        assert!(params.contains_key("properties"));
        let properties = params.get("properties").unwrap().as_object().unwrap();
        assert!(properties.contains_key("command"));
    }
}
