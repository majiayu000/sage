//! MCP server list commands

use sage_core::config::McpConfig;
use sage_core::error::{SageError, SageResult};
use std::path::Path;

/// List all configured MCP servers
pub async fn list_servers(format: &str) -> SageResult<()> {
    let configs = load_all_configs()?;

    if format == "json" {
        let output = serde_json::to_string_pretty(&configs)?;
        println!("{}", output);
        return Ok(());
    }

    // Text format
    if configs.is_empty() {
        println!("No MCP servers configured.");
        println!("\nTo add a server:");
        println!("  sage mcp add <name> <command> [args...]");
        println!("  sage mcp add-json <name> '<json>'");
        println!("  sage mcp add-from-claude-desktop");
        return Ok(());
    }

    println!("Configured MCP Servers:\n");

    for (scope, config) in &configs {
        if config.servers.is_empty() {
            continue;
        }

        println!("  {} scope:", scope);
        for (name, server) in &config.servers {
            let status = if server.enabled { "enabled" } else { "disabled" };
            let transport = &server.transport;

            let target = match transport.as_str() {
                "stdio" => server.command.clone().unwrap_or_default(),
                "http" | "https" | "sse" => server.url.clone().unwrap_or_default(),
                _ => "unknown".to_string(),
            };

            println!("    {} ({}) - {} [{}]", name, transport, target, status);
        }
        println!();
    }

    Ok(())
}

/// Get details of a specific MCP server
pub async fn get_server(name: &str, format: &str) -> SageResult<()> {
    let configs = load_all_configs()?;

    // Find the server in any scope
    for (scope, config) in &configs {
        if let Some(server) = config.servers.get(name) {
            if format == "json" {
                let output = serde_json::json!({
                    "name": name,
                    "scope": scope,
                    "config": server
                });
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                println!("Server: {}", name);
                println!("  Scope: {}", scope);
                println!("  Transport: {}", server.transport);
                println!("  Enabled: {}", server.enabled);

                match server.transport.as_str() {
                    "stdio" => {
                        if let Some(cmd) = &server.command {
                            println!("  Command: {}", cmd);
                        }
                        if !server.args.is_empty() {
                            println!("  Args: {:?}", server.args);
                        }
                        if !server.env.is_empty() {
                            println!("  Environment:");
                            for (k, v) in &server.env {
                                // Mask sensitive values
                                let display_value = if k.to_lowercase().contains("key")
                                    || k.to_lowercase().contains("secret")
                                    || k.to_lowercase().contains("token")
                                {
                                    format!("{}...", &v[..v.len().min(4)])
                                } else {
                                    v.clone()
                                };
                                println!("    {}={}", k, display_value);
                            }
                        }
                    }
                    "http" | "https" | "sse" => {
                        if let Some(url) = &server.url {
                            println!("  URL: {}", url);
                        }
                        if !server.headers.is_empty() {
                            println!("  Headers:");
                            for (k, v) in &server.headers {
                                // Mask authorization headers
                                let display_value = if k.to_lowercase() == "authorization" {
                                    format!("{}...", &v[..v.len().min(10)])
                                } else {
                                    v.clone()
                                };
                                println!("    {}: {}", k, display_value);
                            }
                        }
                    }
                    _ => {}
                }

                if let Some(timeout) = server.timeout_secs {
                    println!("  Timeout: {}s", timeout);
                }
            }
            return Ok(());
        }
    }

    Err(SageError::invalid_input(format!(
        "MCP server '{}' not found",
        name
    )))
}

/// Load all MCP configurations from all scopes
fn load_all_configs() -> SageResult<Vec<(String, McpConfig)>> {
    let mut configs = Vec::new();

    // Load project config
    let project_path = Path::new(".mcp.json");
    if project_path.exists() {
        if let Ok(content) = std::fs::read_to_string(project_path) {
            if let Ok(config) = serde_json::from_str::<McpConfig>(&content) {
                configs.push(("project".to_string(), config));
            }
        }
    }

    // Load user config
    if let Some(home) = dirs::home_dir() {
        let user_path = home.join(".sage/mcp.json");
        if user_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&user_path) {
                if let Ok(config) = serde_json::from_str::<McpConfig>(&content) {
                    configs.push(("user".to_string(), config));
                }
            }
        }
    }

    Ok(configs)
}
