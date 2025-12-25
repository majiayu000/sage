//! Built-in command handlers

use crate::error::SageResult;

use super::types::CommandExecutor;
use crate::commands::types::{CommandInvocation, CommandResult, InteractiveCommand, SlashCommand};

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
        _ => {
            // Fall back to prompt expansion
            let expanded = command.expand(&invocation.arguments);
            Ok(CommandResult::prompt(expanded))
        }
    }
}

/// Execute /help command
async fn execute_help(_invocation: &CommandInvocation) -> SageResult<CommandResult> {
    Ok(CommandResult::prompt(
        "Please provide help information about Sage Agent, including available commands and features.",
    )
    .with_status("Showing help..."))
}

/// Execute /clear command
async fn execute_clear() -> SageResult<CommandResult> {
    Ok(CommandResult::prompt("__CLEAR_CONVERSATION__").with_status("Conversation cleared"))
}

/// Execute /compact command
async fn execute_compact() -> SageResult<CommandResult> {
    Ok(CommandResult::prompt(
        "Please summarize our conversation so far into a concise summary, then we can continue from there.",
    )
    .with_status("Compacting context..."))
}

/// Execute /init command
async fn execute_init() -> SageResult<CommandResult> {
    Ok(CommandResult::prompt(
        "Please initialize a .sage directory in the current project with default configuration files including settings.json and commands directory.",
    )
    .with_status("Initializing Sage..."))
}

/// Execute /config command
async fn execute_config(invocation: &CommandInvocation) -> SageResult<CommandResult> {
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
async fn execute_checkpoint(invocation: &CommandInvocation) -> SageResult<CommandResult> {
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
async fn execute_restore(invocation: &CommandInvocation) -> SageResult<CommandResult> {
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
async fn execute_tasks() -> SageResult<CommandResult> {
    Ok(CommandResult::prompt(
        "List all running and recently completed background tasks with their status.",
    )
    .with_status("Listing tasks..."))
}

/// Execute /undo command
async fn execute_undo(_invocation: &CommandInvocation) -> SageResult<CommandResult> {
    let prompt = r#"Undo the last file changes in the CURRENT WORKING DIRECTORY ONLY. Follow these steps:

1. Run `git status` to see what files have uncommitted changes in this directory
2. Run `git diff` to see the specific changes
3. For each modified file, use `git restore <filename>` to revert it
4. Verify the restoration by checking the file contents

IMPORTANT: Only operate on files in the current working directory. Do NOT touch files outside this directory."#.to_string();

    Ok(CommandResult::prompt(prompt).with_status("Preparing undo..."))
}

/// Execute /commands command
async fn execute_commands(executor: &CommandExecutor) -> SageResult<CommandResult> {
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

/// Execute /cost command - show session cost and token usage
async fn execute_cost() -> SageResult<CommandResult> {
    // This is a local command that displays session statistics
    // The actual cost tracking would need to be integrated with the session system
    let output = r#"
Session Cost & Usage
====================

This command shows the total cost and token usage for the current session.

To get accurate cost information:
1. Token usage is tracked per API call
2. Cost is calculated based on the provider's pricing
3. Cache hits reduce costs significantly

Note: Cost tracking requires session recording to be enabled.
Use 'sage run --trajectory-file <path>' to enable detailed tracking.
"#;
    Ok(CommandResult::local(output).with_status("Showing cost..."))
}

/// Execute /context command - show context/token breakdown
async fn execute_context() -> SageResult<CommandResult> {
    // This is a local command that shows context window usage
    let output = r#"
Context Window Usage
====================

This command visualizes the current context window usage.

Context breakdown includes:
- System prompt tokens
- Conversation history tokens
- Tool definitions tokens
- Available remaining tokens

The context window limit depends on your model:
- GPT-4: 8K-128K tokens
- Claude: 100K-200K tokens
- GLM-4: 128K tokens

Use /compact to reduce context usage when approaching limits.
"#;
    Ok(CommandResult::local(output).with_status("Showing context..."))
}

/// Execute /status command - show agent status
async fn execute_status(executor: &CommandExecutor) -> SageResult<CommandResult> {
    // Get version from cargo
    let version = env!("CARGO_PKG_VERSION");

    let output = format!(
        r#"
Sage Agent Status
=================

Version: {}
Status: Running

Configuration:
- Config loaded from: sage_config.json
- Commands registered: {} builtins

For detailed provider and model info, check your sage_config.json file.
Use /doctor to diagnose any connection issues.
"#,
        version,
        executor.registry.read().await.builtin_count()
    );

    Ok(CommandResult::local(output).with_status("Showing status..."))
}

/// Execute /resume command - resume previous session
async fn execute_resume(invocation: &CommandInvocation) -> SageResult<CommandResult> {
    let session_id = invocation.arguments.first().cloned();
    let show_all = invocation
        .arguments
        .iter()
        .any(|a| a == "--all" || a == "-a");

    // Return an interactive command that the CLI will handle
    Ok(CommandResult::interactive(InteractiveCommand::Resume {
        session_id,
        show_all,
    })
    .with_status("Opening session selector..."))
}

/// Execute /plan command - view/manage execution plan
async fn execute_plan(invocation: &CommandInvocation) -> SageResult<CommandResult> {
    let subcommand = invocation.arguments.first().map(|s| s.as_str());

    match subcommand {
        Some("open") => {
            let prompt = "Open the current execution plan file in the default editor. The plan file is located at .sage/plan.md in the project directory.";
            Ok(CommandResult::prompt(prompt).with_status("Opening plan..."))
        }
        Some("clear") => {
            let prompt = "Clear the current execution plan by removing or emptying .sage/plan.md";
            Ok(CommandResult::prompt(prompt).with_status("Clearing plan..."))
        }
        Some("create") => {
            let prompt = r#"Create a new execution plan for the current task.

The plan should:
1. Analyze the current task requirements
2. Break down into actionable steps
3. Identify dependencies between steps
4. Save to .sage/plan.md

Ask the user what they want to accomplish if no task context is available."#;
            Ok(CommandResult::prompt(prompt).with_status("Creating plan..."))
        }
        _ => {
            // Default: show current plan or indicate no plan exists
            let prompt = r#"Check if an execution plan exists at .sage/plan.md.

If it exists:
- Display the current plan content
- Show plan status (completed steps, pending steps)
- Suggest '/plan open' to edit in editor

If no plan exists:
- Inform the user no plan is active
- Suggest '/plan create' to create a new plan"#;
            Ok(CommandResult::prompt(prompt).with_status("Checking plan..."))
        }
    }
}
