//! Tests for task supervision

#[cfg(test)]
mod tests {
    use super::super::multi_supervisor::Supervisor;
    use super::super::recovery::catch_panic;
    use super::super::task_supervisor::TaskSupervisor;
    use super::super::types::{SupervisionEvent, SupervisionPolicy, SupervisionResult};
    use crate::error::SageError;
    use crate::recovery::RecoveryError;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::Duration;

    #[tokio::test]
    async fn test_task_supervisor_success() {
        let mut supervisor = TaskSupervisor::new("test");

        let result = supervisor
            .supervise(|| async { Ok::<_, SageError>(42) })
            .await;

        assert!(matches!(result, SupervisionResult::Completed));
    }

    #[tokio::test]
    async fn test_task_supervisor_transient_failure() {
        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_clone = attempts.clone();

        let mut supervisor = TaskSupervisor::new("test").with_policy(SupervisionPolicy::Restart {
            max_restarts: 3,
            window: Duration::from_secs(60),
        });

        let result = supervisor
            .supervise(|| {
                let attempts = attempts_clone.clone();
                async move {
                    let count = attempts.fetch_add(1, Ordering::SeqCst);
                    if count < 2 {
                        Err(SageError::http("timeout"))
                    } else {
                        Ok(())
                    }
                }
            })
            .await;

        // First call should result in restart
        assert!(
            matches!(result, SupervisionResult::Restarted { .. })
                || matches!(result, SupervisionResult::Completed)
        );
    }

    #[tokio::test]
    async fn test_task_supervisor_permanent_failure() {
        let mut supervisor = TaskSupervisor::new("test");

        let result = supervisor
            .supervise(|| async { Err::<(), _>(SageError::config("invalid")) })
            .await;

        assert!(matches!(result, SupervisionResult::Stopped { .. }));
    }

    #[tokio::test]
    async fn test_supervisor_spawn_once() {
        let supervisor = Supervisor::new("test_supervisor");
        let mut events = supervisor.subscribe();

        let handle = supervisor.spawn_once("test_task", || async { Ok(()) });

        // Wait for completion
        let result = handle.await.unwrap();
        assert!(matches!(result, SupervisionResult::Completed));

        // Should receive start and complete events
        let event = events.recv().await.unwrap();
        assert!(matches!(event, SupervisionEvent::TaskStarted { .. }));
    }

    #[tokio::test]
    async fn test_supervisor_spawn_restartable() {
        let supervisor = Supervisor::new("test_supervisor");
        let mut events = supervisor.subscribe();

        let handle = supervisor.spawn("test_task", || async { Ok(()) });

        // Wait for completion
        let result = handle.await.unwrap();
        assert!(matches!(result, SupervisionResult::Completed));

        // Should receive start and complete events
        let event = events.recv().await.unwrap();
        assert!(matches!(event, SupervisionEvent::TaskStarted { .. }));
    }

    #[tokio::test]
    async fn test_supervisor_shutdown() {
        let supervisor = Supervisor::new("test_supervisor");
        let mut events = supervisor.subscribe();

        supervisor.shutdown();

        // Should receive shutdown event
        let event = events.recv().await.unwrap();
        assert!(matches!(event, SupervisionEvent::ShuttingDown));
    }

    #[tokio::test]
    async fn test_catch_panic() {
        let result: Result<i32, RecoveryError> = catch_panic(|| async { 42 }).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }
}
