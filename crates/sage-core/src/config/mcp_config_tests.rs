use super::*;

#[test]
fn test_mcp_config_default() {
    let config = McpConfig::default();
    assert!(!config.enabled);
    assert!(config.servers.is_empty());
    assert_eq!(config.default_timeout_secs, 300);
    assert!(!config.default_timeout_secs_set);
    assert!(config.auto_connect);
    assert!(!config.warn_on_tool_trust_drift);
}

#[test]
fn test_mcp_config_warn_on_tool_trust_drift_opt_in() -> Result<(), serde_json::Error> {
    let implicit: McpConfig = serde_json::from_str("{}")?;
    let enabled: McpConfig = serde_json::from_str(r#"{"warn_on_tool_trust_drift": true}"#)?;
    let disabled: McpConfig = serde_json::from_str(r#"{"warn_on_tool_trust_drift": false}"#)?;

    assert!(!implicit.warn_on_tool_trust_drift);
    assert!(!implicit.warn_on_tool_trust_drift_set);
    assert!(enabled.warn_on_tool_trust_drift);
    assert!(enabled.warn_on_tool_trust_drift_set);
    assert!(!disabled.warn_on_tool_trust_drift);
    assert!(disabled.warn_on_tool_trust_drift_set);
    assert!(!serde_json::to_string(&McpConfig::default())?.contains("warn_on_tool_trust_drift"));
    assert!(serde_json::to_string(&enabled)?.contains("\"warn_on_tool_trust_drift\":true"));
    assert!(serde_json::to_string(&disabled)?.contains("\"warn_on_tool_trust_drift\":false"));
    Ok(())
}

#[test]
fn test_mcp_config_merge() {
    let mut config1 = McpConfig::default();
    let mut config2 = McpConfig::default();
    config2.enabled = true;
    config2.default_timeout_secs = 600;
    config2.default_timeout_secs_set = true;
    config2.auto_connect = false;
    config2
        .servers
        .insert("test".to_string(), McpServerConfig::stdio("test", vec![]));

    config1.merge(config2);
    assert!(config1.enabled);
    assert_eq!(config1.default_timeout_secs, 600);
    assert!(!config1.auto_connect);
    assert!(config1.servers.contains_key("test"));
}

#[test]
fn test_mcp_config_merge_default_does_not_override_disabled_auto_connect() {
    let mut config1 = McpConfig::default();
    config1.auto_connect = false;
    config1.auto_connect_set = true;

    config1.merge(McpConfig::default());

    assert!(!config1.auto_connect);
}

#[test]
fn test_mcp_config_merge_programmatic_true_warn_on_tool_trust_drift_without_presence_flag() {
    let mut base = McpConfig::default();
    let mut programmatic = McpConfig::default();
    programmatic.warn_on_tool_trust_drift = true;
    // Presence flag intentionally left false — mirrors `let mut c = McpConfig::default();
    // c.warn_on_tool_trust_drift = true;` construction.

    base.merge(programmatic);

    assert!(base.warn_on_tool_trust_drift);
    assert!(base.warn_on_tool_trust_drift_set);
}

#[test]
fn test_mcp_config_merge_honors_explicit_false_warn_on_tool_trust_drift() {
    let mut config1 = McpConfig::default();
    config1.warn_on_tool_trust_drift = true;
    config1.warn_on_tool_trust_drift_set = true;

    let mut config2 = McpConfig::default();
    config2.warn_on_tool_trust_drift = false;
    config2.warn_on_tool_trust_drift_set = true;

    config1.merge(config2);

    assert!(!config1.warn_on_tool_trust_drift);
    assert!(config1.warn_on_tool_trust_drift_set);
}

#[test]
fn test_mcp_config_merge_default_does_not_override_explicit_warn_on_tool_trust_drift() {
    let mut config1 = McpConfig::default();
    config1.warn_on_tool_trust_drift = true;
    config1.warn_on_tool_trust_drift_set = true;

    config1.merge(McpConfig::default());

    assert!(config1.warn_on_tool_trust_drift);
    assert!(config1.warn_on_tool_trust_drift_set);
}

#[test]
fn test_mcp_config_merge_default_does_not_override_explicit_timeout() {
    let mut config1 = McpConfig::default();
    config1.default_timeout_secs = 10;
    config1.default_timeout_secs_set = true;

    config1.merge(McpConfig::default());

    assert_eq!(config1.default_timeout_secs, 10);
    assert!(config1.default_timeout_secs_set);
}

#[test]
fn test_mcp_config_deserialize_tracks_explicit_auto_connect() {
    let implicit: McpConfig = serde_json::from_str("{}").unwrap();
    let explicit: McpConfig = serde_json::from_str(r#"{"auto_connect": true}"#).unwrap();

    assert!(implicit.auto_connect);
    assert!(!implicit.auto_connect_set);
    assert!(explicit.auto_connect);
    assert!(explicit.auto_connect_set);
}

#[test]
fn test_mcp_config_deserialize_tracks_explicit_timeout() {
    let implicit: McpConfig = serde_json::from_str("{}").unwrap();
    let explicit: McpConfig = serde_json::from_str(r#"{"default_timeout_secs": 10}"#).unwrap();

    assert_eq!(implicit.default_timeout_secs, 300);
    assert!(!implicit.default_timeout_secs_set);
    assert_eq!(explicit.default_timeout_secs, 10);
    assert!(explicit.default_timeout_secs_set);
}

#[test]
fn test_mcp_config_default_does_not_serialize_auto_connect() {
    let json = serde_json::to_string(&McpConfig::default()).unwrap();

    assert!(!json.contains("auto_connect"));
}

#[test]
fn test_mcp_config_default_does_not_serialize_implicit_timeout() -> Result<(), serde_json::Error> {
    let json = serde_json::to_string(&McpConfig::default())?;

    assert!(!json.contains("default_timeout_secs"));
    Ok(())
}

#[test]
fn test_mcp_config_serializes_explicit_default_timeout() -> Result<(), serde_json::Error> {
    let mut config = McpConfig::default();
    config.default_timeout_secs_set = true;

    let json = serde_json::to_string(&config)?;

    assert!(json.contains("\"default_timeout_secs\":300"));
    Ok(())
}

#[test]
fn test_mcp_config_serializes_non_default_timeout() -> Result<(), serde_json::Error> {
    let mut config = McpConfig::default();
    config.default_timeout_secs = 10;

    let json = serde_json::to_value(&config)?;

    assert_eq!(json["default_timeout_secs"], 10);
    Ok(())
}

#[test]
fn test_mcp_config_enabled_servers() {
    let mut config = McpConfig::default();
    config.servers.insert(
        "enabled".to_string(),
        McpServerConfig::stdio("test", vec![]),
    );
    let mut disabled = McpServerConfig::stdio("test", vec![]);
    disabled.enabled = false;
    config.servers.insert("disabled".to_string(), disabled);

    let enabled: Vec<_> = config.enabled_servers().collect();
    assert_eq!(enabled.len(), 1);
    assert!(enabled[0].0 == "enabled");
}

#[test]
fn test_mcp_config_get_timeout() {
    let mut config = McpConfig::default();
    config.default_timeout_secs = 300;

    // Server with custom timeout
    let mut server1 = McpServerConfig::stdio("test", vec![]);
    server1.timeout_secs = Some(120);
    config.servers.insert("custom".to_string(), server1);

    // Server without custom timeout
    let server2 = McpServerConfig::stdio("test", vec![]);
    config.servers.insert("default".to_string(), server2);

    assert_eq!(config.get_timeout("custom"), 120);
    assert_eq!(config.get_timeout("default"), 300);
    assert_eq!(config.get_timeout("nonexistent"), 300);
}

#[test]
fn test_mcp_server_config_stdio() {
    let config = McpServerConfig::stdio("python", vec!["-m".to_string(), "test".to_string()]);
    assert_eq!(config.transport, "stdio");
    assert_eq!(config.command, Some("python".to_string()));
    assert_eq!(config.args, vec!["-m", "test"]);
    assert!(config.enabled);
}

#[test]
fn test_mcp_server_config_http() {
    let config = McpServerConfig::http("http://localhost:8080");
    assert_eq!(config.transport, "http");
    assert_eq!(config.url, Some("http://localhost:8080".to_string()));
    assert!(config.enabled);
}

#[test]
fn test_mcp_server_config_websocket() {
    let config = McpServerConfig::websocket("ws://localhost:9000");
    assert_eq!(config.transport, "websocket");
    assert_eq!(config.url, Some("ws://localhost:9000".to_string()));
    assert!(config.enabled);
}

#[test]
fn test_mcp_server_config_with_env() {
    let config = McpServerConfig::stdio("test", vec![]).with_env("KEY", "value");
    assert_eq!(config.env.get("KEY"), Some(&"value".to_string()));
}

#[test]
fn test_mcp_server_config_with_header() {
    let config = McpServerConfig::http("http://test").with_header("Authorization", "Bearer token");
    assert_eq!(
        config.headers.get("Authorization"),
        Some(&"Bearer token".to_string())
    );
}

#[test]
fn test_mcp_server_config_with_timeout() {
    let config = McpServerConfig::stdio("test", vec![]).with_timeout(120);
    assert_eq!(config.timeout_secs, Some(120));
}

#[test]
fn test_default_functions() {
    assert_eq!(default_mcp_timeout(), 300);
    assert!(default_true());
}
