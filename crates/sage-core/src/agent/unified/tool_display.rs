//! Tool display utilities for formatting tool execution output
//!
//! This module contains helper functions for displaying tool information
//! in the terminal, including icons, parameter formatting, and activity descriptions.

use crate::tools::types::{ToolCall, ToolResult};
#[allow(deprecated)]
use crate::ui::bridge::global_adapter;
use crate::ui::Icons;
use colored::Colorize;
use std::collections::HashMap;

use super::event_manager::{EventManager, ExecutionEvent};

/// Get icon for specific tool type (delegates to Icons::for_tool)
#[allow(dead_code)]
pub fn get_tool_icon(tool_name: &str) -> &'static str {
    Icons::for_tool(tool_name)
}

/// Format tool parameters for display
pub fn format_tool_params(arguments: &HashMap<String, serde_json::Value>) -> String {
    let mut parts = Vec::new();

    // Show file_path or path if present
    if let Some(path) = arguments.get("file_path").or(arguments.get("path")) {
        if let Some(s) = path.as_str() {
            let display = if s.len() > 40 {
                format!("...{}", &s[s.len().saturating_sub(37)..])
            } else {
                s.to_string()
            };
            parts.push(display);
        }
    }

    // Show command if present (for bash) - UTF-8 safe
    if let Some(cmd) = arguments.get("command") {
        if let Some(s) = cmd.as_str() {
            let display = crate::utils::truncate_with_ellipsis(s, 50);
            parts.push(display);
        }
    }

    // Show pattern if present (for grep/glob)
    if let Some(pattern) = arguments.get("pattern") {
        if let Some(s) = pattern.as_str() {
            parts.push(format!("pattern={}", s));
        }
    }

    // Show query if present (for search) - UTF-8 safe
    if let Some(query) = arguments.get("query") {
        if let Some(s) = query.as_str() {
            let display = crate::utils::truncate_with_ellipsis(s, 30);
            parts.push(format!("query=\"{}\"", display));
        }
    }

    if parts.is_empty() {
        String::new()
    } else {
        parts.join(" ")
    }
}

/// Build activity description for progress tracking
///
/// This function builds a human-readable description of tool activity.
/// Currently unused but kept for potential future use.
#[allow(dead_code)]
pub fn build_activity_description(
    tool_name: &str,
    arguments: &HashMap<String, serde_json::Value>,
) -> String {
    let verb = match tool_name.to_lowercase().as_str() {
        "read" => "reading",
        "write" => "writing",
        "edit" => "editing",
        "bash" => "running",
        "glob" => "searching",
        "grep" => "searching",
        "web_fetch" => "fetching",
        "web_search" => "searching web",
        "task" => "running subagent",
        "lsp" => "analyzing",
        _ => "executing",
    };

    // Extract key info
    if let Some(path) = arguments.get("file_path").or(arguments.get("path")) {
        if let Some(s) = path.as_str() {
            let filename = std::path::Path::new(s)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(s);
            return format!("{} {}", verb, filename);
        }
    }

    if let Some(cmd) = arguments.get("command") {
        if let Some(s) = cmd.as_str() {
            let short = crate::utils::truncate_str(s, 30);
            return format!("{} '{}'", verb, short);
        }
    }

    if let Some(pattern) = arguments.get("pattern") {
        if let Some(s) = pattern.as_str() {
            return format!("{} for '{}'", verb, s);
        }
    }

    // Task tool: show description or prompt preview
    if tool_name.to_lowercase() == "task" {
        if let Some(desc) = arguments.get("description") {
            if let Some(s) = desc.as_str() {
                return format!("{}: {}", verb, crate::utils::truncate_str(s, 40));
            }
        }
        if let Some(prompt) = arguments.get("prompt") {
            if let Some(s) = prompt.as_str() {
                let preview = crate::utils::truncate_str(s, 40);
                return format!("{}: {}", verb, preview);
            }
        }
    }

    format!("{} {}", verb, tool_name)
}

/// Display tool execution start information
pub async fn display_tool_start(event_manager: &mut EventManager, tool_call: &ToolCall) {
    let params_preview = format_tool_params(&tool_call.arguments);

    // Claude Code style: blue filled circle for tools, with 2-space indent for result
    #[allow(deprecated)]
    if global_adapter().is_none() {
        println!();
        print!(
            "{} {}",
            Icons::message().bright_blue(),
            tool_call.name.bright_white().bold(),
        );
        // Show tool-specific icon and params
        if !params_preview.is_empty() {
            println!("({})", params_preview.dimmed());
        } else {
            println!();
        }
    }

    // Use tool params (or task description) as the detail for UI tool rows.
    let detail = if tool_call.name.to_lowercase() == "task" {
        tool_call
            .arguments
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| crate::utils::truncate_with_ellipsis(s, 40))
            .unwrap_or_else(|| "Task".to_string())
    } else {
        params_preview.clone()
    };

    // Emit tool execution started event with detail
    event_manager
        .emit_with_detail(ExecutionEvent::ToolExecutionStarted {
            tool_name: tool_call.name.clone(),
            tool_id: tool_call.id.clone(),
        }, detail)
        .await;
}

/// Display tool execution result
pub async fn display_tool_result(
    event_manager: &mut EventManager,
    tool_result: &ToolResult,
    duration_ms: u64,
) {
    // Emit tool execution completed event
    event_manager
        .emit(ExecutionEvent::ToolExecutionCompleted {
            tool_name: tool_result.tool_name.clone(),
            tool_id: tool_result.call_id.clone(),
            success: tool_result.success,
            duration_ms,
        })
        .await;

    // Claude Code style: result indicator with corner bracket
    #[allow(deprecated)]
    if global_adapter().is_none() {
        if tool_result.success {
            // Show brief output preview if available
            if let Some(ref output) = tool_result.output {
                let preview = output.lines().take(3).collect::<Vec<_>>().join("\n  ");
                if !preview.trim().is_empty() {
                    let truncated = crate::utils::truncate_with_ellipsis(&preview, 200);
                    println!("  {} {}", Icons::result().dimmed(), truncated.dimmed());
                }
            }
        } else {
            // Show error
            let err_msg = tool_result.error.as_deref().unwrap_or("Unknown error");
            let first_line = err_msg.lines().next().unwrap_or(err_msg);
            let truncated = crate::utils::truncate_with_ellipsis(first_line, 80);
            println!("  {} {}", Icons::result().red(), truncated.red());
        }
    }
}
