//! MCP serve command - run sage as an MCP server

use sage_core::error::SageResult;

/// Run sage as an MCP server
pub async fn serve_mcp(port: Option<u16>) -> SageResult<()> {
    let port = port.unwrap_or(3000);

    println!("Starting Sage MCP server on port {}...", port);
    println!();
    println!("Available capabilities:");
    println!("  - Tools: Sage built-in tools (bash, edit, read, etc.)");
    println!("  - Resources: File system access");
    println!("  - Prompts: Custom prompt templates");
    println!();
    println!("Connect using:");
    println!("  HTTP: http://localhost:{}/mcp", port);
    println!("  SSE:  http://localhost:{}/mcp/sse", port);
    println!();

    // TODO: Implement actual MCP server
    // This requires:
    // 1. HTTP server (axum or actix-web)
    // 2. MCP protocol handler
    // 3. Tool registration
    // 4. Resource provider
    // 5. Prompt provider

    println!("MCP server mode is not yet fully implemented.");
    println!("This feature is planned for a future release.");

    Ok(())
}
