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

#[tokio::test]
async fn unregister_server_waits_for_capability_refresh_lock() {
    use std::sync::Arc;
    use std::time::Duration;

    let registry = Arc::new(McpRegistry::new());
    let refresh_lock = registry.capability_refresh_lock("svc");
    let guard = refresh_lock.lock().await;

    let unregister = {
        let registry = Arc::clone(&registry);
        tokio::spawn(async move { registry.unregister_server("svc").await })
    };

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(
        !unregister.is_finished(),
        "unregister must wait on capability_refresh_lock held by a concurrent refresh"
    );
    drop(guard);
    unregister
        .await
        .expect("unregister task")
        .expect("unregister ok");
}
