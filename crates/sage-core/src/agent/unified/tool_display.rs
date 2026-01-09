//! Tool display utilities for formatting tool execution output
//!
//! This module contains helper functions for displaying tool information
//! in the terminal, including icons, parameter formatting, and activity descriptions.

use std::collections::HashMap;

/// Get icon for specific tool type
pub fn get_tool_icon(tool_name: &str) -> &'static str {
    match tool_name.to_lowercase().as_str() {
        "bash" | "shell" | "execute" => "",
        "read" | "cat" => "",
        "write" | "edit" => "",
        "grep" | "search" => "",
        "glob" | "find" => "",
        "lsp" | "code" => "",
        "web_fetch" | "web_search" => "󰖟",
        "task" | "todo_write" => "",
        _ => "",
    }
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
