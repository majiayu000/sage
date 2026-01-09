//! Built-in command handlers

mod advanced;
mod basic;

use crate::error::SageResult;

use super::types::CommandExecutor;
use crate::commands::types::{CommandInvocation, CommandResult, SlashCommand};

use advanced::*;
use basic::*;

/// Execute a built-in command
pub(super) async fn execute_builtin(
    executor: &CommandExecutor,
    command: &SlashCommand,
    invocation: &CommandInvocation,
) -> SageResult<CommandResult> {
    match command.name.as_str() {
        "help" => execute_help(invocation).await,
        "clear" => execute_clear().await,
        "compact" => execute_compact().await,
        "init" => execute_init().await,
        "config" => execute_config(invocation).await,
        "checkpoint" => execute_checkpoint(invocation).await,
        "restore" => execute_restore(invocation).await,
        "tasks" => execute_tasks().await,
        "commands" => execute_commands(executor).await,
        "undo" => execute_undo(invocation).await,
        "cost" => execute_cost().await,
        "context" => execute_context().await,
        "status" => execute_status(executor).await,
        "resume" => execute_resume(invocation).await,
        "plan" => execute_plan(invocation).await,
        "title" => execute_title(invocation).await,
        "login" => execute_login().await,
        _ => {
            // Fall back to prompt expansion
            let expanded = command.expand(&invocation.arguments);
            Ok(CommandResult::prompt(expanded))
        }
    }
}
