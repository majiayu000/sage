//! Tests for plugin registry

use super::*;
use crate::plugins::TestPlugin;

#[test]
fn test_registry_register() {
    let registry = PluginRegistry::new();
    let plugin = Box::new(TestPlugin::new("test", "1.0.0"));

    assert!(registry.register(plugin).is_ok());
    assert!(registry.contains("test"));
    assert_eq!(registry.len(), 1);
}

#[test]
fn test_registry_duplicate() {
    let registry = PluginRegistry::new();
    let plugin1 = Box::new(TestPlugin::new("test", "1.0.0"));
    let plugin2 = Box::new(TestPlugin::new("test", "1.0.0"));

    assert!(registry.register(plugin1).is_ok());
    assert!(registry.register(plugin2).is_err());
}

#[tokio::test]
async fn test_registry_unregister() {
    let registry = PluginRegistry::new();
    let plugin = Box::new(TestPlugin::new("test", "1.0.0"));

    registry.register(plugin).unwrap();
    assert!(registry.unregister("test").await.is_ok());
    assert!(!registry.contains("test"));
}

#[tokio::test]
async fn test_registry_initialize() {
    let registry = PluginRegistry::new();
    let plugin = Box::new(TestPlugin::new("test", "1.0.0"));

    registry.register(plugin).unwrap();
    assert!(registry.initialize_plugin("test", vec![]).await.is_ok());

    let entry = registry.get("test").unwrap();
    assert_eq!(entry.state().await, PluginState::Active);
}

#[tokio::test]
async fn test_registry_enable_disable() {
    let registry = PluginRegistry::new();
    let plugin = Box::new(TestPlugin::new("test", "1.0.0"));

    registry.register(plugin).unwrap();
    registry.initialize_plugin("test", vec![]).await.unwrap();

    // Disable
    assert!(registry.disable_plugin("test").await.is_ok());
    {
        let entry = registry.get("test").unwrap();
        assert!(!entry.is_enabled().await);
        assert_eq!(entry.state().await, PluginState::Suspended);
    }

    // Enable
    assert!(registry.enable_plugin("test").await.is_ok());
    {
        let entry = registry.get("test").unwrap();
        assert!(entry.is_enabled().await);
        assert_eq!(entry.state().await, PluginState::Active);
    }
}

#[tokio::test]
async fn test_registry_plugin_infos() {
    let registry = PluginRegistry::new();
    registry
        .register(Box::new(TestPlugin::new("test1", "1.0.0")))
        .unwrap();
    registry
        .register(Box::new(TestPlugin::new("test2", "2.0.0")))
        .unwrap();

    let infos = registry.plugin_infos().await;
    assert_eq!(infos.len(), 2);
}

#[tokio::test]
async fn test_registry_capability_query() {
    let registry = PluginRegistry::new();
    registry
        .register(Box::new(TestPlugin::new("test", "1.0.0")))
        .unwrap();
    registry.initialize_plugin("test", vec![]).await.unwrap();

    let plugins = registry
        .plugins_with_capability(PluginCapability::Tools)
        .await;
    assert_eq!(plugins.len(), 1);
    assert_eq!(plugins[0], "test");

    let plugins = registry
        .plugins_with_capability(PluginCapability::Storage)
        .await;
    assert!(plugins.is_empty());
}

#[tokio::test]
async fn test_registry_shutdown_all() {
    let registry = PluginRegistry::new();
    registry
        .register(Box::new(TestPlugin::new("test1", "1.0.0")))
        .unwrap();
    registry
        .register(Box::new(TestPlugin::new("test2", "1.0.0")))
        .unwrap();

    registry.initialize_plugin("test1", vec![]).await.unwrap();
    registry.initialize_plugin("test2", vec![]).await.unwrap();

    let results = registry.shutdown_all().await;
    assert_eq!(results.len(), 2);
    for result in results {
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_plugin_entry_lifecycle() {
    let plugin = Box::new(TestPlugin::new("test", "1.0.0"));
    let entry = PluginEntry::new(plugin);

    // Initial state
    assert_eq!(entry.state().await, PluginState::Created);
    assert!(entry.is_enabled().await);
    assert_eq!(entry.name().await, "test");
    assert_eq!(entry.version().await, "1.0.0");

    // Initialize
    let ctx = PluginContext::new("test", "1.0.0");
    assert!(entry.initialize(&ctx).await.is_ok());
    assert_eq!(entry.state().await, PluginState::Active);

    // Suspend
    assert!(entry.suspend().await.is_ok());
    assert_eq!(entry.state().await, PluginState::Suspended);

    // Resume
    assert!(entry.resume().await.is_ok());
    assert_eq!(entry.state().await, PluginState::Active);

    // Shutdown
    assert!(entry.shutdown().await.is_ok());
    assert_eq!(entry.state().await, PluginState::Stopped);
}

#[tokio::test]
async fn test_plugin_entry_with_load_order() {
    let plugin = Box::new(TestPlugin::new("test", "1.0.0"));
    let entry = PluginEntry::with_load_order(plugin, 42);

    assert_eq!(entry.load_order(), 42);
}

#[tokio::test]
async fn test_plugin_entry_set_enabled() {
    let plugin = Box::new(TestPlugin::new("test", "1.0.0"));
    let entry = PluginEntry::new(plugin);

    assert!(entry.is_enabled().await);
    entry.set_enabled(false).await;
    assert!(!entry.is_enabled().await);
    entry.set_enabled(true).await;
    assert!(entry.is_enabled().await);
}
