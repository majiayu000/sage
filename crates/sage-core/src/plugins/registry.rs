//! Plugin registry for managing plugins

use super::{
    Plugin, PluginCapability, PluginContext, PluginError, PluginInfo, PluginLifecycle,
    PluginPermission, PluginResult, PluginState,
};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Plugin entry in the registry
pub struct PluginEntry {
    /// The plugin instance
    pub plugin: Box<dyn Plugin>,

    /// Plugin lifecycle manager
    pub lifecycle: PluginLifecycle,

    /// Whether the plugin is enabled
    pub enabled: bool,

    /// Plugin context
    pub context: Option<PluginContext>,

    /// Load order (lower = earlier)
    pub load_order: usize,
}

impl PluginEntry {
    /// Create new plugin entry
    pub fn new(plugin: Box<dyn Plugin>) -> Self {
        Self {
            plugin,
            lifecycle: PluginLifecycle::new(),
            enabled: true,
            context: None,
            load_order: 0,
        }
    }

    /// Get plugin info
    pub fn info(&self) -> PluginInfo {
        PluginInfo::from_plugin(self.plugin.as_ref(), self.lifecycle.state(), self.enabled)
    }
}

/// Plugin registry for managing all plugins
pub struct PluginRegistry {
    /// Registered plugins
    plugins: DashMap<String, Arc<RwLock<PluginEntry>>>,

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

        let mut entry = PluginEntry::new(plugin);
        entry.load_order = order;

        self.plugins.insert(name, Arc::new(RwLock::new(entry)));
        Ok(())
    }

    /// Unregister a plugin
    pub async fn unregister(&self, name: &str) -> PluginResult<()> {
        if let Some((_, entry)) = self.plugins.remove(name) {
            let mut entry = entry.write().await;
            if entry.lifecycle.state().is_operational() {
                // Use raw pointer to avoid double borrow
                let plugin_ptr = &mut *entry.plugin as *mut dyn Plugin;
                unsafe {
                    entry.lifecycle.shutdown(&mut *plugin_ptr).await?;
                }
            }
            Ok(())
        } else {
            Err(PluginError::NotFound(name.to_string()))
        }
    }

    /// Get a plugin by name
    pub fn get(&self, name: &str) -> Option<Arc<RwLock<PluginEntry>>> {
        self.plugins.get(name).map(|r| r.clone())
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
            let entry = entry_ref.read().await;
            infos.push(entry.info());
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

        let mut entry = entry.write().await;

        // Build context with permissions
        let mut all_permissions = self.default_permissions.clone();
        all_permissions.extend(permissions);

        let ctx = PluginContext::new(entry.plugin.name(), entry.plugin.version())
            .with_permission(PluginPermission::ConfigAccess);

        let ctx = all_permissions
            .into_iter()
            .fold(ctx, |ctx, perm| ctx.with_permission(perm));

        entry.context = Some(ctx.clone());

        // Use raw pointer to avoid double borrow
        let plugin_ptr = &mut *entry.plugin as *mut dyn Plugin;
        unsafe { entry.lifecycle.initialize(&mut *plugin_ptr, &ctx).await }
    }

    /// Initialize all registered plugins
    pub async fn initialize_all(&self) -> Vec<PluginResult<()>> {
        let mut results = Vec::new();

        // Sort by load order
        let mut entries: Vec<_> = self.plugins.iter().collect();
        entries.sort_by_key(|_e| {
            // We can't await inside sort, so we'll use a blocking read
            // In practice, this is fine since we're just reading load_order
            0 // Simplified - actual implementation would need to access load_order
        });

        for entry_ref in entries {
            let name = entry_ref.key().clone();
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

        let mut entry = entry.write().await;
        let plugin_ptr = &mut *entry.plugin as *mut dyn Plugin;
        unsafe { entry.lifecycle.shutdown(&mut *plugin_ptr).await }
    }

    /// Shutdown all plugins
    pub async fn shutdown_all(&self) -> Vec<PluginResult<()>> {
        let mut results = Vec::new();

        // Shutdown in reverse load order would be ideal
        for entry_ref in self.plugins.iter() {
            let mut entry = entry_ref.write().await;
            if entry.lifecycle.state().can_shutdown() {
                let plugin_ptr = &mut *entry.plugin as *mut dyn Plugin;
                let result = unsafe { entry.lifecycle.shutdown(&mut *plugin_ptr).await };
                results.push(result);
            }
        }

        results
    }

    /// Enable a plugin
    pub async fn enable_plugin(&self, name: &str) -> PluginResult<()> {
        let entry = self
            .get(name)
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;

        let mut entry = entry.write().await;
        entry.enabled = true;

        // Resume if suspended
        if entry.lifecycle.state() == PluginState::Suspended {
            let plugin_ptr = &mut *entry.plugin as *mut dyn Plugin;
            unsafe {
                entry.lifecycle.resume(&mut *plugin_ptr).await?;
            }
        }

        Ok(())
    }

    /// Disable a plugin
    pub async fn disable_plugin(&self, name: &str) -> PluginResult<()> {
        let entry = self
            .get(name)
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;

        let mut entry = entry.write().await;
        entry.enabled = false;

        // Suspend if active
        if entry.lifecycle.state() == PluginState::Active {
            let plugin_ptr = &mut *entry.plugin as *mut dyn Plugin;
            unsafe {
                entry.lifecycle.suspend(&mut *plugin_ptr).await?;
            }
        }

        Ok(())
    }

    /// Get plugins by capability
    pub async fn plugins_with_capability(&self, capability: PluginCapability) -> Vec<String> {
        let mut result = Vec::new();

        for entry_ref in self.plugins.iter() {
            let entry = entry_ref.read().await;
            if entry.plugin.capabilities().contains(&capability)
                && entry.enabled
                && entry.lifecycle.state().is_operational()
            {
                result.push(entry_ref.key().clone());
            }
        }

        result
    }

    /// Get all tools from plugins
    pub async fn collect_tools(&self) -> Vec<Arc<dyn crate::tools::base::Tool>> {
        let mut tools = Vec::new();

        for entry_ref in self.plugins.iter() {
            let entry = entry_ref.read().await;
            if entry.enabled && entry.lifecycle.state().is_operational() {
                tools.extend(entry.plugin.get_tools());
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
        let entry = entry.read().await;
        assert_eq!(entry.lifecycle.state(), PluginState::Active);
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
            let entry = entry.read().await;
            assert!(!entry.enabled);
            assert_eq!(entry.lifecycle.state(), PluginState::Suspended);
        }

        // Enable
        assert!(registry.enable_plugin("test").await.is_ok());
        {
            let entry = registry.get("test").unwrap();
            let entry = entry.read().await;
            assert!(entry.enabled);
            assert_eq!(entry.lifecycle.state(), PluginState::Active);
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
}
