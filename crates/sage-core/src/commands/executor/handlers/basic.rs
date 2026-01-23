//! Basic built-in command handlers
//!
//! Design principles:
//! 1. Local-first: Commands that can be handled locally should NOT call LLM
//! 2. Decoupled: Each command is independent and self-contained
//! 3. Reusable: Common utilities are shared across commands

use crate::config::load_config;
use crate::error::SageResult;

use super::super::types::CommandExecutor;
use crate::commands::types::{CommandInvocation, CommandResult};

/// Execute /help command - show help locally
pub(super) async fn execute_help(_invocation: &CommandInvocation) -> SageResult<CommandResult> {
    let help_text = r#"Sage Agent - AI-powered coding assistant

## Quick Start
Just type your request and press Enter. Sage will help you with coding tasks.

## Available Commands
- /help      - Show this help message
- /commands  - List all available commands
- /config    - Show current configuration
- /status    - Show agent status
- /clear     - Clear conversation history
- /compact   - Summarize conversation to save context
- /cost      - Show session cost and token usage
- /context   - Show context window usage
- /undo      - Undo last file changes (via git restore)
- /login     - Configure API credentials
- /output    - Switch output mode (streaming/batch/silent)
- /resume    - Resume a previous session
- /title     - Set session title

## Tips
- Use Tab to auto-complete commands
- Use ↑/↓ to navigate command suggestions
- Press Ctrl+C to cancel current operation
- Press ESC to clear input

For more information, visit: https://github.com/anthropics/sage"#;

    Ok(CommandResult::local(help_text))
}

/// Execute /clear command - clear conversation (handled by executor)
pub(super) async fn execute_clear() -> SageResult<CommandResult> {
    // Return interactive command for clear - the CLI will handle the actual clearing
    Ok(CommandResult::interactive(
        crate::commands::types::InteractiveCommand::Clear,
    )
    .with_status("Conversation cleared"))
}

/// Execute /compact command - needs LLM to summarize
pub(super) async fn execute_compact() -> SageResult<CommandResult> {
    // This genuinely needs LLM to summarize the conversation
    Ok(CommandResult::prompt(
        "Please summarize our conversation so far into a concise summary, then we can continue from there.",
    )
    .with_status("Compacting context..."))
}

/// Execute /init command - initialize .sage directory locally
pub(super) async fn execute_init() -> SageResult<CommandResult> {
    use std::fs;
    use std::path::Path;

    let sage_dir = Path::new(".sage");

    if sage_dir.exists() {
        return Ok(CommandResult::local(
            ".sage directory already exists.\n\nContents:\n- Use /config to view configuration\n- Use /commands to see available commands",
        ));
    }

    // Create .sage directory structure
    match fs::create_dir_all(sage_dir.join("commands")) {
        Ok(_) => {
            // Create default config
            let default_config = r#"{
  "version": "1.0",
  "commands": {}
}"#;
            let _ = fs::write(sage_dir.join("settings.json"), default_config);

            Ok(CommandResult::local(
                "Initialized .sage directory with:\n- .sage/settings.json\n- .sage/commands/\n\nYou can now add custom commands in .sage/commands/",
            ))
        }
        Err(e) => Ok(CommandResult::local(format!(
            "Failed to initialize .sage directory: {}",
            e
        ))),
    }
}

/// Execute /config command - show configuration locally
pub(super) async fn execute_config(invocation: &CommandInvocation) -> SageResult<CommandResult> {
    if invocation.arguments.is_empty() {
        // Show current configuration locally
        match load_config() {
            Ok(config) => {
                let mut output = String::from("Current Sage Configuration:\n\n");

                // Default provider
                output.push_str(&format!(
                    "## Default Provider\n{}\n\n",
                    config.get_default_provider()
                ));

                // Max steps
                output.push_str("## Execution Limits\n");
                match config.max_steps {
                    Some(steps) => output.push_str(&format!("- Max steps: {}\n", steps)),
                    None => output.push_str("- Max steps: unlimited\n"),
                }
                if let Some(budget) = config.total_token_budget {
                    output.push_str(&format!("- Token budget: {}\n", budget));
                }
                output.push('\n');

                // List configured providers
                output.push_str("## Configured Providers\n");
                for (name, params) in &config.model_providers {
                    output.push_str(&format!("- {} (model: {})\n", name, params.model));
                }
                output.push('\n');

                // Trajectory settings
                output.push_str("## Trajectory Recording\n");
                output.push_str(&format!("- Enabled: {}\n", config.trajectory.enabled));

                Ok(CommandResult::local(output))
            }
            Err(e) => Ok(CommandResult::local(format!("Error loading config: {}", e))),
        }
    } else {
        // Show usage for config modification
        Ok(CommandResult::local(
            "Configuration modification is not yet supported via CLI.\n\nTo modify configuration:\n1. Edit sage_config.json directly\n2. Or use environment variables (SAGE_DEFAULT_PROVIDER, etc.)",
        ))
    }
}

/// Execute /checkpoint command - create checkpoint locally
pub(super) async fn execute_checkpoint(
    invocation: &CommandInvocation,
) -> SageResult<CommandResult> {
    use std::process::Command;

    let name = invocation
        .arguments
        .first()
        .map(|s| s.as_str())
        .unwrap_or("checkpoint");

    // Use git stash to create a checkpoint
    let output = Command::new("git")
        .args(["stash", "push", "-m", &format!("sage-checkpoint: {}", name)])
        .output();

    match output {
        Ok(result) => {
            if result.status.success() {
                Ok(CommandResult::local(format!(
                    "Checkpoint '{}' created.\n\nUse '/restore {}' to restore this checkpoint.",
                    name, name
                )))
            } else {
                let stderr = String::from_utf8_lossy(&result.stderr);
                if stderr.contains("No local changes") {
                    Ok(CommandResult::local(
                        "No changes to checkpoint. Working directory is clean.",
                    ))
                } else {
                    Ok(CommandResult::local(format!(
                        "Failed to create checkpoint: {}",
                        stderr
                    )))
                }
            }
        }
        Err(e) => Ok(CommandResult::local(format!(
            "Failed to run git: {}\n\nMake sure you're in a git repository.",
            e
        ))),
    }
}

/// Execute /restore command - restore checkpoint locally
pub(super) async fn execute_restore(invocation: &CommandInvocation) -> SageResult<CommandResult> {
    use std::process::Command;

    if invocation.arguments.is_empty() {
        // List available checkpoints
        let output = Command::new("git")
            .args(["stash", "list"])
            .output();

        match output {
            Ok(result) => {
                let stdout = String::from_utf8_lossy(&result.stdout);
                if stdout.is_empty() {
                    Ok(CommandResult::local("No checkpoints available."))
                } else {
                    let mut list = String::from("Available checkpoints:\n\n");
                    for line in stdout.lines() {
                        if line.contains("sage-checkpoint:") {
                            list.push_str(&format!("- {}\n", line));
                        }
                    }
                    if list == "Available checkpoints:\n\n" {
                        list.push_str("(No sage checkpoints found. Use /checkpoint to create one.)");
                    }
                    Ok(CommandResult::local(list))
                }
            }
            Err(e) => Ok(CommandResult::local(format!("Failed to list checkpoints: {}", e))),
        }
    } else {
        let checkpoint_name = &invocation.arguments[0];

        // Find and restore the checkpoint
        let list_output = Command::new("git")
            .args(["stash", "list"])
            .output();

        match list_output {
            Ok(result) => {
                let stdout = String::from_utf8_lossy(&result.stdout);
                let mut stash_index = None;

                for (i, line) in stdout.lines().enumerate() {
                    if line.contains(&format!("sage-checkpoint: {}", checkpoint_name)) {
                        stash_index = Some(i);
                        break;
                    }
                }

                match stash_index {
                    Some(idx) => {
                        let restore = Command::new("git")
                            .args(["stash", "pop", &format!("stash@{{{}}}", idx)])
                            .output();

                        match restore {
                            Ok(r) if r.status.success() => Ok(CommandResult::local(format!(
                                "Checkpoint '{}' restored successfully.",
                                checkpoint_name
                            ))),
                            Ok(r) => Ok(CommandResult::local(format!(
                                "Failed to restore: {}",
                                String::from_utf8_lossy(&r.stderr)
                            ))),
                            Err(e) => Ok(CommandResult::local(format!("Failed to restore: {}", e))),
                        }
                    }
                    None => Ok(CommandResult::local(format!(
                        "Checkpoint '{}' not found.\n\nUse '/restore' to list available checkpoints.",
                        checkpoint_name
                    ))),
                }
            }
            Err(e) => Ok(CommandResult::local(format!("Failed to list checkpoints: {}", e))),
        }
    }
}

/// Execute /tasks command - show background tasks locally
pub(super) async fn execute_tasks() -> SageResult<CommandResult> {
    // TODO: Integrate with actual task tracking system
    Ok(CommandResult::local(
        "Background Tasks\n================\n\nNo background tasks running.\n\nBackground tasks will appear here when:\n- Long-running operations are in progress\n- Async file operations are pending",
    ))
}

/// Execute /undo command - undo file changes via git
pub(super) async fn execute_undo(_invocation: &CommandInvocation) -> SageResult<CommandResult> {
    use std::process::Command;

    // First check git status
    let status = Command::new("git")
        .args(["status", "--porcelain"])
        .output();

    match status {
        Ok(result) => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            if stdout.is_empty() {
                return Ok(CommandResult::local(
                    "No changes to undo. Working directory is clean.",
                ));
            }

            // Show what will be undone
            let mut output = String::from("Files with changes:\n\n");
            for line in stdout.lines() {
                output.push_str(&format!("  {}\n", line));
            }
            output.push_str("\nTo undo these changes, run:\n  git restore .\n\nOr use git restore <file> to undo specific files.");

            Ok(CommandResult::local(output))
        }
        Err(e) => Ok(CommandResult::local(format!(
            "Failed to check git status: {}\n\nMake sure you're in a git repository.",
            e
        ))),
    }
}

/// Execute /commands command - list all commands locally
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
                "- /{} - {}\n",
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
                "- /{} - {}\n",
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
                "- /{} - {}\n",
                cmd.name,
                cmd.description.as_deref().unwrap_or("No description")
            ));
        }
    }

    Ok(CommandResult::local(output))
}
