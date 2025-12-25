//! Schema definitions for memory tools

use sage_core::tools::{ToolParameter, ToolSchema};
use serde::{Deserialize, Serialize};

/// Input structure for Remember tool
/// Reserved for future typed input deserialization
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct RememberInput {
    pub memory: String,
    #[serde(default)]
    pub memory_type: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Create schema for Remember tool
pub fn remember_schema() -> ToolSchema {
    ToolSchema::new(
        "Remember",
        r#"Store information in long-term memory that persists across sessions.

Use this tool when:
- User explicitly asks you to remember something
- You learn an important fact about the user's preferences
- You discover something important about the codebase or project
- You learn lessons from mistakes or successes

Memory types:
- fact: General facts about the user, project, or codebase
- preference: User preferences for coding style, tools, etc.
- lesson: Lessons learned from tasks
- note: General session notes

Do NOT use for:
- Temporary information that's only relevant to the current task
- Information that's already in files (use the codebase instead)
- Sensitive information (passwords, secrets, etc.)"#,
        vec![
            ToolParameter::string("memory", "The concise (1-2 sentences) memory to store."),
            ToolParameter::optional_string(
                "memory_type",
                "Type of memory: fact, preference, lesson, note. Defaults to 'fact'.",
            ),
            ToolParameter::optional_string(
                "tags",
                "Comma-separated tags to categorize the memory (e.g., 'rust,coding,preference').",
            ),
        ],
    )
}

/// Create schema for SessionNotes tool
pub fn session_notes_schema() -> ToolSchema {
    ToolSchema::new(
        "SessionNotes",
        r#"View, search, or manage session notes and memories.

Actions:
- list: Show all memories (optionally filtered by type)
- search: Search memories by text
- delete: Delete a memory by ID
- clear: Clear all memories (use with caution)
- stats: Show memory statistics"#,
        vec![
            ToolParameter::string(
                "action",
                "Action to perform: list, search, delete, clear, stats",
            ),
            ToolParameter::string("query", "Search query (for 'search' action)"),
            ToolParameter::string(
                "memory_type",
                "Filter by type: fact, preference, lesson, note",
            ),
            ToolParameter::string("memory_id", "Memory ID (for 'delete' action)"),
        ],
    )
}
