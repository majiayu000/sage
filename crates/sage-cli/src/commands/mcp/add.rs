//! MCP server add commands

use sage_core::config::{McpConfig, McpServerConfig};
use sage_core::error::{SageError, SageResult};
use std::collections::HashMap;
use std::path::Path;

/// Add a stdio MCP server
pub async fn add_server(
    name: &str,
    command: &str,
    args: Vec<String>,
    env_vars: Vec<String>,
    scope: &str,
) -> SageResult<()> {
    // Parse environment variables
    let env: HashMap<String, String> = env_vars
        .iter()
        .filter_map(|e| {
            let parts: Vec<&str> = e.splitn(2, '=').collect();
            if parts.len() == 2 {
                Some((parts[0].to_string(), parts[1].to_string()))
            } else {
                None
            }
        })
        .collect();

    let mut server_config = McpServerConfig::stdio(command, args);
    server_config.env = env;

    save_server_config(name, server_config, scope).await?;

    println!("Added MCP server '{}' ({})", name, scope);
    Ok(())
}

/// Add an MCP server from JSON configuration
pub async fn add_server_json(name: &str, json: &str, scope: &str) -> SageResult<()> {
    let server_config: McpServerConfig = serde_json::from_str(json).map_err(|e| {
        SageError::invalid_input(format!("Invalid JSON configuration: {}", e))
    })?;

    save_server_config(name, server_config, scope).await?;

    println!("Added MCP server '{}' from JSON ({})", name, scope);
    Ok(())
}

/// Add MCP servers from Claude Desktop configuration
pub async fn add_from_claude_desktop() -> SageResult<()> {
    let claude_config_path = get_claude_desktop_config_path()?;

    if !claude_config_path.exists() {
        return Err(SageError::invalid_input(
            "Claude Desktop configuration not found. Make sure Claude Desktop is installed.",
        ));
    }

    let content = std::fs::read_to_string(&claude_config_path)?;
    let claude_config: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
        SageError::invalid_input(format!("Failed to parse Claude Desktop config: {}", e))
    })?;

    // Extract mcpServers from Claude Desktop config
    let mcp_servers = claude_config
        .get("mcpServers")
        .and_then(|v| v.as_object())
        .ok_or_else(|| {
            SageError::invalid_input("No MCP servers found in Claude Desktop configuration")
        })?;

    let mut added_count = 0;
    for (name, config) in mcp_servers {
        // Convert Claude Desktop format to our format
        let server_config = convert_claude_desktop_server(config)?;
        save_server_config(name, server_config, "user").await?;
        println!("  Added: {}", name);
        added_count += 1;
    }

    println!(
        "\nImported {} MCP server(s) from Claude Desktop",
        added_count
    );
    Ok(())
}

/// Get Claude Desktop configuration path
fn get_claude_desktop_config_path() -> SageResult<std::path::PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| SageError::invalid_input("Cannot find home directory"))?;

    #[cfg(target_os = "macos")]
    let config_path = home.join("Library/Application Support/Claude/claude_desktop_config.json");

    #[cfg(target_os = "windows")]
    let config_path = home.join("AppData/Roaming/Claude/claude_desktop_config.json");

    #[cfg(target_os = "linux")]
    let config_path = home.join(".config/Claude/claude_desktop_config.json");

    Ok(config_path)
}

/// Convert Claude Desktop server config to our format
fn convert_claude_desktop_server(config: &serde_json::Value) -> SageResult<McpServerConfig> {
    let command = config
        .get("command")
        .and_then(|v| v.as_str())
        .ok_or_else(|| SageError::invalid_input("Server config missing 'command' field"))?;

    let args: Vec<String> = config
        .get("args")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let env: HashMap<String, String> = config
        .get("env")
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default();

    let mut server_config = McpServerConfig::stdio(command, args);
    server_config.env = env;

    Ok(server_config)
}

/// Save server configuration to the appropriate config file
async fn save_server_config(
    name: &str,
    server_config: McpServerConfig,
    scope: &str,
) -> SageResult<()> {
    let config_path = get_config_path(scope)?;

    // Load existing config or create new
    let mut mcp_config = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        serde_json::from_str::<McpConfig>(&content).unwrap_or_default()
    } else {
        McpConfig::default()
    };

    // Add/update server
    mcp_config.enabled = true;
    mcp_config.servers.insert(name.to_string(), server_config);

    // Ensure parent directory exists
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Save config
    let content = serde_json::to_string_pretty(&mcp_config)?;
    std::fs::write(&config_path, content)?;

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
