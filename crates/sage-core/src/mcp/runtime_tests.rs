use super::{
    McpAuthState, McpError, McpFailureKind, McpRegistry, McpRuntimeState, McpServerSource,
    McpSourceKind, McpSourceSet, merge_mcp_sources,
};
use crate::config::{McpAuthConfig, McpAuthKind, McpServerConfig};
use crate::mcp::registry::ToolRoute;
use crate::plugins::PackageMcpServerRegistration;
use serde_json::json;
use std::path::PathBuf;

fn registry_with_source(source: McpServerSource) -> McpRegistry {
    let registry = McpRegistry::new();
    let source_set = merge_mcp_sources([source]).expect("source set should merge");
    registry.apply_source_set(source_set);
    registry
}

#[tokio::test]
async fn mcp_auth_required_blocks_tool_execution_with_recovery_prompt() {
    let registry = registry_with_source(McpServerSource::direct(
        "secure",
        McpServerConfig::http("https://mcp.example.test").with_auth(McpAuthConfig {
            required: true,
            kind: McpAuthKind::OAuth,
            token_env: None,
            authorization_url: Some("https://auth.example.test/start".to_string()),
            scopes: vec!["docs.read".to_string()],
        }),
        true,
    ));

    let err = registry
        .call_tool("mcp__secure__read", json!({}))
        .await
        .expect_err("auth should block tools");

    match err {
        McpError::AuthRequired { prompt, .. } => {
            assert_eq!(
                prompt.authorization_url.as_deref(),
                Some("https://auth.example.test/start")
            );
            assert_eq!(prompt.scopes, vec!["docs.read"]);
        }
        other => panic!("expected auth_required, got {other:?}"),
    }
}

#[tokio::test]
async fn mcp_runtime_unsupported_transport_fails_closed() {
    let registry = registry_with_source(McpServerSource::direct(
        "future",
        McpServerConfig::websocket("ws://localhost:9000"),
        true,
    ));

    let err = registry
        .connect_configured_server("future")
        .await
        .expect_err("websocket should fail closed");

    assert!(matches!(err, McpError::UnsupportedTransport { .. }));
    let status = registry
        .server_runtime_status("future")
        .expect("runtime status");
    assert_eq!(status.state, McpRuntimeState::UnsupportedTransport);
    assert_eq!(
        status.last_error.expect("structured error").kind,
        McpFailureKind::UnsupportedTransport
    );
}

#[tokio::test]
async fn mcp_runtime_disconnect_and_retry_update_structured_status() {
    let registry = registry_with_source(McpServerSource::direct(
        "offline",
        McpServerConfig::stdio("__sage_missing_mcp_binary__", Vec::new()),
        true,
    ));

    let retry = registry.retry_configured_server("offline").await;
    assert!(retry.is_err());
    let failed = registry
        .server_runtime_status("offline")
        .expect("failed status");
    assert_eq!(failed.state, McpRuntimeState::ConnectionError);
    assert!(failed.last_connect_attempt.is_some());

    let disconnected = registry
        .disconnect_configured_server("offline")
        .await
        .expect("disconnect should be controlled");
    assert_eq!(disconnected.status.state, McpRuntimeState::Disconnected);
}

#[test]
fn mcp_package_disabled_removes_source_from_runtime_set() {
    let registration = PackageMcpServerRegistration {
        package_id: "pkg.docs".to_string(),
        asset_id: "docs".to_string(),
        package_root: PathBuf::from("/tmp/pkg.docs"),
        config: McpServerConfig::stdio("docs-server", Vec::new()),
    };
    let registry = registry_with_source(McpServerSource::package(&registration));

    assert_eq!(
        registry
            .server_runtime_status("docs")
            .expect("status")
            .source
            .kind,
        McpSourceKind::Package
    );

    registry.apply_source_set(McpSourceSet::default());

    assert!(registry.server_runtime_status("docs").is_none());
    assert!(registry.configured_server_names().is_empty());
}

#[test]
fn mcp_source_replacement_clears_stale_routes() {
    let registration = PackageMcpServerRegistration {
        package_id: "pkg.docs".to_string(),
        asset_id: "docs".to_string(),
        package_root: PathBuf::from("/tmp/pkg.docs"),
        config: McpServerConfig::stdio("docs-server", Vec::new()),
    };
    let registry = registry_with_source(McpServerSource::package(&registration));
    registry.tool_mapping.insert(
        "mcp__docs__read".to_string(),
        ToolRoute {
            server_name: "docs".to_string(),
            remote_name: "read".to_string(),
        },
    );
    registry
        .resource_mapping
        .insert("file:///docs".to_string(), "docs".to_string());
    registry
        .prompt_mapping
        .insert("summarize".to_string(), "docs".to_string());

    registry.apply_source_set(McpSourceSet::default());

    assert!(registry.tool_mapping.is_empty());
    assert!(registry.resource_mapping.is_empty());
    assert!(registry.prompt_mapping.is_empty());
}

#[tokio::test]
#[serial_test::serial]
async fn mcp_auth_retry_refreshes_recovery_status() {
    const TOKEN_ENV: &str = "SAGE_GH87_TEST_MCP_TOKEN";
    unsafe {
        std::env::remove_var(TOKEN_ENV);
    }
    let registry = registry_with_source(McpServerSource::direct(
        "secure",
        McpServerConfig::stdio("__sage_missing_mcp_binary__", Vec::new()).with_auth(
            McpAuthConfig {
                required: true,
                kind: McpAuthKind::Bearer,
                token_env: Some(TOKEN_ENV.to_string()),
                authorization_url: None,
                scopes: Vec::new(),
            },
        ),
        true,
    ));

    let first = registry.connect_configured_server("secure").await;
    assert!(matches!(first, Err(McpError::AuthRequired { .. })));

    unsafe {
        std::env::set_var(TOKEN_ENV, "test-token");
    }
    let retry = registry.retry_configured_server("secure").await;
    unsafe {
        std::env::remove_var(TOKEN_ENV);
    }

    assert!(matches!(retry, Err(McpError::Connection { .. })));
    let status = registry
        .server_runtime_status("secure")
        .expect("runtime status");
    assert_eq!(status.auth.state, McpAuthState::Authorized);
    assert_eq!(status.state, McpRuntimeState::ConnectionError);
}
