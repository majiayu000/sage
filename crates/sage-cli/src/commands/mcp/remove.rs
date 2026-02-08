//! MCP server remove command

use sage_core::config::McpConfig;
use sage_core::error::{SageError, SageResult};
use std::path::Path;

/// Remove an MCP server
pub async fn remove_server(name: &str, scope: &str) -> SageResult<()> {
    let config_path = get_config_path(scope)?;

    if !config_path.exists() {
        return Err(SageError::invalid_input(format!(
            "No MCP configuration found in {} scope",
            scope
        )));
    }

    let content = std::fs::read_to_string(&config_path)?;
    let mut config: McpConfig = serde_json::from_str(&content).map_err(|e| {
        SageError::invalid_input(format!("Failed to parse MCP config: {}", e))
    })?;

    if config.servers.remove(name).is_none() {
        return Err(SageError::invalid_input(format!(
            "MCP server '{}' not found in {} scope",
            name, scope
        )));
    }

    // Save updated config
    let content = serde_json::to_string_pretty(&config)?;
    std::fs::write(&config_path, content)?;

    println!("Removed MCP server '{}' from {} scope", name, scope);
    Ok(())
}

/// Get configuration file path based on scope
fn get_config_path(scope: &str) -> SageResult<std::path::PathBuf> {
    match scope {
        "project" => Ok(Path::new(".mcp.json").to_path_buf()),
        "user" | _ => {
            let home = dirs::home_dir()
                .ok_or_else(|| SageError::invalid_input("Cannot find home directory"))?;
            Ok(home.join(".sage/mcp.json"))
        }
    }
}
