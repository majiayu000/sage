//! Tool schema definitions for learning tools

use sage_core::tools::{ToolParameter, ToolSchema};

/// Create schema for the Learn tool
pub fn learn_tool_schema() -> ToolSchema {
    ToolSchema::new(
        "Learn",
        r#"Learn from user corrections and preferences to improve future interactions.

Use this tool when:
- User explicitly corrects your behavior ("don't do X, do Y instead")
- User states a preference ("I prefer X over Y")
- You discover a pattern in the user's workflow
- User teaches you something about their codebase or project

Pattern types:
- correction: User corrected something you did wrong
- preference: User preference for tool usage or workflow
- style: Coding style preference (formatting, naming)
- workflow: Workflow preference (commit frequency, testing approach)

Do NOT use for:
- One-off instructions that won't apply to future interactions
- Sensitive information
- Project-specific facts (use Remember tool instead)"#,
        vec![
            ToolParameter::string(
                "pattern_type",
                "Type of pattern: correction, preference, style, workflow",
            ),
            ToolParameter::string(
                "description",
                "Brief description of what was learned (1-2 sentences)",
            ),
            ToolParameter::string("rule", "The actual rule or behavior to follow"),
            ToolParameter::optional_string(
                "context",
                "Comma-separated context tags (e.g., 'rust,testing,bash')",
            ),
        ],
    )
}

/// Create schema for the LearningPatterns tool
pub fn learning_patterns_tool_schema() -> ToolSchema {
    ToolSchema::new(
        "LearningPatterns",
        r#"View, search, or manage learned patterns.

Actions:
- list: Show all patterns (optionally filtered by type)
- search: Search patterns by text
- delete: Delete a pattern by ID
- clear: Clear all patterns (use with caution)
- stats: Show learning statistics
- apply_decay: Apply time-based decay to patterns"#,
        vec![
            ToolParameter::string(
                "action",
                "Action to perform: list, search, delete, clear, stats, apply_decay",
            ),
            ToolParameter::optional_string("query", "Search query (for 'search' action)"),
            ToolParameter::optional_string(
                "pattern_type",
                "Filter by type: correction, preference, style, workflow",
            ),
            ToolParameter::optional_string("pattern_id", "Pattern ID (for 'delete' action)"),
        ],
    )
}
