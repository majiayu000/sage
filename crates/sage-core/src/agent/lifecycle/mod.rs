//! Agent lifecycle hooks
//!
//! Provides lifecycle management for agents with hooks at various points
//! in the agent execution flow.

mod context;
mod error;
pub mod hooks;
mod manager;
mod phase;

// Re-export main types for backward compatibility
pub use context::{HookResult, LifecycleContext};
pub use error::{LifecycleError, LifecycleResult};
pub use hooks::builtin::{LoggingHook, MetricsHook};
pub use hooks::{AgentLifecycle, LifecycleHook, LifecycleHookRegistry};
pub use manager::LifecycleManager;
pub use phase::LifecyclePhase;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{AgentState, AgentStep};
    use crate::types::TaskMetadata;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;
    use tokio::sync::RwLock;

    struct CountingHook {
        name: String,
        phases: Vec<LifecyclePhase>,
        count: Arc<AtomicU32>,
    }

    impl CountingHook {
        fn new(name: &str, phases: Vec<LifecyclePhase>) -> Self {
            Self {
                name: name.to_string(),
                phases,
                count: Arc::new(AtomicU32::new(0)),
            }
        }

        fn count(&self) -> u32 {
            self.count.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl LifecycleHook for CountingHook {
        fn name(&self) -> &str {
            &self.name
        }

        fn phases(&self) -> Vec<LifecyclePhase> {
            self.phases.clone()
        }

        async fn execute(&self, _context: &LifecycleContext) -> LifecycleResult<HookResult> {
            self.count.fetch_add(1, Ordering::SeqCst);
            Ok(HookResult::Continue)
        }
    }

    #[tokio::test]
    async fn test_hook_registry() {
        let registry = LifecycleHookRegistry::new();

        let hook = Arc::new(CountingHook::new(
            "test",
            vec![LifecyclePhase::Init, LifecyclePhase::TaskStart],
        ));
        registry.register(hook.clone()).await;

        assert_eq!(registry.count().await, 1);

        // Execute init hook
        let context = LifecycleContext::new(LifecyclePhase::Init, AgentState::Initializing);
        registry
            .execute_hooks(LifecyclePhase::Init, context)
            .await
            .unwrap();

        assert_eq!(hook.count(), 1);

        // Execute task start hook
        let context = LifecycleContext::new(LifecyclePhase::TaskStart, AgentState::Thinking);
        registry
            .execute_hooks(LifecyclePhase::TaskStart, context)
            .await
            .unwrap();

        assert_eq!(hook.count(), 2);

        // Execute hook for different phase (should not increment)
        let context = LifecycleContext::new(LifecyclePhase::Shutdown, AgentState::Completed);
        registry
            .execute_hooks(LifecyclePhase::Shutdown, context)
            .await
            .unwrap();

        assert_eq!(hook.count(), 2);
    }

    #[tokio::test]
    async fn test_hook_priority() {
        let registry = LifecycleHookRegistry::new();

        struct PriorityHook {
            name: String,
            priority: i32,
            order: Arc<RwLock<Vec<String>>>,
        }

        #[async_trait]
        impl LifecycleHook for PriorityHook {
            fn name(&self) -> &str {
                &self.name
            }

            fn phases(&self) -> Vec<LifecyclePhase> {
                vec![LifecyclePhase::Init]
            }

            fn priority(&self) -> i32 {
                self.priority
            }

            async fn execute(&self, _context: &LifecycleContext) -> LifecycleResult<HookResult> {
                self.order.write().await.push(self.name.clone());
                Ok(HookResult::Continue)
            }
        }

        let order = Arc::new(RwLock::new(Vec::new()));

        let hook1 = Arc::new(PriorityHook {
            name: "low".to_string(),
            priority: 0,
            order: order.clone(),
        });
        let hook2 = Arc::new(PriorityHook {
            name: "high".to_string(),
            priority: 100,
            order: order.clone(),
        });
        let hook3 = Arc::new(PriorityHook {
            name: "medium".to_string(),
            priority: 50,
            order: order.clone(),
        });

        registry.register(hook1).await;
        registry.register(hook2).await;
        registry.register(hook3).await;

        let context = LifecycleContext::new(LifecyclePhase::Init, AgentState::Initializing);
        registry
            .execute_hooks(LifecyclePhase::Init, context)
            .await
            .unwrap();

        let execution_order = order.read().await;
        assert_eq!(*execution_order, vec!["high", "medium", "low"]);
    }

    #[tokio::test]
    async fn test_hook_abort() {
        let registry = LifecycleHookRegistry::new();

        struct AbortHook;

        #[async_trait]
        impl LifecycleHook for AbortHook {
            fn name(&self) -> &str {
                "abort"
            }

            fn phases(&self) -> Vec<LifecyclePhase> {
                vec![LifecyclePhase::Init]
            }

            async fn execute(&self, _context: &LifecycleContext) -> LifecycleResult<HookResult> {
                Ok(HookResult::Abort("Test abort".to_string()))
            }
        }

        registry.register(Arc::new(AbortHook)).await;

        let context = LifecycleContext::new(LifecyclePhase::Init, AgentState::Initializing);
        let result = registry.execute_hooks(LifecyclePhase::Init, context).await;

        assert!(result.is_err());
        match result {
            Err(LifecycleError::Aborted { phase, reason }) => {
                assert_eq!(phase, LifecyclePhase::Init);
                assert_eq!(reason, "Test abort");
            }
            _ => panic!("Expected Aborted error"),
        }
    }

    #[tokio::test]
    async fn test_lifecycle_manager() {
        let manager = LifecycleManager::new();
        let agent_id = uuid::Uuid::new_v4();

        // Register a counting hook
        let hook = Arc::new(CountingHook::new(
            "test",
            vec![
                LifecyclePhase::Init,
                LifecyclePhase::TaskStart,
                LifecyclePhase::StepStart,
                LifecyclePhase::StepComplete,
                LifecyclePhase::TaskComplete,
            ],
        ));
        manager.registry().register(hook.clone()).await;

        // Initialize
        manager.initialize(agent_id).await.unwrap();
        assert!(manager.is_initialized().await);
        assert_eq!(hook.count(), 1);

        // Task start
        let task = TaskMetadata::new("Test task", "/tmp");
        manager.notify_task_start(agent_id, &task).await.unwrap();
        assert_eq!(manager.state().await, AgentState::Thinking);
        assert_eq!(hook.count(), 2);

        // Step start
        manager.notify_step_start(agent_id, 1).await.unwrap();
        assert_eq!(hook.count(), 3);

        // Step complete
        let step = AgentStep::new(1, AgentState::ToolExecution);
        manager.notify_step_complete(agent_id, &step).await.unwrap();
        assert_eq!(hook.count(), 4);

        // Shutdown
        manager.shutdown(agent_id).await.unwrap();
        assert!(!manager.is_initialized().await);
    }

    #[tokio::test]
    async fn test_lifecycle_context_builder() {
        let context = LifecycleContext::new(LifecyclePhase::TaskStart, AgentState::Thinking)
            .with_agent_id(uuid::Uuid::new_v4())
            .with_task(TaskMetadata::new("Test", "/tmp"))
            .with_step_number(1)
            .with_metadata("custom", serde_json::json!({"key": "value"}));

        assert_eq!(context.phase, LifecyclePhase::TaskStart);
        assert_eq!(context.state, AgentState::Thinking);
        assert!(context.agent_id.is_some());
        assert!(context.task.is_some());
        assert_eq!(context.step_number, Some(1));
        assert!(context.metadata.contains_key("custom"));
    }

    #[tokio::test]
    async fn test_logging_hook() {
        let hook = LoggingHook::all_phases();
        assert_eq!(hook.name(), "logging");
        assert_eq!(hook.phases().len(), 8);

        let context = LifecycleContext::new(LifecyclePhase::Init, AgentState::Initializing);
        let result = hook.execute(&context).await.unwrap();
        assert!(matches!(result, HookResult::Continue));
    }

    #[tokio::test]
    async fn test_metrics_hook() {
        let hook = MetricsHook::new();
        assert_eq!(hook.name(), "metrics");
        assert_eq!(hook.priority(), -100);

        let context = LifecycleContext::new(LifecyclePhase::TaskStart, AgentState::Thinking)
            .with_task(TaskMetadata::new("Test", "/tmp"));
        let result = hook.execute(&context).await.unwrap();
        assert!(matches!(result, HookResult::Continue));
    }

    #[tokio::test]
    async fn test_unregister_hook() {
        let registry = LifecycleHookRegistry::new();

        let hook1 = Arc::new(CountingHook::new("hook1", vec![LifecyclePhase::Init]));
        let hook2 = Arc::new(CountingHook::new("hook2", vec![LifecyclePhase::Init]));

        registry.register(hook1).await;
        registry.register(hook2.clone()).await;
        assert_eq!(registry.count().await, 2);

        registry.unregister("hook1").await;
        assert_eq!(registry.count().await, 1);

        let context = LifecycleContext::new(LifecyclePhase::Init, AgentState::Initializing);
        registry
            .execute_hooks(LifecyclePhase::Init, context)
            .await
            .unwrap();

        assert_eq!(hook2.count(), 1);
    }

    #[test]
    fn test_lifecycle_error_display() {
        let err = LifecycleError::InitFailed("test".to_string());
        assert_eq!(err.to_string(), "Initialization failed: test");

        let err = LifecycleError::HookFailed {
            hook: LifecyclePhase::TaskStart,
            message: "failed".to_string(),
        };
        assert!(err.to_string().contains("task_start"));

        let err = LifecycleError::InvalidTransition {
            from: AgentState::Completed,
            to: AgentState::Thinking,
        };
        assert!(err.to_string().contains("Invalid state transition"));
    }

    #[test]
    fn test_lifecycle_phase_display() {
        assert_eq!(format!("{}", LifecyclePhase::Init), "init");
        assert_eq!(format!("{}", LifecyclePhase::TaskStart), "task_start");
        assert_eq!(format!("{}", LifecyclePhase::StepStart), "step_start");
        assert_eq!(format!("{}", LifecyclePhase::StepComplete), "step_complete");
        assert_eq!(format!("{}", LifecyclePhase::TaskComplete), "task_complete");
        assert_eq!(format!("{}", LifecyclePhase::Shutdown), "shutdown");
        assert_eq!(
            format!("{}", LifecyclePhase::StateTransition),
            "state_transition"
        );
        assert_eq!(format!("{}", LifecyclePhase::Error), "error");
    }
}
