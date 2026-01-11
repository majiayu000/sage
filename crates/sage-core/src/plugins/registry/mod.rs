//! Plugin registry for managing plugins

mod entry;

pub use entry::PluginEntry;

use crate::plugins::{
    Plugin, PluginCapability, PluginContext, PluginError, PluginInfo, PluginPermission,
    PluginResult, PluginState,
};
use dashmap::DashMap;
use std::sync::Arc;

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

        let mut entries_with_order: Vec<_> = self
            .plugins
            .iter()
            .map(|r| (r.key().clone(), r.load_order()))
            .collect();

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

        let mut entries_with_order: Vec<_> = self
            .plugins
            .iter()
            .map(|r| (r.key().clone(), r.load_order()))
            .collect();

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
mod tests;
