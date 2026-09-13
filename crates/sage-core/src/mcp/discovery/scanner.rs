//! MCP server discovery from various sources

use super::types::DiscoverySource;
use crate::config::{McpConfig, McpServerConfig};
use crate::mcp::error::McpError;
use std::path::PathBuf;
use tracing::debug;

/// Discover servers from a single source
pub async fn discover_from_source(
    source: DiscoverySource,
) -> Result<(McpConfig, Vec<(String, McpServerConfig)>), McpError> {
    match source {
        DiscoverySource::Config(config) => Ok(extract_servers_from_config(config)),
        DiscoverySource::Environment(var_name) => discover_from_environment(&var_name).await,
        DiscoverySource::File(path) => discover_from_file(&path).await,
        DiscoverySource::Standard => discover_from_standard_paths().await,
    }
}

/// Extract enabled servers from config
fn extract_servers_from_config(config: McpConfig) -> (McpConfig, Vec<(String, McpServerConfig)>) {
    let servers = if config.enabled && config.auto_connect {
        config
            .enabled_servers()
            .map(|(name, cfg)| (name.clone(), cfg.clone()))
            .collect()
    } else {
        Vec::new()
    };
    (config, servers)
}

/// Discover servers from environment variable
async fn discover_from_environment(
    var_name: &str,
) -> Result<(McpConfig, Vec<(String, McpServerConfig)>), McpError> {
    let value = std::env::var(var_name)
        .map_err(|_| McpError::connection(format!("Environment variable {} not set", var_name)))?;

    let config: McpConfig = serde_json::from_str(&value)
        .map_err(|e| McpError::protocol(format!("Invalid JSON in {}: {}", var_name, e)))?;

    Ok(extract_servers_from_config(config))
}

/// Discover servers from a file
async fn discover_from_file(
    path: &PathBuf,
) -> Result<(McpConfig, Vec<(String, McpServerConfig)>), McpError> {
    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| McpError::connection(format!("Failed to read file {:?}: {}", path, e)))?;

    let config: McpConfig = serde_json::from_str(&content)
        .map_err(|e| McpError::protocol(format!("Invalid JSON in {:?}: {}", path, e)))?;

    Ok(extract_servers_from_config(config))
}

/// Discover servers from standard paths
async fn discover_from_standard_paths()
-> Result<(McpConfig, Vec<(String, McpServerConfig)>), McpError> {
    discover_from_candidate_paths(&get_standard_mcp_paths()).await
}

async fn discover_from_candidate_paths(
    standard_paths: &[PathBuf],
) -> Result<(McpConfig, Vec<(String, McpServerConfig)>), McpError> {
    for path in standard_paths {
        if path.exists() {
            debug!("Checking standard MCP config path: {:?}", path);
            match discover_from_file(path).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    debug!("No valid MCP config at {:?}: {}", path, e);
                }
            }
        }
    }

    // Treat a complete miss as discovery failure (not a default config) so
    // callers do not apply McpConfig::default() and silently reset an existing
    // warn-mode trust policy to fail-closed.
    Err(McpError::connection(
        "No MCP configuration found in standard paths",
    ))
}

/// Get standard MCP configuration paths
pub fn get_standard_mcp_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // Current directory
    paths.push(PathBuf::from("mcp.json"));
    paths.push(PathBuf::from(".mcp.json"));

    // Home directory
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".config/sage/mcp.json"));
        paths.push(home.join(".sage/mcp.json"));
    }

    // XDG config directory
    if let Some(config_dir) = dirs::config_dir() {
        paths.push(config_dir.join("sage/mcp.json"));
    }

    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn empty_candidate_paths_are_error_not_default_config() {
        let err = discover_from_candidate_paths(&[])
            .await
            .expect_err("miss must be Err, not Ok(default)");
        assert!(
            err.to_string()
                .contains("No MCP configuration found in standard paths"),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn missing_candidate_file_is_error_not_default_config() {
        let missing = PathBuf::from("/tmp/__sage_missing_standard_mcp_config__.json");
        let err = discover_from_candidate_paths(&[missing])
            .await
            .expect_err("missing file must be Err, not Ok(default)");
        assert!(
            err.to_string()
                .contains("No MCP configuration found in standard paths"),
            "unexpected error: {err}"
        );
    }
}
