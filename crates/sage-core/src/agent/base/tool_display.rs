//! Tool action display formatting

use crate::tools::ToolCall;
use colored::*;

/// Format tool action for display
pub(super) fn format_tool_action(tool_call: &ToolCall) -> String {
    match tool_call.name.as_str() {
        "bash" => {
            if let Some(command) = tool_call.arguments.get("command") {
                let cmd_str = command.as_str().unwrap_or("");
                if cmd_str.chars().count() > 60 {
                    let truncated: String = cmd_str.chars().take(57).collect();
                    format!("🖥️  {}...", truncated)
                } else {
                    format!("🖥️  {}", cmd_str)
                }
            } else {
                "🖥️  Running command".to_string()
            }
        }
        "str_replace_based_edit_tool" => {
            if let Some(action) = tool_call.arguments.get("action") {
                match action.as_str().unwrap_or("") {
                    "view" => {
                        if let Some(path) = tool_call.arguments.get("path") {
                            format!("📖 Reading: {}", path.as_str().unwrap_or(""))
                        } else {
                            "📖 Reading file".to_string()
                        }
                    }
                    "create" => {
                        if let Some(path) = tool_call.arguments.get("path") {
                            let content_preview = if let Some(content) =
                                tool_call.arguments.get("file_text")
                            {
                                let content_str = content.as_str().unwrap_or("");
                                if content_str.len() > 50 {
                                    format!(" ({}...)", &content_str[..47])
                                } else if !content_str.is_empty() {
                                    format!(" ({})", content_str)
                                } else {
                                    "".to_string()
                                }
                            } else {
                                "".to_string()
                            };
                            format!(
                                "📝 Creating: {}{}",
                                path.as_str().unwrap_or(""),
                                content_preview
                            )
                        } else {
                            "📝 Creating file".to_string()
                        }
                    }
                    "str_replace" => {
                        if let Some(path) = tool_call.arguments.get("path") {
                            format!("✏️ Editing: {}", path.as_str().unwrap_or(""))
                        } else {
                            "✏️ Editing file".to_string()
                        }
                    }
                    _ => {
                        if let Some(path) = tool_call.arguments.get("path") {
                            format!("📄 File op: {}", path.as_str().unwrap_or(""))
                        } else {
                            "📄 File operation".to_string()
                        }
                    }
                }
            } else {
                "📄 File operation".to_string()
            }
        }
        "task_done" => {
            if let Some(summary) = tool_call.arguments.get("summary") {
                let summary_str = summary.as_str().unwrap_or("");
                if summary_str.chars().count() > 50 {
                    let truncated: String = summary_str.chars().take(47).collect();
                    format!("✅ Done: {}...", truncated)
                } else {
                    format!("✅ Done: {}", summary_str)
                }
            } else {
                "✅ Task completed".to_string()
            }
        }
        "sequentialthinking" => {
            if let Some(thought) = tool_call.arguments.get("thought") {
                let thought_str = thought.as_str().unwrap_or("");
                if thought_str.chars().count() > 50 {
                    let truncated: String = thought_str.chars().take(47).collect();
                    format!("🧠 Thinking: {}...", truncated)
                } else {
                    format!("🧠 Thinking: {}", thought_str)
                }
            } else {
                "🧠 Thinking step by step".to_string()
            }
        }
        "json_edit_tool" => {
            if let Some(path) = tool_call.arguments.get("path") {
                format!("📝 JSON edit: {}", path.as_str().unwrap_or(""))
            } else {
                "📝 JSON operation".to_string()
            }
        }
        _ => format!("🔧 Using {}", tool_call.name),
    }
}

/// Display tool actions
pub(super) fn display_tool_actions(tool_calls: &[ToolCall]) {
    for tool_call in tool_calls {
        let action = format_tool_action(tool_call);
        println!("{}", action.blue());
    }
}
