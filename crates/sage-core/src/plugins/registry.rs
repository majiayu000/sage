//! Plugin registry for managing plugins

use super::{
    Plugin, PluginCapability, PluginContext, PluginError, PluginInfo, PluginLifecycle,
    PluginPermission, PluginResult, PluginState,
};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Plugin entry in the registry
///
/// Uses interior mutability with `RwLock` to allow safe concurrent access
/// to the plugin and its lifecycle state without requiring external synchronization.
pub struct PluginEntry {
    /// The plugin instance (behind RwLock for async mutable access)
    plugin: RwLock<Box<dyn Plugin>>,

    /// Plugin lifecycle manager (behind RwLock for async mutable access)
    lifecycle: RwLock<PluginLifecycle>,

    /// Whether the plugin is enabled (behind RwLock for mutable access)
    enabled: RwLock<bool>,

    /// Plugin context
    context: RwLock<Option<PluginContext>>,

    /// Load order (lower = earlier) - immutable after creation
    load_order: usize,
}

impl PluginEntry {
    /// Create new plugin entry
    pub fn new(plugin: Box<dyn Plugin>) -> Self {
        Self {
            plugin: RwLock::new(plugin),
            lifecycle: RwLock::new(PluginLifecycle::new()),
            enabled: RwLock::new(true),
            context: RwLock::new(None),
            load_order: 0,
        }
    }

    /// Create new plugin entry with load order
    pub fn with_load_order(plugin: Box<dyn Plugin>, load_order: usize) -> Self {
        Self {
            plugin: RwLock::new(plugin),
            lifecycle: RwLock::new(PluginLifecycle::new()),
            enabled: RwLock::new(true),
            context: RwLock::new(None),
            load_order,
        }
    }

    /// Get plugin info
    pub async fn info(&self) -> PluginInfo {
        let plugin = self.plugin.read().await;
        let lifecycle = self.lifecycle.read().await;
        let enabled = *self.enabled.read().await;
        PluginInfo::from_plugin(plugin.as_ref(), lifecycle.state(), enabled)
    }

    /// Get the current lifecycle state
    pub async fn state(&self) -> PluginState {
        self.lifecycle.read().await.state()
    }

    /// Check if the plugin is enabled
    pub async fn is_enabled(&self) -> bool {
        *self.enabled.read().await
    }

    /// Set enabled status
    pub async fn set_enabled(&self, enabled: bool) {
        *self.enabled.write().await = enabled;
    }

    /// Get the load order
    pub fn load_order(&self) -> usize {
        self.load_order
    }

    /// Get plugin name
    pub async fn name(&self) -> String {
        self.plugin.read().await.name().to_string()
    }

    /// Get plugin version
    pub async fn version(&self) -> String {
        self.plugin.read().await.version().to_string()
    }

    /// Get plugin capabilities
    pub async fn capabilities(&self) -> Vec<PluginCapability> {
        self.plugin.read().await.capabilities()
    }

    /// Get tools provided by this plugin
    pub async fn get_tools(&self) -> Vec<Arc<dyn crate::tools::base::Tool>> {
        self.plugin.read().await.get_tools()
    }

    /// Set the plugin context
    pub async fn set_context(&self, ctx: PluginContext) {
        *self.context.write().await = Some(ctx);
    }

    /// Initialize the plugin with lifecycle management
    ///
    /// This method safely handles the lifecycle state transition and plugin
    /// initialization without requiring unsafe code.
    pub async fn initialize(&self, ctx: &PluginContext) -> PluginResult<()> {
        let mut lifecycle = self.lifecycle.write().await;
        let mut plugin = self.plugin.write().await;
        lifecycle.initialize(plugin.as_mut(), ctx).await
    }

    /// Shutdown the plugin with lifecycle management
    pub async fn shutdown(&self) -> PluginResult<()> {
        let mut lifecycle = self.lifecycle.write().await;
        let mut plugin = self.plugin.write().await;
        lifecycle.shutdown(plugin.as_mut()).await
    }

    /// Suspend the plugin with lifecycle management
    pub async fn suspend(&self) -> PluginResult<()> {
        let mut lifecycle = self.lifecycle.write().await;
        let mut plugin = self.plugin.write().await;
        lifecycle.suspend(plugin.as_mut()).await
    }

    /// Resume the plugin with lifecycle management
    pub async fn resume(&self) -> PluginResult<()> {
        let mut lifecycle = self.lifecycle.write().await;
        let mut plugin = self.plugin.write().await;
        lifecycle.resume(plugin.as_mut()).await
    }
}

/// Plugin registry for managing all plugins
///
/// Uses `DashMap` for thread-safe concurrent access to plugins.
/// Each `PluginEntry` handles its own interior mutability via `RwLock`,
/// so we don't need additional `Arc<RwLock<>>` wrapping at the registry level.
pub struct PluginRegistry {
    /// Registered plugins - DashMap provides thread-safe access
    plugins: DashMap<String, PluginEntry>,

    /// Plugin load order counter
    load_order: std::sync::atomic::AtomicUsize,

    /// Default permissions granted to all plugins
    default_permissions: Vec<PluginPermission>,

    /// Whether to require explicit permission approval
    require_permission_approval: bool,
}

impl PluginRegistry {
    /// Create new plugin registry
    pub fn new() -> Self {
        Self {
            plugins: DashMap::new(),
            load_order: std::sync::atomic::AtomicUsize::new(0),
            default_permissions: vec![PluginPermission::ReadFiles, PluginPermission::ConfigAccess],
            require_permission_approval: false,
        }
    }

    /// Create registry with explicit permission approval
    pub fn with_permission_approval() -> Self {
        let mut registry = Self::new();
        registry.require_permission_approval = true;
        registry
    }

    /// Set default permissions
    pub fn set_default_permissions(&mut self, permissions: Vec<PluginPermission>) {
        self.default_permissions = permissions;
    }

    /// Register a plugin
    pub fn register(&self, plugin: Box<dyn Plugin>) -> PluginResult<()> {
        let name = plugin.name().to_string();

        if self.plugins.contains_key(&name) {
            return Err(PluginError::AlreadyLoaded(name));
        }

        // Validate manifest
        let manifest = plugin.manifest();
        if let Err(errors) = manifest.validate() {
            return Err(PluginError::InvalidManifest(errors.join("; ")));
        }

        // Check dependencies
        for dep in &manifest.dependencies {
            if !dep.optional && !self.plugins.contains_key(&dep.name) {
                return Err(PluginError::MissingDependency {
                    plugin: name.clone(),
                    dependency: dep.name.clone(),
                });
            }
        }

        // Check permissions
        if self.require_permission_approval {
            for perm in &manifest.permissions {
                if perm.is_dangerous() && !self.default_permissions.contains(perm) {
                    return Err(PluginError::PermissionDenied {
                        plugin: name.clone(),
                        permission: format!("{:?}", perm),
                    });
                }
            }
        }

        let order = self
            .load_order
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let entry = PluginEntry::with_load_order(plugin, order);

        self.plugins.insert(name, entry);
        Ok(())
    }

    /// Unregister a plugin
    pub async fn unregister(&self, name: &str) -> PluginResult<()> {
        if let Some((_, entry)) = self.plugins.remove(name) {
            if entry.state().await.is_operational() {
                entry.shutdown().await?;
            }
            Ok(())
        } else {
            Err(PluginError::NotFound(name.to_string()))
        }
    }

    /// Get a reference to a plugin entry by name
    ///
    /// Returns a DashMap reference guard that provides read access to the plugin entry.
    /// The entry uses interior mutability, so all operations on it are async.
    pub fn get(&self, name: &str) -> Option<dashmap::mapref::one::Ref<'_, String, PluginEntry>> {
        self.plugins.get(name)
    }

    /// Check if a plugin is registered
    pub fn contains(&self, name: &str) -> bool {
        self.plugins.contains_key(name)
    }

    /// Get all plugin names
    pub fn plugin_names(&self) -> Vec<String> {
        self.plugins.iter().map(|r| r.key().clone()).collect()
    }

    /// Get all plugin infos
    pub async fn plugin_infos(&self) -> Vec<PluginInfo> {
        let mut infos = Vec::new();
        for entry_ref in self.plugins.iter() {
            infos.push(entry_ref.info().await);
        }
        infos
    }

    /// Initialize a specific plugin
    pub async fn initialize_plugin(
        &self,
        name: &str,
        permissions: Vec<PluginPermission>,
    ) -> PluginResult<()> {
        let entry = self
            .get(name)
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;

        // Build context with permissions
        let mut all_permissions = self.default_permissions.clone();
        all_permissions.extend(permissions);

        let plugin_name = entry.name().await;
        let plugin_version = entry.version().await;

        let ctx = PluginContext::new(plugin_name, plugin_version)
            .with_permission(PluginPermission::ConfigAccess);

        let ctx = all_permissions
            .into_iter()
            .fold(ctx, |ctx, perm| ctx.with_permission(perm));

        entry.set_context(ctx.clone()).await;
        entry.initialize(&ctx).await
    }

    /// Initialize all registered plugins
    pub async fn initialize_all(&self) -> Vec<PluginResult<()>> {
        let mut results = Vec::new();

        // Collect names with load order for sorting
        let mut entries_with_order: Vec<_> = self
            .plugins
            .iter()
            .map(|r| (r.key().clone(), r.load_order()))
            .collect();

        // Sort by load order (lower = earlier)
        entries_with_order.sort_by_key(|(_, order)| *order);

        for (name, _) in entries_with_order {
            let result = self.initialize_plugin(&name, vec![]).await;
            results.push(result);
        }

        results
    }

    /// Shutdown a specific plugin
    pub async fn shutdown_plugin(&self, name: &str) -> PluginResult<()> {
        let entry = self
            .get(name)
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;

        entry.shutdown().await
    }

    /// Shutdown all plugins
    pub async fn shutdown_all(&self) -> Vec<PluginResult<()>> {
        let mut results = Vec::new();

        // Collect names with load order for reverse shutdown order
        let mut entries_with_order: Vec<_> = self
            .plugins
            .iter()
            .map(|r| (r.key().clone(), r.load_order()))
            .collect();

        // Sort by load order in reverse (higher = earlier shutdown)
        entries_with_order.sort_by_key(|(_, order)| std::cmp::Reverse(*order));

        for (name, _) in entries_with_order {
            if let Some(entry) = self.get(&name) {
                if entry.state().await.can_shutdown() {
                    let result = entry.shutdown().await;
                    results.push(result);
                }
            }
        }

        results
    }

    /// Enable a plugin
    pub async fn enable_plugin(&self, name: &str) -> PluginResult<()> {
        let entry = self
            .get(name)
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;

        entry.set_enabled(true).await;

        // Resume if suspended
        if entry.state().await == PluginState::Suspended {
            entry.resume().await?;
        }

        Ok(())
    }

    /// Disable a plugin
    pub async fn disable_plugin(&self, name: &str) -> PluginResult<()> {
        let entry = self
            .get(name)
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;

        entry.set_enabled(false).await;

        // Suspend if active
        if entry.state().await == PluginState::Active {
            entry.suspend().await?;
        }

        Ok(())
    }

    /// Get plugins by capability
    pub async fn plugins_with_capability(&self, capability: PluginCapability) -> Vec<String> {
        let mut result = Vec::new();

        for entry_ref in self.plugins.iter() {
            let capabilities = entry_ref.capabilities().await;
            let enabled = entry_ref.is_enabled().await;
            let state = entry_ref.state().await;

            if capabilities.contains(&capability) && enabled && state.is_operational() {
                result.push(entry_ref.key().clone());
            }
        }

        result
    }

    /// Get all tools from plugins
    pub async fn collect_tools(&self) -> Vec<Arc<dyn crate::tools::base::Tool>> {
        let mut tools = Vec::new();

        for entry_ref in self.plugins.iter() {
            let enabled = entry_ref.is_enabled().await;
            let state = entry_ref.state().await;

            if enabled && state.is_operational() {
                tools.extend(entry_ref.get_tools().await);
            }
        }

        tools
    }

    /// Get plugin count
    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
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
        // Test PluginEntry directly to ensure lifecycle operations work correctly
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
}
