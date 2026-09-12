use super::*;

#[test]
fn test_registry_creation() {
    let registry = McpRegistry::new();
    assert!(registry.server_names().is_empty());
}

#[test]
fn test_namespaced_tool_name() {
    let namespaced = McpToolAdapter::namespaced_tool_name("filesystem-server", "Read File");
    assert_eq!(namespaced, "mcp__filesystem_server__read_file");
}

#[test]
fn test_transport_config() {
    let config = TransportConfig::stdio("echo", vec!["hello".to_string()]);
    assert!(matches!(config, TransportConfig::Stdio { .. }));
}

#[tokio::test]
async fn set_warn_on_tool_trust_drift_updates_flag_without_clients() {
    let registry = McpRegistry::new();
    assert!(!registry.warn_on_tool_trust_drift());
    registry.set_warn_on_tool_trust_drift(true).await;
    assert!(registry.warn_on_tool_trust_drift());
    registry.set_warn_on_tool_trust_drift(false).await;
    assert!(!registry.warn_on_tool_trust_drift());
}
