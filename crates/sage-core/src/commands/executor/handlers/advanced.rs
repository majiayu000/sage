//! Advanced built-in command handlers

use crate::error::SageResult;

use super::super::types::CommandExecutor;
use crate::commands::types::{CommandInvocation, CommandResult, InteractiveCommand};

/// Execute /cost command - show session cost and token usage
pub(super) async fn execute_cost() -> SageResult<CommandResult> {
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
pub(super) async fn execute_context() -> SageResult<CommandResult> {
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
pub(super) async fn execute_status(executor: &CommandExecutor) -> SageResult<CommandResult> {
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

/// Execute /plan command - view/manage execution plan
pub(super) async fn execute_plan(invocation: &CommandInvocation) -> SageResult<CommandResult> {
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
