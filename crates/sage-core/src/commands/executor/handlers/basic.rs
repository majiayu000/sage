//! Basic built-in command handlers

use crate::error::SageResult;

use super::super::types::CommandExecutor;
use crate::commands::types::{CommandInvocation, CommandResult};

/// Execute /help command
pub(super) async fn execute_help(_invocation: &CommandInvocation) -> SageResult<CommandResult> {
    Ok(CommandResult::prompt(
        "Please provide help information about Sage Agent, including available commands and features.",
    )
    .with_status("Showing help..."))
}

/// Execute /clear command
pub(super) async fn execute_clear() -> SageResult<CommandResult> {
    Ok(CommandResult::prompt("__CLEAR_CONVERSATION__").with_status("Conversation cleared"))
}

/// Execute /compact command
pub(super) async fn execute_compact() -> SageResult<CommandResult> {
    Ok(CommandResult::prompt(
        "Please summarize our conversation so far into a concise summary, then we can continue from there.",
    )
    .with_status("Compacting context..."))
}

/// Execute /init command
pub(super) async fn execute_init() -> SageResult<CommandResult> {
    Ok(CommandResult::prompt(
        "Please initialize a .sage directory in the current project with default configuration files including settings.json and commands directory.",
    )
    .with_status("Initializing Sage..."))
}

/// Execute /config command
pub(super) async fn execute_config(invocation: &CommandInvocation) -> SageResult<CommandResult> {
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
pub(super) async fn execute_checkpoint(
    invocation: &CommandInvocation,
) -> SageResult<CommandResult> {
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
pub(super) async fn execute_restore(invocation: &CommandInvocation) -> SageResult<CommandResult> {
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
pub(super) async fn execute_tasks() -> SageResult<CommandResult> {
    Ok(CommandResult::prompt(
        "List all running and recently completed background tasks with their status.",
    )
    .with_status("Listing tasks..."))
}

/// Execute /undo command
pub(super) async fn execute_undo(_invocation: &CommandInvocation) -> SageResult<CommandResult> {
    let prompt = r#"Undo the last file changes in the CURRENT WORKING DIRECTORY ONLY. Follow these steps:

1. Run `git status` to see what files have uncommitted changes in this directory
2. Run `git diff` to see the specific changes
3. For each modified file, use `git restore <filename>` to revert it
4. Verify the restoration by checking the file contents

IMPORTANT: Only operate on files in the current working directory. Do NOT touch files outside this directory."#.to_string();

    Ok(CommandResult::prompt(prompt).with_status("Preparing undo..."))
}

/// Execute /commands command
pub(super) async fn execute_commands(executor: &CommandExecutor) -> SageResult<CommandResult> {
    let registry = executor.registry.read().await;
    let commands = registry.list();

    let mut output = String::from("Available slash commands:\n\n");

    // Group by source
    let mut builtins = Vec::new();
    let mut project = Vec::new();
    let mut user = Vec::new();

    for (cmd, source) in commands {
        match source {
            crate::commands::types::CommandSource::Builtin => builtins.push(cmd),
            crate::commands::types::CommandSource::Project => project.push(cmd),
            crate::commands::types::CommandSource::User => user.push(cmd),
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
