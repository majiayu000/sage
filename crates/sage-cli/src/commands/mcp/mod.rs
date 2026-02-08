//! MCP (Model Context Protocol) CLI subcommands
//!
//! Provides commands for managing MCP servers:
//! - sage mcp add <name> <command> [args...]  - Add a stdio MCP server
//! - sage mcp add-json <name> <json>          - Add server from JSON config
//! - sage mcp remove <name>                   - Remove a server
//! - sage mcp list                            - List all servers
//! - sage mcp get <name>                      - Get server details
//! - sage mcp serve                           - Run sage as MCP server

mod add;
mod list;
mod remove;
mod serve;

pub use add::{add_server, add_server_json};
pub use list::{list_servers, get_server};
pub use remove::remove_server;
pub use serve::serve_mcp;

use clap::Subcommand;
use sage_core::error::SageResult;

/// MCP subcommand actions
#[derive(Subcommand, Clone, Debug)]
pub enum McpAction {
    /// Add a new MCP server (stdio transport)
    Add {
        /// Server name (unique identifier)
        name: String,
        /// Command to execute
        command: String,
        /// Command arguments
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
        /// Environment variables (KEY=VALUE format)
        #[arg(short, long = "env", value_name = "KEY=VALUE")]
        env: Vec<String>,
        /// Scope: "user" (global) or "project" (local .mcp.json)
        #[arg(short, long, default_value = "user")]
        scope: String,
    },

    /// Add a new MCP server from JSON configuration
    AddJson {
        /// Server name (unique identifier)
        name: String,
        /// JSON configuration string
        json: String,
        /// Scope: "user" (global) or "project" (local .mcp.json)
        #[arg(short, long, default_value = "user")]
        scope: String,
    },

    /// Add MCP servers from Claude Desktop configuration
    #[command(name = "add-from-claude-desktop")]
    AddFromClaudeDesktop,

    /// Remove an MCP server
    Remove {
        /// Server name to remove
        name: String,
        /// Scope: "user" (global) or "project" (local .mcp.json)
        #[arg(short, long, default_value = "user")]
        scope: String,
    },

    /// List all configured MCP servers
    List {
        /// Output format: "text" or "json"
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Get details of a specific MCP server
    Get {
        /// Server name
        name: String,
        /// Output format: "text" or "json"
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Run sage as an MCP server
    Serve {
        /// Port to listen on (for HTTP transport)
        #[arg(short, long)]
        port: Option<u16>,
    },

    /// Reset project-specific MCP choices
    #[command(name = "reset-project-choices")]
    ResetProjectChoices,
}

/// Execute MCP subcommand
pub async fn execute(action: McpAction) -> SageResult<()> {
    match action {
        McpAction::Add {
            name,
            command,
            args,
            env,
            scope,
        } => add_server(&name, &command, args, env, &scope).await,
        McpAction::AddJson { name, json, scope } => add_server_json(&name, &json, &scope).await,
        McpAction::AddFromClaudeDesktop => add::add_from_claude_desktop().await,
        McpAction::Remove { name, scope } => remove_server(&name, &scope).await,
        McpAction::List { format } => list_servers(&format).await,
        McpAction::Get { name, format } => get_server(&name, &format).await,
        McpAction::Serve { port } => serve_mcp(port).await,
        McpAction::ResetProjectChoices => reset_project_choices().await,
    }
}

/// Reset project-specific MCP choices
async fn reset_project_choices() -> SageResult<()> {
    let project_config = std::path::Path::new(".mcp.json");
    if project_config.exists() {
        std::fs::remove_file(project_config)?;
        println!("Project MCP choices have been reset.");
    } else {
        println!("No project MCP configuration found.");
    }
    Ok(())
}
