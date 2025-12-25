//! Core types for command executor

use crate::error::SageResult;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::commands::registry::CommandRegistry;
use crate::commands::types::{CommandInvocation, CommandResult, SlashCommand};

/// Command executor for processing slash commands
pub struct CommandExecutor {
    /// Command registry
    pub(super) registry: Arc<RwLock<CommandRegistry>>,
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
        super::executor::process_command(self, input).await
    }

    /// Execute a specific command
    pub(super) async fn execute(
        &self,
        command: &SlashCommand,
        invocation: &CommandInvocation,
    ) -> SageResult<CommandResult> {
        super::executor::execute_command(self, command, invocation).await
    }

    /// Execute a built-in command
    pub(super) async fn execute_builtin(
        &self,
        command: &SlashCommand,
        invocation: &CommandInvocation,
    ) -> SageResult<CommandResult> {
        super::handlers::execute_builtin(self, command, invocation).await
    }

    /// Reload commands from disk
    pub async fn reload(&self) -> SageResult<usize> {
        super::executor::reload_commands(self).await
    }

    /// Get command suggestions for autocomplete
    pub async fn get_suggestions(&self, prefix: &str) -> Vec<String> {
        super::executor::get_command_suggestions(self, prefix).await
    }
}
