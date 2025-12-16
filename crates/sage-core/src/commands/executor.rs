//! Slash command executor
//!
//! This module provides the command executor for processing
//! and executing slash commands.

use crate::error::{SageError, SageResult};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::registry::CommandRegistry;
use super::types::{CommandInvocation, CommandResult, SlashCommand};

/// Command executor for processing slash commands
pub struct CommandExecutor {
    /// Command registry
    registry: Arc<RwLock<CommandRegistry>>,
}

impl CommandExecutor {
    /// Create a new command executor
    pub fn new(registry: Arc<RwLock<CommandRegistry>>) -> Self {
        Self { registry }
    }

    /// Check if input is a slash command
    pub fn is_command(input: &str) -> bool {
        CommandInvocation::is_slash_command(input)
    }

    /// Process a potential slash command
    ///
    /// Returns Ok(Some(result)) if command was executed,
    /// Ok(None) if input is not a slash command,
    /// Err if command execution failed.
    pub async fn process(&self, input: &str) -> SageResult<Option<CommandResult>> {
        // Parse the command invocation
        let invocation = match CommandInvocation::parse(input) {
            Some(inv) => inv,
            None => return Ok(None),
        };

        // Look up the command
        let registry = self.registry.read().await;
        let command = registry.get(&invocation.command_name).cloned();
        drop(registry);

        match command {
            Some(cmd) => {
                let result = self.execute(&cmd, &invocation).await?;
                Ok(Some(result))
            }
            None => {
                // Unknown command - return helpful error
                Err(SageError::NotFound(format!(
                    "Unknown command: /{}. Use /commands to see available commands.",
                    invocation.command_name
                )))
            }
        }
    }

    /// Execute a specific command
    async fn execute(
        &self,
        command: &SlashCommand,
        invocation: &CommandInvocation,
    ) -> SageResult<CommandResult> {
        // Check minimum arguments
        let min_args = command.min_args();
        if invocation.arguments.len() < min_args {
            return Err(SageError::InvalidInput(format!(
                "Command /{} requires at least {} argument(s), got {}",
                command.name,
                min_args,
                invocation.arguments.len()
            )));
        }

        // Handle builtin commands specially
        if command.is_builtin {
            return self.execute_builtin(command, invocation).await;
        }

        // Expand the prompt template
        let expanded = command.expand(&invocation.arguments);

        Ok(CommandResult::prompt(expanded).with_status(format!(
            "/{} is running...",
            command.name
        )))
    }

    /// Execute a built-in command
    async fn execute_builtin(
        &self,
        command: &SlashCommand,
        invocation: &CommandInvocation,
    ) -> SageResult<CommandResult> {
        match command.name.as_str() {
            "help" => self.execute_help(invocation).await,
            "clear" => self.execute_clear().await,
            "compact" => self.execute_compact().await,
            "init" => self.execute_init().await,
            "config" => self.execute_config(invocation).await,
            "checkpoint" => self.execute_checkpoint(invocation).await,
            "restore" => self.execute_restore(invocation).await,
            "tasks" => self.execute_tasks().await,
            "commands" => self.execute_commands().await,
            _ => {
                // Fall back to prompt expansion
                let expanded = command.expand(&invocation.arguments);
                Ok(CommandResult::prompt(expanded))
            }
        }
    }

    /// Execute /help command
    async fn execute_help(&self, _invocation: &CommandInvocation) -> SageResult<CommandResult> {
        Ok(CommandResult::prompt(
            "Please provide help information about Sage Agent, including available commands and features.",
        )
        .with_status("Showing help..."))
    }

    /// Execute /clear command
    async fn execute_clear(&self) -> SageResult<CommandResult> {
        Ok(CommandResult::prompt("__CLEAR_CONVERSATION__")
            .with_status("Conversation cleared"))
    }

    /// Execute /compact command
    async fn execute_compact(&self) -> SageResult<CommandResult> {
        Ok(CommandResult::prompt(
            "Please summarize our conversation so far into a concise summary, then we can continue from there.",
        )
        .with_status("Compacting context..."))
    }

    /// Execute /init command
    async fn execute_init(&self) -> SageResult<CommandResult> {
        Ok(CommandResult::prompt(
            "Please initialize a .sage directory in the current project with default configuration files including settings.json and commands directory.",
        )
        .with_status("Initializing Sage..."))
    }

    /// Execute /config command
    async fn execute_config(&self, invocation: &CommandInvocation) -> SageResult<CommandResult> {
        if invocation.arguments.is_empty() {
            Ok(CommandResult::prompt(
                "Show the current Sage configuration settings.",
            ))
        } else {
            let prompt = format!(
                "Update Sage configuration: {}",
                invocation.arguments.join(" ")
            );
            Ok(CommandResult::prompt(prompt))
        }
    }

    /// Execute /checkpoint command
    async fn execute_checkpoint(&self, invocation: &CommandInvocation) -> SageResult<CommandResult> {
        let name = invocation.arguments.first().cloned();
        let prompt = match name {
            Some(n) => format!(
                "Create a checkpoint named '{}' of the current project state.",
                n
            ),
            None => "Create a checkpoint of the current project state.".to_string(),
        };
        Ok(CommandResult::prompt(prompt).with_status("Creating checkpoint..."))
    }

    /// Execute /restore command
    async fn execute_restore(&self, invocation: &CommandInvocation) -> SageResult<CommandResult> {
        let checkpoint_id = invocation.arguments.first().cloned();
        let prompt = match checkpoint_id {
            Some(id) => format!(
                "Restore the project to checkpoint '{}'. Show what will change before proceeding.",
                id
            ),
            None => "List available checkpoints that can be restored.".to_string(),
        };
        Ok(CommandResult::prompt(prompt).with_status("Preparing restore..."))
    }

    /// Execute /tasks command
    async fn execute_tasks(&self) -> SageResult<CommandResult> {
        Ok(CommandResult::prompt(
            "List all running and recently completed background tasks with their status.",
        )
        .with_status("Listing tasks..."))
    }

    /// Execute /commands command
    async fn execute_commands(&self) -> SageResult<CommandResult> {
        let registry = self.registry.read().await;
        let commands = registry.list();

        let mut output = String::from("Available slash commands:\n\n");

        // Group by source
        let mut builtins = Vec::new();
        let mut project = Vec::new();
        let mut user = Vec::new();

        for (cmd, source) in commands {
            match source {
                super::types::CommandSource::Builtin => builtins.push(cmd),
                super::types::CommandSource::Project => project.push(cmd),
                super::types::CommandSource::User => user.push(cmd),
            }
        }

        if !builtins.is_empty() {
            output.push_str("## Built-in Commands\n");
            for cmd in builtins {
                output.push_str(&format!(
                    "- **/{}`** - {}\n",
                    cmd.name,
                    cmd.description.as_deref().unwrap_or("No description")
                ));
            }
            output.push('\n');
        }

        if !project.is_empty() {
            output.push_str("## Project Commands\n");
            for cmd in project {
                output.push_str(&format!(
                    "- **/{}`** - {}\n",
                    cmd.name,
                    cmd.description.as_deref().unwrap_or("No description")
                ));
            }
            output.push('\n');
        }

        if !user.is_empty() {
            output.push_str("## User Commands\n");
            for cmd in user {
                output.push_str(&format!(
                    "- **/{}`** - {}\n",
                    cmd.name,
                    cmd.description.as_deref().unwrap_or("No description")
                ));
            }
        }

        Ok(CommandResult::prompt(output).show())
    }

    /// Reload commands from disk
    pub async fn reload(&self) -> SageResult<usize> {
        let mut registry = self.registry.write().await;
        // Keep builtins, clear others
        let builtins: Vec<_> = registry
            .list()
            .into_iter()
            .filter(|(_, src)| **src == super::types::CommandSource::Builtin)
            .map(|(cmd, _)| cmd.clone())
            .collect();

        registry.clear();

        for cmd in builtins {
            registry.register(cmd, super::types::CommandSource::Builtin);
        }

        registry.discover().await
    }

    /// Get command suggestions for autocomplete
    pub async fn get_suggestions(&self, prefix: &str) -> Vec<String> {
        let registry = self.registry.read().await;
        let prefix = prefix.trim_start_matches('/');

        registry
            .list()
            .iter()
            .filter(|(cmd, _)| cmd.name.starts_with(prefix))
            .map(|(cmd, _)| format!("/{}", cmd.name))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_executor() -> CommandExecutor {
        let mut registry = CommandRegistry::new("/project");
        registry.register_builtins();
        CommandExecutor::new(Arc::new(RwLock::new(registry)))
    }

    #[tokio::test]
    async fn test_is_command() {
        assert!(CommandExecutor::is_command("/help"));
        assert!(CommandExecutor::is_command("/test arg"));
        assert!(!CommandExecutor::is_command("help"));
        assert!(!CommandExecutor::is_command(""));
    }

    #[tokio::test]
    async fn test_process_builtin() {
        let executor = create_test_executor().await;

        let result = executor.process("/help").await.unwrap();
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_process_unknown_command() {
        let executor = create_test_executor().await;

        let result = executor.process("/nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_process_not_command() {
        let executor = create_test_executor().await;

        let result = executor.process("just text").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_execute_help() {
        let executor = create_test_executor().await;
        let result = executor.process("/help").await.unwrap().unwrap();

        assert!(!result.expanded_prompt.is_empty());
    }

    #[tokio::test]
    async fn test_execute_clear() {
        let executor = create_test_executor().await;
        let result = executor.process("/clear").await.unwrap().unwrap();

        assert!(result.expanded_prompt.contains("CLEAR"));
    }

    #[tokio::test]
    async fn test_execute_checkpoint_with_name() {
        let executor = create_test_executor().await;
        let result = executor
            .process("/checkpoint my-save")
            .await
            .unwrap()
            .unwrap();

        assert!(result.expanded_prompt.contains("my-save"));
    }

    #[tokio::test]
    async fn test_execute_commands() {
        let executor = create_test_executor().await;
        let result = executor.process("/commands").await.unwrap().unwrap();

        assert!(result.expanded_prompt.contains("help"));
        assert!(result.show_expansion);
    }

    #[tokio::test]
    async fn test_custom_command_execution() {
        let mut registry = CommandRegistry::new("/project");
        registry.register(
            SlashCommand::new("greet", "Say hello to $ARGUMENTS"),
            super::super::types::CommandSource::Project,
        );

        let executor = CommandExecutor::new(Arc::new(RwLock::new(registry)));
        let result = executor.process("/greet World").await.unwrap().unwrap();

        assert_eq!(result.expanded_prompt, "Say hello to World");
    }

    #[tokio::test]
    async fn test_command_min_args_validation() {
        let mut registry = CommandRegistry::new("/project");
        registry.register(
            SlashCommand::new("swap", "Swap $ARG1 with $ARG2"),
            super::super::types::CommandSource::Project,
        );

        let executor = CommandExecutor::new(Arc::new(RwLock::new(registry)));

        // Should fail with insufficient args
        let result = executor.process("/swap only-one").await;
        assert!(result.is_err());

        // Should succeed with enough args
        let result = executor.process("/swap a b").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_suggestions() {
        let executor = create_test_executor().await;

        let suggestions = executor.get_suggestions("/he").await;
        assert!(suggestions.contains(&"/help".to_string()));

        let suggestions = executor.get_suggestions("/ch").await;
        assert!(suggestions.contains(&"/checkpoint".to_string()));
    }

    #[tokio::test]
    async fn test_reload() {
        let executor = create_test_executor().await;

        // Should preserve builtins
        let count = executor.reload().await.unwrap();
        assert!(count >= 0); // May find files or not

        let result = executor.process("/help").await.unwrap();
        assert!(result.is_some()); // Builtins still work
    }
}
