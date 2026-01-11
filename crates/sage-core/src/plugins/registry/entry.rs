//! Plugin entry for the registry

use crate::plugins::{
    Plugin, PluginCapability, PluginContext, PluginInfo, PluginLifecycle, PluginResult,
    PluginState,
};
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
