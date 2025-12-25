//! Command execution logic

use crate::error::{SageError, SageResult};

use super::types::CommandExecutor;
use crate::commands::types::{CommandInvocation, CommandResult, SlashCommand};

/// Process a potential slash command
///
/// Returns Ok(Some(result)) if command was executed,
/// Ok(None) if input is not a slash command,
/// Err if command execution failed.
pub(super) async fn process_command(
    executor: &CommandExecutor,
    input: &str,
) -> SageResult<Option<CommandResult>> {
    // Parse the command invocation
    let invocation = match CommandInvocation::parse(input) {
        Some(inv) => inv,
        None => return Ok(None),
    };

    // Look up the command
    let registry = executor.registry.read().await;
    let command = registry.get(&invocation.command_name).cloned();
    drop(registry);

    match command {
        Some(cmd) => {
            let result = executor.execute(&cmd, &invocation).await?;
            Ok(Some(result))
        }
        None => {
            // Unknown command - return helpful error
            Err(SageError::not_found(format!(
                "Unknown command: /{}. Use /commands to see available commands.",
                invocation.command_name
            )))
        }
    }
}

/// Execute a specific command
pub(super) async fn execute_command(
    executor: &CommandExecutor,
    command: &SlashCommand,
    invocation: &CommandInvocation,
) -> SageResult<CommandResult> {
    // Check minimum arguments
    let min_args = command.min_args();
    if invocation.arguments.len() < min_args {
        return Err(SageError::invalid_input(format!(
            "Command /{} requires at least {} argument(s), got {}",
            command.name,
            min_args,
            invocation.arguments.len()
        )));
    }

    // Handle builtin commands specially
    if command.is_builtin {
        return executor.execute_builtin(command, invocation).await;
    }

    // Expand the prompt template
    let expanded = command.expand(&invocation.arguments);

    Ok(CommandResult::prompt(expanded).with_status(format!("/{} is running...", command.name)))
}

/// Reload commands from disk
pub(super) async fn reload_commands(executor: &CommandExecutor) -> SageResult<usize> {
    let mut registry = executor.registry.write().await;
    // Keep builtins, clear others
    let builtins: Vec<_> = registry
        .list()
        .into_iter()
        .filter(|(_, src)| **src == crate::commands::types::CommandSource::Builtin)
        .map(|(cmd, _)| cmd.clone())
        .collect();

    registry.clear();

    for cmd in builtins {
        registry.register(cmd, crate::commands::types::CommandSource::Builtin);
    }

    registry.discover().await
}

/// Get command suggestions for autocomplete
pub(super) async fn get_command_suggestions(
    executor: &CommandExecutor,
    prefix: &str,
) -> Vec<String> {
    let registry = executor.registry.read().await;
    let prefix = prefix.trim_start_matches('/');

    registry
        .list()
        .iter()
        .filter(|(cmd, _)| cmd.name.starts_with(prefix))
        .map(|(cmd, _)| format!("/{}", cmd.name))
        .collect()
}
