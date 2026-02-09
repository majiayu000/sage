//! Tools command implementation

use crate::console::CliConsole;
use sage_core::error::SageResult;

/// Show available tools and their descriptions
pub async fn show_tools() -> SageResult<()> {
    let console = CliConsole::new(true);

    console.print_header("Available Tools");

    let mut tools = sage_tools::get_default_tools();
    tools.sort_by(|a, b| a.name().cmp(b.name()));

    console.print_table_header(&["Tool Name", "Description"]);

    for tool in &tools {
        console.print_table_row(&[tool.name(), tool.description()]);
    }

    console.info("");
    console.info(&format!("Total tools available: {}", tools.len()));
    console.info("Use these tools in your task descriptions to perform specific operations.");

    Ok(())
}
