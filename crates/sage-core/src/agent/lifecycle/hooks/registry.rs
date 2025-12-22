//! Lifecycle hook registry

use std::sync::Arc;
use tokio::sync::RwLock;

use super::super::context::{HookResult, LifecycleContext};
use super::super::error::{LifecycleError, LifecycleResult};
use super::super::phase::LifecyclePhase;
use super::traits::LifecycleHook;

/// Registry for managing lifecycle hooks
pub struct LifecycleHookRegistry {
    hooks: RwLock<Vec<Arc<dyn LifecycleHook>>>,
}

impl LifecycleHookRegistry {
    /// Create a new hook registry
    pub fn new() -> Self {
        Self {
            hooks: RwLock::new(Vec::new()),
        }
    }

    /// Register a hook
    pub async fn register(&self, hook: Arc<dyn LifecycleHook>) {
        let mut hooks = self.hooks.write().await;
        hooks.push(hook);
        // Sort by priority (higher first)
        hooks.sort_by(|a, b| b.priority().cmp(&a.priority()));
    }

    /// Unregister a hook by name
    pub async fn unregister(&self, name: &str) {
        let mut hooks = self.hooks.write().await;
        hooks.retain(|h| h.name() != name);
    }

    /// Execute all hooks for a phase
    pub async fn execute_hooks(
        &self,
        phase: LifecyclePhase,
        mut context: LifecycleContext,
    ) -> LifecycleResult<LifecycleContext> {
        let hooks = self.hooks.read().await;

        for hook in hooks.iter() {
            if !hook.phases().contains(&phase) {
                continue;
            }

            match hook.execute(&context).await? {
                HookResult::Continue => continue,
                HookResult::Skip => break,
                HookResult::Abort(reason) => {
                    return Err(LifecycleError::aborted(phase, reason));
                }
                HookResult::ModifyContext(new_context) => {
                    context = *new_context;
                }
            }
        }

        Ok(context)
    }

    /// Get all registered hooks
    pub async fn hooks(&self) -> Vec<Arc<dyn LifecycleHook>> {
        self.hooks.read().await.clone()
    }

    /// Get hooks count
    pub async fn count(&self) -> usize {
        self.hooks.read().await.len()
    }
}

impl Default for LifecycleHookRegistry {
    fn default() -> Self {
        Self::new()
    }
}
