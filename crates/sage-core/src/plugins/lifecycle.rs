//! Plugin lifecycle management

use super::{Plugin, PluginContext, PluginError, PluginResult};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Plugin lifecycle state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginState {
    /// Plugin is created but not initialized
    Created,

    /// Plugin is initializing
    Initializing,

    /// Plugin is ready and active
    Active,

    /// Plugin is being suspended
    Suspending,

    /// Plugin is suspended (inactive but loaded)
    Suspended,

    /// Plugin is being shutdown
    ShuttingDown,

    /// Plugin is stopped
    Stopped,

    /// Plugin failed to initialize or crashed
    Failed,
}

impl PluginState {
    /// Check if plugin can be initialized
    pub fn can_initialize(&self) -> bool {
        matches!(self, PluginState::Created | PluginState::Stopped)
    }

    /// Check if plugin can be suspended
    pub fn can_suspend(&self) -> bool {
        matches!(self, PluginState::Active)
    }

    /// Check if plugin can be resumed
    pub fn can_resume(&self) -> bool {
        matches!(self, PluginState::Suspended)
    }

    /// Check if plugin can be shutdown
    pub fn can_shutdown(&self) -> bool {
        matches!(
            self,
            PluginState::Active | PluginState::Suspended | PluginState::Failed
        )
    }

    /// Check if plugin is operational
    pub fn is_operational(&self) -> bool {
        matches!(self, PluginState::Active)
    }
}

/// Plugin lifecycle manager
pub struct PluginLifecycle {
    /// Current state
    state: PluginState,

    /// State change history
    history: Vec<StateChange>,

    /// Maximum history entries
    max_history: usize,
}

/// State change record
#[derive(Debug, Clone)]
pub struct StateChange {
    /// Previous state
    pub from: PluginState,

    /// New state
    pub to: PluginState,

    /// Timestamp
    pub timestamp: std::time::SystemTime,

    /// Reason for change
    pub reason: Option<String>,
}

impl PluginLifecycle {
    /// Create new lifecycle manager
    pub fn new() -> Self {
        Self {
            state: PluginState::Created,
            history: Vec::new(),
            max_history: 100,
        }
    }

    /// Get current state
    pub fn state(&self) -> PluginState {
        self.state
    }

    /// Get state history
    pub fn history(&self) -> &[StateChange] {
        &self.history
    }

    /// Transition to new state
    pub fn transition(&mut self, new_state: PluginState, reason: Option<String>) -> PluginResult<()> {
        let valid = match (self.state, new_state) {
            // From Created
            (PluginState::Created, PluginState::Initializing) => true,
            (PluginState::Created, PluginState::Failed) => true,

            // From Initializing
            (PluginState::Initializing, PluginState::Active) => true,
            (PluginState::Initializing, PluginState::Failed) => true,

            // From Active
            (PluginState::Active, PluginState::Suspending) => true,
            (PluginState::Active, PluginState::ShuttingDown) => true,
            (PluginState::Active, PluginState::Failed) => true,

            // From Suspending
            (PluginState::Suspending, PluginState::Suspended) => true,
            (PluginState::Suspending, PluginState::Failed) => true,

            // From Suspended
            (PluginState::Suspended, PluginState::Active) => true,
            (PluginState::Suspended, PluginState::ShuttingDown) => true,

            // From ShuttingDown
            (PluginState::ShuttingDown, PluginState::Stopped) => true,
            (PluginState::ShuttingDown, PluginState::Failed) => true,

            // From Stopped (can restart)
            (PluginState::Stopped, PluginState::Initializing) => true,

            // From Failed (can retry)
            (PluginState::Failed, PluginState::Initializing) => true,
            (PluginState::Failed, PluginState::Stopped) => true,

            _ => false,
        };

        if !valid {
            return Err(PluginError::Internal(format!(
                "Invalid state transition: {:?} -> {:?}",
                self.state, new_state
            )));
        }

        let change = StateChange {
            from: self.state,
            to: new_state,
            timestamp: std::time::SystemTime::now(),
            reason,
        };

        self.history.push(change);
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }

        self.state = new_state;
        Ok(())
    }

    /// Initialize plugin
    pub async fn initialize(
        &mut self,
        plugin: &mut dyn Plugin,
        ctx: &PluginContext,
    ) -> PluginResult<()> {
        if !self.state.can_initialize() {
            return Err(PluginError::Internal(format!(
                "Cannot initialize plugin in state {:?}",
                self.state
            )));
        }

        self.transition(PluginState::Initializing, Some("Starting initialization".into()))?;

        match plugin.initialize(ctx).await {
            Ok(()) => {
                self.transition(PluginState::Active, Some("Initialization successful".into()))?;
                Ok(())
            }
            Err(e) => {
                self.transition(
                    PluginState::Failed,
                    Some(format!("Initialization failed: {}", e)),
                )?;
                Err(e)
            }
        }
    }

    /// Suspend plugin
    pub async fn suspend(&mut self, _plugin: &mut dyn Plugin) -> PluginResult<()> {
        if !self.state.can_suspend() {
            return Err(PluginError::Internal(format!(
                "Cannot suspend plugin in state {:?}",
                self.state
            )));
        }

        self.transition(PluginState::Suspending, Some("Starting suspension".into()))?;
        self.transition(PluginState::Suspended, Some("Suspended successfully".into()))?;

        Ok(())
    }

    /// Resume plugin
    pub async fn resume(&mut self, _plugin: &mut dyn Plugin) -> PluginResult<()> {
        if !self.state.can_resume() {
            return Err(PluginError::Internal(format!(
                "Cannot resume plugin in state {:?}",
                self.state
            )));
        }

        self.transition(PluginState::Active, Some("Resumed successfully".into()))?;
        Ok(())
    }

    /// Shutdown plugin
    pub async fn shutdown(&mut self, plugin: &mut dyn Plugin) -> PluginResult<()> {
        if !self.state.can_shutdown() {
            return Err(PluginError::Internal(format!(
                "Cannot shutdown plugin in state {:?}",
                self.state
            )));
        }

        self.transition(PluginState::ShuttingDown, Some("Starting shutdown".into()))?;

        match plugin.shutdown().await {
            Ok(()) => {
                self.transition(PluginState::Stopped, Some("Shutdown successful".into()))?;
                Ok(())
            }
            Err(e) => {
                self.transition(
                    PluginState::Failed,
                    Some(format!("Shutdown failed: {}", e)),
                )?;
                Err(e)
            }
        }
    }

    /// Mark as failed
    pub fn fail(&mut self, reason: impl Into<String>) {
        let _ = self.transition(PluginState::Failed, Some(reason.into()));
    }
}

impl Default for PluginLifecycle {
    fn default() -> Self {
        Self::new()
    }
}

/// Health check result
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Plugin name
    pub plugin_name: String,

    /// Whether the plugin is healthy
    pub healthy: bool,

    /// Status message
    pub message: String,

    /// Response time in milliseconds
    pub response_time_ms: u64,

    /// Last check timestamp
    pub timestamp: std::time::SystemTime,
}

impl HealthCheck {
    /// Create successful health check
    pub fn ok(name: impl Into<String>, response_time_ms: u64) -> Self {
        Self {
            plugin_name: name.into(),
            healthy: true,
            message: "OK".to_string(),
            response_time_ms,
            timestamp: std::time::SystemTime::now(),
        }
    }

    /// Create failed health check
    pub fn failed(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            plugin_name: name.into(),
            healthy: false,
            message: message.into(),
            response_time_ms: 0,
            timestamp: std::time::SystemTime::now(),
        }
    }
}

/// Perform health check on a plugin
#[allow(dead_code)]
pub async fn check_plugin_health(
    plugin: &dyn Plugin,
    lifecycle: &PluginLifecycle,
) -> HealthCheck {
    let start = Instant::now();

    if !lifecycle.state().is_operational() {
        return HealthCheck::failed(
            plugin.name(),
            format!("Plugin not operational: {:?}", lifecycle.state()),
        );
    }

    let response_time_ms = start.elapsed().as_millis() as u64;
    HealthCheck::ok(plugin.name(), response_time_ms)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::TestPlugin;

    #[test]
    fn test_plugin_state_transitions() {
        assert!(PluginState::Created.can_initialize());
        assert!(PluginState::Active.can_suspend());
        assert!(PluginState::Suspended.can_resume());
        assert!(PluginState::Active.can_shutdown());

        assert!(!PluginState::Active.can_initialize());
        assert!(!PluginState::Suspended.can_suspend());
    }

    #[test]
    fn test_lifecycle_valid_transitions() {
        let mut lifecycle = PluginLifecycle::new();

        assert!(lifecycle
            .transition(PluginState::Initializing, None)
            .is_ok());
        assert!(lifecycle.transition(PluginState::Active, None).is_ok());
        assert!(lifecycle.transition(PluginState::Suspending, None).is_ok());
        assert!(lifecycle.transition(PluginState::Suspended, None).is_ok());
        assert!(lifecycle.transition(PluginState::Active, None).is_ok());
        assert!(lifecycle
            .transition(PluginState::ShuttingDown, None)
            .is_ok());
        assert!(lifecycle.transition(PluginState::Stopped, None).is_ok());
    }

    #[test]
    fn test_lifecycle_invalid_transitions() {
        let mut lifecycle = PluginLifecycle::new();

        // Cannot go directly from Created to Active
        assert!(lifecycle.transition(PluginState::Active, None).is_err());

        // Cannot suspend from Created
        assert!(lifecycle.transition(PluginState::Suspending, None).is_err());
    }

    #[test]
    fn test_lifecycle_history() {
        let mut lifecycle = PluginLifecycle::new();

        lifecycle
            .transition(PluginState::Initializing, Some("test".into()))
            .unwrap();
        lifecycle.transition(PluginState::Active, None).unwrap();

        assert_eq!(lifecycle.history().len(), 2);
        assert_eq!(lifecycle.history()[0].from, PluginState::Created);
        assert_eq!(lifecycle.history()[0].to, PluginState::Initializing);
    }

    #[tokio::test]
    async fn test_lifecycle_initialize() {
        let mut lifecycle = PluginLifecycle::new();
        let mut plugin = TestPlugin::new("test", "1.0.0");
        let ctx = PluginContext::new("test", "1.0.0");

        assert!(lifecycle.initialize(&mut plugin, &ctx).await.is_ok());
        assert_eq!(lifecycle.state(), PluginState::Active);
    }

    #[tokio::test]
    async fn test_lifecycle_suspend_resume() {
        let mut lifecycle = PluginLifecycle::new();
        let mut plugin = TestPlugin::new("test", "1.0.0");
        let ctx = PluginContext::new("test", "1.0.0");

        lifecycle.initialize(&mut plugin, &ctx).await.unwrap();

        assert!(lifecycle.suspend(&mut plugin).await.is_ok());
        assert_eq!(lifecycle.state(), PluginState::Suspended);

        assert!(lifecycle.resume(&mut plugin).await.is_ok());
        assert_eq!(lifecycle.state(), PluginState::Active);
    }

    #[tokio::test]
    async fn test_lifecycle_shutdown() {
        let mut lifecycle = PluginLifecycle::new();
        let mut plugin = TestPlugin::new("test", "1.0.0");
        let ctx = PluginContext::new("test", "1.0.0");

        lifecycle.initialize(&mut plugin, &ctx).await.unwrap();

        assert!(lifecycle.shutdown(&mut plugin).await.is_ok());
        assert_eq!(lifecycle.state(), PluginState::Stopped);
    }

    #[tokio::test]
    async fn test_health_check() {
        let mut lifecycle = PluginLifecycle::new();
        let mut plugin = TestPlugin::new("test", "1.0.0");
        let ctx = PluginContext::new("test", "1.0.0");

        // Not initialized - unhealthy
        let health = check_plugin_health(&plugin, &lifecycle).await;
        assert!(!health.healthy);

        // After init - healthy
        lifecycle.initialize(&mut plugin, &ctx).await.unwrap();
        let health = check_plugin_health(&plugin, &lifecycle).await;
        assert!(health.healthy);
    }
}
