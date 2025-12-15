//! Plugin system for extending agent capabilities
//!
//! Provides:
//! - Plugin trait for custom extensions
//! - Plugin registry for managing plugins
//! - Plugin lifecycle management
//! - Security validation for plugins

mod lifecycle;
mod manifest;
mod registry;

pub use lifecycle::{PluginLifecycle, PluginState};
pub use manifest::{PluginManifest, PluginPermission, PluginDependency};
pub use registry::{PluginRegistry, PluginEntry};

use crate::error::SageError;
use crate::tools::base::Tool;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

/// Plugin result type
pub type PluginResult<T> = Result<T, PluginError>;

/// Plugin error types
#[derive(Debug, Clone, thiserror::Error)]
pub enum PluginError {
    /// Plugin not found
    #[error("Plugin not found: {0}")]
    NotFound(String),

    /// Plugin already loaded
    #[error("Plugin already loaded: {0}")]
    AlreadyLoaded(String),

    /// Plugin load failed
    #[error("Failed to load plugin '{name}': {reason}")]
    LoadFailed { name: String, reason: String },

    /// Plugin initialization failed
    #[error("Plugin '{name}' initialization failed: {reason}")]
    InitFailed { name: String, reason: String },

    /// Plugin execution failed
    #[error("Plugin '{name}' execution failed: {reason}")]
    ExecutionFailed { name: String, reason: String },

    /// Missing dependency
    #[error("Plugin '{plugin}' missing dependency: {dependency}")]
    MissingDependency { plugin: String, dependency: String },

    /// Version mismatch
    #[error("Plugin '{plugin}' version mismatch: required {required}, found {found}")]
    VersionMismatch {
        plugin: String,
        required: String,
        found: String,
    },

    /// Permission denied
    #[error("Plugin '{plugin}' permission denied: {permission}")]
    PermissionDenied { plugin: String, permission: String },

    /// Invalid manifest
    #[error("Invalid plugin manifest: {0}")]
    InvalidManifest(String),

    /// Plugin disabled
    #[error("Plugin is disabled: {0}")]
    Disabled(String),

    /// Internal error
    #[error("Plugin internal error: {0}")]
    Internal(String),
}

impl From<PluginError> for SageError {
    fn from(error: PluginError) -> Self {
        SageError::Agent(format!("Plugin error: {}", error))
    }
}

/// Plugin capability types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PluginCapability {
    /// Provides tools
    Tools,
    /// Provides hooks
    Hooks,
    /// Provides middleware
    Middleware,
    /// Provides LLM provider
    LlmProvider,
    /// Provides storage backend
    Storage,
    /// Provides custom commands
    Commands,
}

/// Plugin context passed to plugins during execution
#[derive(Clone)]
pub struct PluginContext {
    /// Plugin name
    pub plugin_name: String,

    /// Plugin version
    pub plugin_version: String,

    /// Working directory
    pub working_dir: Option<std::path::PathBuf>,

    /// Configuration values
    pub config: HashMap<String, serde_json::Value>,

    /// Granted permissions
    pub permissions: Vec<PluginPermission>,
}

impl PluginContext {
    /// Create new plugin context
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            plugin_name: name.into(),
            plugin_version: version.into(),
            working_dir: None,
            config: HashMap::new(),
            permissions: Vec::new(),
        }
    }

    /// Set working directory
    pub fn with_working_dir(mut self, dir: impl Into<std::path::PathBuf>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// Set configuration value
    pub fn with_config(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.config.insert(key.into(), value);
        self
    }

    /// Add permission
    pub fn with_permission(mut self, permission: PluginPermission) -> Self {
        self.permissions.push(permission);
        self
    }

    /// Check if permission is granted
    pub fn has_permission(&self, permission: &PluginPermission) -> bool {
        self.permissions.contains(permission)
    }

    /// Get configuration value
    pub fn get_config(&self, key: &str) -> Option<&serde_json::Value> {
        self.config.get(key)
    }
}

/// Main plugin trait
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Get plugin name
    fn name(&self) -> &str;

    /// Get plugin version
    fn version(&self) -> &str;

    /// Get plugin description
    fn description(&self) -> &str {
        ""
    }

    /// Get plugin author
    fn author(&self) -> Option<&str> {
        None
    }

    /// Get plugin manifest
    fn manifest(&self) -> PluginManifest {
        PluginManifest::new(self.name(), self.version())
    }

    /// Get plugin capabilities
    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![]
    }

    /// Initialize the plugin
    async fn initialize(&mut self, ctx: &PluginContext) -> PluginResult<()> {
        let _ = ctx;
        Ok(())
    }

    /// Shutdown the plugin
    async fn shutdown(&mut self) -> PluginResult<()> {
        Ok(())
    }

    /// Get tools provided by this plugin
    fn get_tools(&self) -> Vec<Arc<dyn Tool>> {
        vec![]
    }

    /// Get configuration schema
    fn config_schema(&self) -> Option<serde_json::Value> {
        None
    }

    /// Handle custom command
    async fn handle_command(
        &self,
        command: &str,
        args: serde_json::Value,
    ) -> PluginResult<serde_json::Value> {
        let _ = (command, args);
        Err(PluginError::ExecutionFailed {
            name: self.name().to_string(),
            reason: "Command not supported".to_string(),
        })
    }

    /// Cast to Any for downcasting
    fn as_any(&self) -> &dyn Any;

    /// Cast to mutable Any
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Plugin info for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<String>,
    pub capabilities: Vec<PluginCapability>,
    pub state: PluginState,
    pub enabled: bool,
}

impl PluginInfo {
    /// Create from plugin
    pub fn from_plugin(plugin: &dyn Plugin, state: PluginState, enabled: bool) -> Self {
        Self {
            name: plugin.name().to_string(),
            version: plugin.version().to_string(),
            description: plugin.description().to_string(),
            author: plugin.author().map(String::from),
            capabilities: plugin.capabilities(),
            state,
            enabled,
        }
    }
}

/// Plugin factory for creating plugin instances
pub trait PluginFactory: Send + Sync {
    /// Create a new plugin instance
    fn create(&self) -> Box<dyn Plugin>;

    /// Get plugin name
    fn plugin_name(&self) -> &str;

    /// Get plugin version
    fn plugin_version(&self) -> &str;
}

/// Simple plugin implementation for testing
#[cfg(test)]
pub(crate) struct TestPlugin {
    name: String,
    version: String,
}

#[cfg(test)]
impl TestPlugin {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
        }
    }
}

#[cfg(test)]
#[async_trait]
impl Plugin for TestPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn description(&self) -> &str {
        "Test plugin"
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Tools]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_context() {
        let ctx = PluginContext::new("test", "1.0.0")
            .with_working_dir("/tmp")
            .with_config("key", serde_json::json!("value"))
            .with_permission(PluginPermission::ReadFiles);

        assert_eq!(ctx.plugin_name, "test");
        assert!(ctx.has_permission(&PluginPermission::ReadFiles));
        assert!(!ctx.has_permission(&PluginPermission::WriteFiles));
        assert_eq!(
            ctx.get_config("key"),
            Some(&serde_json::json!("value"))
        );
    }

    #[test]
    fn test_plugin_error_display() {
        let err = PluginError::NotFound("test".to_string());
        assert!(err.to_string().contains("test"));

        let err = PluginError::MissingDependency {
            plugin: "a".to_string(),
            dependency: "b".to_string(),
        };
        assert!(err.to_string().contains("a"));
        assert!(err.to_string().contains("b"));
    }

    #[tokio::test]
    async fn test_plugin_trait() {
        let mut plugin = TestPlugin::new("test", "1.0.0");
        let ctx = PluginContext::new("test", "1.0.0");

        assert_eq!(plugin.name(), "test");
        assert_eq!(plugin.version(), "1.0.0");
        assert!(plugin.initialize(&ctx).await.is_ok());
        assert!(plugin.shutdown().await.is_ok());
    }
}
