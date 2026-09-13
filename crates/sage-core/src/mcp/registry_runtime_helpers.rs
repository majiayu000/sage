//! Free helpers for MCP registry runtime capability refresh.

use super::auth_status::McpAuthStatus;
use super::error::McpError;
use super::runtime_status::McpServerRuntimeStatus;
use super::source::MergedMcpServerSource;
use super::tool_trust::{McpToolTrustDecision, McpToolTrustStore, validate_tool_description_trust};
use super::types::McpTool;
use crate::config::McpServerConfig;
use std::collections::HashSet;

pub(crate) fn trusted_mcp_tools_for_server(
    server_name: &str,
    tools: Vec<McpTool>,
    trust_store: &mut McpToolTrustStore,
    warn_on_drift: bool,
) -> Result<Vec<(McpTool, McpToolTrustDecision)>, McpError> {
    // Reject duplicate names before baselines/routes: call_tool authorizes by
    // name only, so conflicting schemas in one listing are unsafe to expose.
    let mut seen_names = HashSet::new();
    let mut duplicate_names = HashSet::new();
    for tool in &tools {
        if !seen_names.insert(tool.name.as_str()) {
            duplicate_names.insert(tool.name.clone());
        }
    }
    if !duplicate_names.is_empty() {
        let mut names: Vec<_> = duplicate_names.into_iter().collect();
        names.sort();
        return Err(McpError::schema(format!(
            "MCP server '{server_name}' returned duplicate tool names: {}",
            names.join(", ")
        )));
    }

    let mut trusted_tools = Vec::with_capacity(tools.len());
    for tool in tools {
        validate_mcp_tool_schema(server_name, &tool)?;
        if let Err(error) = validate_tool_description_trust(server_name, &tool) {
            tracing::warn!(
                server = server_name,
                tool = tool.name.as_str(),
                error = %error,
                "Skipping untrusted MCP tool while keeping the server available"
            );
            continue;
        }
        let trust_decision = trust_store.check_tool(server_name, &tool);
        if let McpToolTrustDecision::Drift { previous, current } = &trust_decision {
            if !warn_on_drift {
                tracing::warn!(
                    server = server_name,
                    tool = tool.name.as_str(),
                    previous = previous.as_str(),
                    current = current.as_str(),
                    "MCP tool trust baseline drift detected; skipping tool (fail closed)"
                );
                continue;
            }
        }
        trusted_tools.push((tool, trust_decision));
    }
    Ok(trusted_tools)
}

pub(crate) fn log_mcp_tool_trust_decision(
    server_name: &str,
    tool_name: &str,
    decision: McpToolTrustDecision,
) {
    match decision {
        McpToolTrustDecision::BaselineCreated { hash } => {
            tracing::info!(
                server = server_name,
                tool = tool_name,
                hash = hash.as_str(),
                "Created MCP tool trust baseline"
            );
        }
        McpToolTrustDecision::Unchanged => {}
        McpToolTrustDecision::Drift { previous, current } => {
            tracing::warn!(
                server = server_name,
                tool = tool_name,
                previous = previous.as_str(),
                current = current.as_str(),
                "MCP tool description/schema trust baseline drift detected"
            );
        }
    }
}

pub(crate) fn ensure_supported_transport(config: &McpServerConfig) -> Result<(), McpError> {
    match config.transport.as_str() {
        "websocket" => Err(McpError::unsupported_transport(
            "websocket",
            "WebSocket MCP transport is not controlled by this runtime and fails closed",
        )),
        "stdio" => {
            let command = config.command.as_deref().unwrap_or_default();
            if matches!(command, "ssh" | "plink" | "nc" | "ncat") {
                return Err(McpError::unsupported_transport(
                    "stdio",
                    "Remote stdio MCP transport is not controlled by this runtime and fails closed",
                ));
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn validate_mcp_tool_schema(server_name: &str, tool: &McpTool) -> Result<(), McpError> {
    if tool.input_schema.is_null() {
        return Ok(());
    }
    let Some(schema) = tool.input_schema.as_object() else {
        return Err(McpError::schema(format!(
            "MCP server '{server_name}' returned non-object schema for tool '{}'",
            tool.name
        )));
    };
    if schema
        .get("properties")
        .is_some_and(|properties| !properties.is_object())
    {
        return Err(McpError::schema(format!(
            "MCP server '{server_name}' returned invalid properties schema for tool '{}'",
            tool.name
        )));
    }
    if schema
        .get("required")
        .is_some_and(|required| !required.is_array())
    {
        return Err(McpError::schema(format!(
            "MCP server '{server_name}' returned invalid required schema for tool '{}'",
            tool.name
        )));
    }
    Ok(())
}

pub(crate) fn refresh_status_auth(
    source: &MergedMcpServerSource,
    status: &mut McpServerRuntimeStatus,
) {
    status.auth =
        McpAuthStatus::from_server_config(&source.selected.server_id, &source.selected.config);
    if status.enabled
        && status.auth_blocks_tools()
        && !matches!(
            status.state,
            super::runtime_status::McpRuntimeState::AuthRequired
        )
    {
        status.state = super::runtime_status::McpRuntimeState::AuthRequired;
    } else if status.enabled
        && !status.auth_blocks_tools()
        && matches!(
            status.state,
            super::runtime_status::McpRuntimeState::AuthRequired
        )
    {
        status.state = super::runtime_status::McpRuntimeState::Disconnected;
    }
}
