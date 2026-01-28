//! Permission handling for destructive operations
//!
//! This module handles user permission dialogs for dangerous commands
//! that require explicit user confirmation before execution.

use crate::tools::types::{ToolCall, ToolResult};
use std::io::{self, Write};

use super::event_manager::EventManager;
use super::tool_orchestrator::ToolOrchestrator;

/// User's choice in the permission dialog
#[derive(Debug, Clone, PartialEq)]
pub enum PermissionChoice {
    YesOnce,
    #[allow(dead_code)]
    YesAlways,
    NoOnce,
    #[allow(dead_code)]
    NoAlways,
    Cancelled,
}

/// Handle permission check for destructive operations
///
/// If the tool returns ConfirmationRequired error, this will:
/// 1. Stop the animation
/// 2. Show a permission dialog to the user
/// 3. If user confirms, re-execute with user_confirmed=true
/// 4. If user denies, return a rejection message
pub async fn execute_with_permission_check(
    tool_orchestrator: &ToolOrchestrator,
    event_manager: &EventManager,
    tool_call: &ToolCall,
    cancel_token: tokio_util::sync::CancellationToken,
) -> ToolResult {
    // First attempt - may fail with ConfirmationRequired
    let result = tool_orchestrator
        .execution_phase(tool_call, cancel_token.clone())
        .await;

    // Check if the result indicates confirmation is required
    if !result.success {
        if let Some(ref error_msg) = result.error {
            if error_msg.contains("DESTRUCTIVE COMMAND BLOCKED")
                || error_msg.contains("Confirmation required")
            {
                return handle_permission_dialog(
                    tool_orchestrator,
                    event_manager,
                    tool_call,
                    cancel_token,
                )
                .await;
            }
        }
    }

    result
}

/// Show a simple permission dialog to the user
fn show_permission_dialog_simple(tool_name: &str, command: &str) -> PermissionChoice {
    println!();
    println!("  \x1b[33m⚠ Permission Required\x1b[0m");
    println!();
    println!("  Tool: \x1b[1m{}\x1b[0m", tool_name);
    println!("  Command: \x1b[36m{}\x1b[0m", command);
    println!();
    println!("  \x1b[2mThis is a destructive operation that may make irreversible changes.\x1b[0m");
    println!();
    print!("  Allow this operation? [y/N]: ");
    io::stdout().flush().ok();

    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(_) => {
            let answer = input.trim().to_lowercase();
            if answer == "y" || answer == "yes" {
                PermissionChoice::YesOnce
            } else if answer.is_empty() || answer == "n" || answer == "no" {
                PermissionChoice::NoOnce
            } else {
                PermissionChoice::Cancelled
            }
        }
        Err(_) => PermissionChoice::Cancelled,
    }
}

/// Handle permission dialog for destructive operations
async fn handle_permission_dialog(
    tool_orchestrator: &ToolOrchestrator,
    event_manager: &EventManager,
    tool_call: &ToolCall,
    cancel_token: tokio_util::sync::CancellationToken,
) -> ToolResult {
    // Stop animation to show dialog
    event_manager.stop_animation().await;

    let command = tool_call
        .arguments
        .get("command")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown command");

    let choice = show_permission_dialog_simple(&tool_call.name, command);

    // Restart animation using the event manager
    // Note: start_animation_tool needs &mut self but we only have &self here
    // Just emit a thinking event instead for now

    match choice {
        PermissionChoice::YesOnce | PermissionChoice::YesAlways => {
            execute_confirmed(tool_orchestrator, tool_call, command, cancel_token).await
        }
        PermissionChoice::NoOnce | PermissionChoice::NoAlways => {
            create_rejection_result(tool_call, command)
        }
        PermissionChoice::Cancelled => create_cancelled_result(tool_call),
    }
}

/// Execute tool with user confirmation flag
async fn execute_confirmed(
    tool_orchestrator: &ToolOrchestrator,
    tool_call: &ToolCall,
    command: &str,
    cancel_token: tokio_util::sync::CancellationToken,
) -> ToolResult {
    let mut confirmed_call = tool_call.clone();
    confirmed_call
        .arguments
        .insert("user_confirmed".to_string(), serde_json::Value::Bool(true));

    tracing::info!(
        tool = %tool_call.name,
        command = %command,
        "user confirmed destructive operation"
    );

    tool_orchestrator
        .execution_phase(&confirmed_call, cancel_token)
        .await
}

/// Create rejection result when user denies operation
fn create_rejection_result(tool_call: &ToolCall, command: &str) -> ToolResult {
    tracing::info!(
        tool = %tool_call.name,
        command = %command,
        "user rejected destructive operation"
    );

    ToolResult::error(
        &tool_call.id,
        &tool_call.name,
        format!(
            "Operation cancelled by user. The user rejected the command: {}",
            command
        ),
    )
}

/// Create cancelled result when user cancels dialog
fn create_cancelled_result(tool_call: &ToolCall) -> ToolResult {
    tracing::info!(
        tool = %tool_call.name,
        "user cancelled permission dialog"
    );

    ToolResult::error(
        &tool_call.id,
        &tool_call.name,
        "Operation cancelled by user (Ctrl+C or empty input).",
    )
}
