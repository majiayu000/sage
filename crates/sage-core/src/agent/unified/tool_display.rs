//! Tool display utilities for formatting tool execution output
//!
//! This module contains helper functions for displaying tool information
//! in the terminal, including icons, parameter formatting, and activity descriptions.

use crate::tools::types::{ToolCall, ToolResult};
use crate::ui::Icons;
use colored::Colorize;
use std::collections::HashMap;

use super::event_manager::{EventManager, ExecutionEvent};

/// Get icon for specific tool type (delegates to Icons::for_tool)
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
    let tool_icon = get_tool_icon(&tool_call.name);
    let params_preview = format_tool_params(&tool_call.arguments);

    println!();
    println!(
        "  {} {} {}",
        tool_icon.bright_magenta(),
        tool_call.name.bright_magenta().bold(),
        params_preview.dimmed()
    );

    // Emit tool execution started event
    event_manager
        .emit(ExecutionEvent::ToolExecutionStarted {
            tool_name: tool_call.name.clone(),
            tool_id: tool_call.id.clone(),
        })
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

    let status_icon = if tool_result.success {
        "✓".green()
    } else {
        "✗".red()
    };

    print!("    {} ", status_icon);
    if tool_result.success {
        println!("{} ({}ms)", "done".green(), duration_ms);
    } else {
        println!("{} ({}ms)", "failed".red(), duration_ms);
        if let Some(ref err) = tool_result.error {
            let first_line = err.lines().next().unwrap_or(err);
            let truncated = crate::utils::truncate_with_ellipsis(first_line, 60);
            println!("      {}", truncated.dimmed());
        }
    }
}
