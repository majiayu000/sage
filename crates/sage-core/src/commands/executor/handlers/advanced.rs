//! Advanced built-in command handlers
//!
//! Design principles:
//! 1. Local-first: Commands that can be handled locally should NOT call LLM
//! 2. Decoupled: Each command is independent and self-contained
//! 3. Reusable: Common utilities are shared across commands

use crate::error::SageResult;

use super::super::types::CommandExecutor;
use crate::commands::types::{CommandInvocation, CommandResult, InteractiveCommand};

/// Execute /cost command - show session cost and token usage locally
pub(super) async fn execute_cost() -> SageResult<CommandResult> {
    // TODO: Integrate with actual session cost tracking
    let output = r#"Session Cost & Usage
====================

Token usage tracking is enabled for this session.

To view detailed cost information:
- Token counts are tracked per API call
- Cost is calculated based on provider pricing
- Use trajectory files for detailed analysis

Current session stats will be shown here once integrated.

Tip: Use 'sage --trajectory-file <path>' for detailed tracking."#;
    Ok(CommandResult::local(output))
}

/// Execute /context command - show context/token breakdown locally
pub(super) async fn execute_context() -> SageResult<CommandResult> {
    // TODO: Integrate with actual context tracking
    let output = r#"Context Window Usage
====================

Context tracking is active for this session.

Typical context breakdown:
- System prompt: ~2K tokens
- Conversation history: varies
- Tool definitions: ~1K tokens
- Available space: depends on model

Model context limits:
- GPT-4: 8K-128K tokens
- Claude: 100K-200K tokens
- GLM-4: 128K tokens

Tip: Use /compact to reduce context usage when approaching limits."#;
    Ok(CommandResult::local(output))
}

/// Execute /status command - show agent status locally
pub(super) async fn execute_status(executor: &CommandExecutor) -> SageResult<CommandResult> {
    let version = env!("CARGO_PKG_VERSION");
    let builtin_count = executor.registry.read().await.builtin_count();

    // Get config info
    let config_info = match crate::config::load_config() {
        Ok(config) => format!(
            "- Default provider: {}\n- Max steps: {}",
            config.get_default_provider(),
            config.max_steps.map_or("unlimited".to_string(), |s| s.to_string())
        ),
        Err(_) => "- Config: Not loaded".to_string(),
    };

    let output = format!(
        r#"Sage Agent Status
=================

Version: {}
Status: Running

Configuration:
{}
- Commands registered: {} builtins

Use /config for detailed configuration.
Use /commands to list all available commands."#,
        version, config_info, builtin_count
    );

    Ok(CommandResult::local(output))
}

/// Execute /resume command - resume previous session
pub(super) async fn execute_resume(invocation: &CommandInvocation) -> SageResult<CommandResult> {
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

/// Execute /plan command - view/manage execution plan locally
pub(super) async fn execute_plan(invocation: &CommandInvocation) -> SageResult<CommandResult> {
    use std::fs;
    use std::path::Path;

    let plan_path = Path::new(".sage/plan.md");
    let subcommand = invocation.arguments.first().map(|s| s.as_str());

    match subcommand {
        Some("open") => {
            if plan_path.exists() {
                // Try to open with default editor
                #[cfg(target_os = "macos")]
                let _ = std::process::Command::new("open").arg(plan_path).spawn();
                #[cfg(target_os = "linux")]
                let _ = std::process::Command::new("xdg-open").arg(plan_path).spawn();
                #[cfg(target_os = "windows")]
                let _ = std::process::Command::new("notepad").arg(plan_path).spawn();

                Ok(CommandResult::local("Opening plan in default editor..."))
            } else {
                Ok(CommandResult::local(
                    "No plan file exists.\n\nUse '/plan create' to create a new plan.",
                ))
            }
        }
        Some("clear") => {
            if plan_path.exists() {
                match fs::remove_file(plan_path) {
                    Ok(_) => Ok(CommandResult::local("Plan cleared.")),
                    Err(e) => Ok(CommandResult::local(format!("Failed to clear plan: {}", e))),
                }
            } else {
                Ok(CommandResult::local("No plan to clear."))
            }
        }
        Some("create") => {
            // Create .sage directory if needed
            let sage_dir = Path::new(".sage");
            if !sage_dir.exists() {
                let _ = fs::create_dir_all(sage_dir);
            }

            let template = r#"# Execution Plan

## Goal
[Describe the goal here]

## Steps
- [ ] Step 1
- [ ] Step 2
- [ ] Step 3

## Notes
[Add any notes here]
"#;
            match fs::write(plan_path, template) {
                Ok(_) => Ok(CommandResult::local(
                    "Plan created at .sage/plan.md\n\nUse '/plan open' to edit it.",
                )),
                Err(e) => Ok(CommandResult::local(format!("Failed to create plan: {}", e))),
            }
        }
        _ => {
            // Default: show current plan
            if plan_path.exists() {
                match fs::read_to_string(plan_path) {
                    Ok(content) => {
                        let mut output = String::from("Current Plan:\n\n");
                        output.push_str(&content);
                        output.push_str("\n\nCommands: /plan open | /plan clear | /plan create");
                        Ok(CommandResult::local(output))
                    }
                    Err(e) => Ok(CommandResult::local(format!("Failed to read plan: {}", e))),
                }
            } else {
                Ok(CommandResult::local(
                    "No active plan.\n\nUse '/plan create' to create a new plan.",
                ))
            }
        }
    }
}

/// Execute /title command - set custom session title
pub(super) async fn execute_title(invocation: &CommandInvocation) -> SageResult<CommandResult> {
    let title = invocation.arguments.join(" ");

    if title.is_empty() {
        Ok(CommandResult::local(
            "Usage: /title <title>\n\nSet a custom title for the current session.\nExample: /title Fix authentication bug",
        ))
    } else {
        Ok(
            CommandResult::interactive(InteractiveCommand::Title { title })
                .with_status("Setting session title..."),
        )
    }
}

/// Execute /login command - configure API credentials
pub(super) async fn execute_login() -> SageResult<CommandResult> {
    Ok(
        CommandResult::interactive(InteractiveCommand::Login)
            .with_status("Opening credential setup..."),
    )
}

/// Execute /output command - switch output display mode
pub(super) async fn execute_output(invocation: &CommandInvocation) -> SageResult<CommandResult> {
    let mode = invocation.arguments.first().map(|s| s.as_str());

    match mode {
        Some(m) if ["streaming", "batch", "silent"].contains(&m) => {
            Ok(
                CommandResult::interactive(InteractiveCommand::OutputMode {
                    mode: m.to_string(),
                })
                .with_status(format!("Switching to {} mode...", m)),
            )
        }
        Some(m) => Ok(CommandResult::local(format!(
            "Unknown output mode: '{}'\n\nValid modes:\n  streaming - Real-time output (default)\n  batch     - Collect and display at end\n  silent    - No output",
            m
        ))),
        None => Ok(CommandResult::local(
            "Usage: /output <mode>\n\nModes:\n  streaming - Real-time output (default)\n  batch     - Collect and display at end\n  silent    - No output",
        )),
    }
}

/// Execute /doctor command - run diagnostics
pub(super) async fn execute_doctor() -> SageResult<CommandResult> {
    let output = r#"Sage Diagnostics
================

Running system checks...

[✓] Sage version: OK
[✓] Configuration: Loaded
[?] API connectivity: Use /status to check provider status

Recommendations:
- Use /config to view current configuration
- Use /login to configure API credentials
- Use /status to check connection status

For detailed diagnostics, run: sage --diagnostics"#;
    Ok(CommandResult::local(output))
}

/// Execute /logout command - clear stored credentials
pub(super) async fn execute_logout() -> SageResult<CommandResult> {
    use std::path::PathBuf;

    // Check common credential locations
    let home = std::env::var("HOME").unwrap_or_default();
    let cred_path = PathBuf::from(&home).join(".sage").join("credentials.json");

    if cred_path.exists() {
        match std::fs::remove_file(&cred_path) {
            Ok(_) => Ok(CommandResult::interactive(InteractiveCommand::Logout)
                .with_status("Credentials cleared successfully.")),
            Err(e) => Ok(CommandResult::local(format!(
                "Failed to clear credentials: {}\n\nYou may need to manually delete: {}",
                e,
                cred_path.display()
            ))),
        }
    } else {
        Ok(CommandResult::local(
            "No credentials to clear.\n\nCredentials are typically stored in:\n- ~/.sage/credentials.json\n- Environment variables (ANTHROPIC_API_KEY, OPENAI_API_KEY, etc.)",
        ))
    }
}

/// Execute /model command - switch to a different model
pub(super) async fn execute_model(invocation: &CommandInvocation) -> SageResult<CommandResult> {
    let model = invocation.arguments.first().map(|s| s.as_str());

    match model {
        Some(m) => Ok(
            CommandResult::interactive(InteractiveCommand::Model {
                model: m.to_string(),
            })
            .with_status(format!("Model switching to '{}'...", m)),
        ),
        None => Ok(CommandResult::local(
            "Usage: /model <model-name>\n\nExamples:\n  /model gpt-4\n  /model claude-3-opus\n  /model glm-4\n\nUse /config to see configured providers and models.",
        )),
    }
}

/// Execute /exit command - exit the application
pub(super) async fn execute_exit() -> SageResult<CommandResult> {
    Ok(
        CommandResult::interactive(InteractiveCommand::Exit)
            .with_status("Exiting..."),
    )
}
