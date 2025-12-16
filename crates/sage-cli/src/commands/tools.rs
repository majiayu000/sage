//! Tools command implementation

use crate::console::CLIConsole;
use sage_core::error::SageResult;

/// Show available tools and their descriptions
pub async fn show_tools() -> SageResult<()> {
    let console = CLIConsole::new(true);

    console.print_header("Available Tools");

    // Tool information - this should ideally come from the tools registry
    let tools = vec![
        (
            "str_replace_based_edit_tool",
            "Edit files using string replacement operations",
        ),
        (
            "sequentialthinking",
            "Sequential thinking and reasoning tool",
        ),
        ("json_edit_tool", "Edit JSON files with JSONPath operations"),
        ("task_done", "Mark a task as completed"),
        ("bash", "Execute bash commands in the system"),
    ];

    console.print_table_header(&["Tool Name", "Description"]);

    for (name, description) in &tools {
        console.print_table_row(&[name, description]);
    }

    console.info("");
    console.info(&format!("Total tools available: {}", tools.len()));
    console.info("Use these tools in your task descriptions to perform specific operations.");

    Ok(())
}
