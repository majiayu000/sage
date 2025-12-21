//! Background task registry for managing all running background tasks
//!
//! This module provides a global registry for tracking and managing background shell tasks.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use dashmap::DashMap;
use tracing::{debug, info, warn};

use super::background_task::{BackgroundShellTask, BackgroundTaskStatus};

/// Summary information about a background task
#[derive(Debug, Clone)]
pub struct BackgroundTaskSummary {
    /// Shell ID
    pub shell_id: String,
    /// Process ID
    pub pid: Option<u32>,
    /// Command being executed
    pub command: String,
    /// Working directory
    pub working_dir: String,
    /// Current status
    pub status: BackgroundTaskStatus,
    /// Uptime in seconds
    pub uptime_secs: f64,
}

/// Global registry for background tasks
pub struct BackgroundTaskRegistry {
    /// Map of shell_id -> task
    tasks: DashMap<String, Arc<BackgroundShellTask>>,
    /// Counter for generating unique shell IDs
    next_id: AtomicUsize,
}

impl BackgroundTaskRegistry {
    /// Create a new background task registry
    pub fn new() -> Self {
        Self {
            tasks: DashMap::new(),
            next_id: AtomicUsize::new(1),
        }
    }

    /// Generate a unique shell ID
    pub fn generate_shell_id(&self) -> String {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        format!("shell_{}", id)
    }

    /// Register a background task
    pub fn register(&self, task: Arc<BackgroundShellTask>) {
        let shell_id = task.shell_id.clone();
        self.tasks.insert(shell_id.clone(), task);
        debug!("Registered background task: {}", shell_id);
    }

    /// Get a task by shell ID
    pub fn get(&self, shell_id: &str) -> Option<Arc<BackgroundShellTask>> {
        self.tasks.get(shell_id).map(|entry| entry.clone())
    }

    /// Remove a task from the registry
    pub fn remove(&self, shell_id: &str) -> Option<Arc<BackgroundShellTask>> {
        self.tasks.remove(shell_id).map(|(_, task)| task)
    }

    /// Check if a task exists
    pub fn exists(&self, shell_id: &str) -> bool {
        self.tasks.contains_key(shell_id)
    }

    /// Get list of all active shell IDs
    pub fn list_active(&self) -> Vec<String> {
        self.tasks.iter().map(|entry| entry.key().clone()).collect()
    }

    /// Get count of active tasks
    pub fn count(&self) -> usize {
        self.tasks.len()
    }

    /// Get summary of all tasks
    pub async fn list_summaries(&self) -> Vec<BackgroundTaskSummary> {
        let mut summaries = Vec::new();

        for entry in self.tasks.iter() {
            let task = entry.value();
            let status = task.status().await;
            summaries.push(BackgroundTaskSummary {
                shell_id: task.shell_id.clone(),
                pid: task.pid,
                command: task.command.clone(),
                working_dir: task.working_dir.clone(),
                status,
                uptime_secs: task.uptime_secs(),
            });
        }

        // Sort by shell_id for consistent ordering
        summaries.sort_by(|a, b| a.shell_id.cmp(&b.shell_id));
        summaries
    }

    /// Clean up completed tasks older than max_age
    pub async fn cleanup_old_tasks(&self, max_age: Duration) {
        let now = Instant::now();
        let mut to_remove = Vec::new();

        for entry in self.tasks.iter() {
            let task = entry.value();
            let status = task.status().await;

            // Only clean up non-running tasks
            if !matches!(status, BackgroundTaskStatus::Running) {
                if now.duration_since(task.started_at) > max_age {
                    to_remove.push(entry.key().clone());
                }
            }
        }

        for shell_id in to_remove {
            if self.remove(&shell_id).is_some() {
                info!("Cleaned up old background task: {}", shell_id);
            }
        }
    }

    /// Kill all running tasks
    pub async fn kill_all(&self) {
        for entry in self.tasks.iter() {
            let task = entry.value();
            if task.is_running().await {
                if let Err(e) = task.kill().await {
                    warn!("Failed to kill task '{}': {}", task.shell_id, e);
                }
            }
        }
    }

    /// Kill a specific task
    pub async fn kill(&self, shell_id: &str) -> Option<()> {
        if let Some(task) = self.get(shell_id) {
            task.kill().await.ok();
            Some(())
        } else {
            None
        }
    }

    /// Get output from a task
    pub async fn get_output(&self, shell_id: &str) -> Option<(String, String)> {
        if let Some(task) = self.get(shell_id) {
            Some(task.get_output().await)
        } else {
            None
        }
    }

    /// Get incremental output from a task
    pub async fn get_incremental_output(&self, shell_id: &str) -> Option<(String, String)> {
        if let Some(task) = self.get(shell_id) {
            Some(task.get_incremental_output().await)
        } else {
            None
        }
    }

    /// Get status of a task
    pub async fn get_status(&self, shell_id: &str) -> Option<BackgroundTaskStatus> {
        if let Some(task) = self.get(shell_id) {
            Some(task.status().await)
        } else {
            None
        }
    }
}

impl Default for BackgroundTaskRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Global singleton registry
lazy_static::lazy_static! {
    /// Global background task registry
    pub static ref BACKGROUND_REGISTRY: Arc<BackgroundTaskRegistry> =
        Arc::new(BackgroundTaskRegistry::new());
}

/// Get the global background task registry
pub fn global_registry() -> &'static Arc<BackgroundTaskRegistry> {
    &BACKGROUND_REGISTRY
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tokio_util::sync::CancellationToken;

    #[tokio::test]
    async fn test_registry_generate_shell_id() {
        let registry = BackgroundTaskRegistry::new();

        let id1 = registry.generate_shell_id();
        let id2 = registry.generate_shell_id();
        let id3 = registry.generate_shell_id();

        assert_eq!(id1, "shell_1");
        assert_eq!(id2, "shell_2");
        assert_eq!(id3, "shell_3");
    }

    #[tokio::test]
    async fn test_registry_register_and_get() {
        let registry = BackgroundTaskRegistry::new();
        let cancel_token = CancellationToken::new();

        let task = BackgroundShellTask::spawn(
            "test_reg_1".to_string(),
            "echo 'test'",
            &PathBuf::from("/tmp"),
            cancel_token,
        )
        .await
        .unwrap();

        registry.register(Arc::new(task));

        assert!(registry.exists("test_reg_1"));
        assert!(!registry.exists("nonexistent"));

        let retrieved = registry.get("test_reg_1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().shell_id, "test_reg_1");
    }

    #[tokio::test]
    async fn test_registry_remove() {
        let registry = BackgroundTaskRegistry::new();
        let cancel_token = CancellationToken::new();

        let task = BackgroundShellTask::spawn(
            "test_reg_2".to_string(),
            "echo 'test'",
            &PathBuf::from("/tmp"),
            cancel_token,
        )
        .await
        .unwrap();

        registry.register(Arc::new(task));
        assert!(registry.exists("test_reg_2"));

        let removed = registry.remove("test_reg_2");
        assert!(removed.is_some());
        assert!(!registry.exists("test_reg_2"));
    }

    #[tokio::test]
    async fn test_registry_list_active() {
        let registry = BackgroundTaskRegistry::new();

        let cancel1 = CancellationToken::new();
        let cancel2 = CancellationToken::new();

        let task1 = BackgroundShellTask::spawn(
            "test_list_1".to_string(),
            "sleep 1",
            &PathBuf::from("/tmp"),
            cancel1,
        )
        .await
        .unwrap();

        let task2 = BackgroundShellTask::spawn(
            "test_list_2".to_string(),
            "sleep 1",
            &PathBuf::from("/tmp"),
            cancel2,
        )
        .await
        .unwrap();

        registry.register(Arc::new(task1));
        registry.register(Arc::new(task2));

        let active = registry.list_active();
        assert_eq!(active.len(), 2);
        assert!(active.contains(&"test_list_1".to_string()));
        assert!(active.contains(&"test_list_2".to_string()));
    }

    #[tokio::test]
    async fn test_registry_count() {
        let registry = BackgroundTaskRegistry::new();
        assert_eq!(registry.count(), 0);

        let cancel = CancellationToken::new();
        let task = BackgroundShellTask::spawn(
            "test_count_1".to_string(),
            "echo 'test'",
            &PathBuf::from("/tmp"),
            cancel,
        )
        .await
        .unwrap();

        registry.register(Arc::new(task));
        assert_eq!(registry.count(), 1);
    }

    #[tokio::test]
    async fn test_registry_list_summaries() {
        let registry = BackgroundTaskRegistry::new();
        let cancel = CancellationToken::new();

        let task = BackgroundShellTask::spawn(
            "test_summary_1".to_string(),
            "echo 'test'",
            &PathBuf::from("/tmp"),
            cancel,
        )
        .await
        .unwrap();

        registry.register(Arc::new(task));

        // Wait for completion
        tokio::time::sleep(Duration::from_millis(100)).await;

        let summaries = registry.list_summaries().await;
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].shell_id, "test_summary_1");
        assert_eq!(summaries[0].command, "echo 'test'");
    }

    #[tokio::test]
    async fn test_registry_cleanup() {
        let registry = BackgroundTaskRegistry::new();
        let cancel = CancellationToken::new();

        let task = BackgroundShellTask::spawn(
            "test_cleanup_1".to_string(),
            "echo 'test'",
            &PathBuf::from("/tmp"),
            cancel,
        )
        .await
        .unwrap();

        registry.register(Arc::new(task));

        // Wait for completion
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Cleanup with very short max_age
        registry.cleanup_old_tasks(Duration::from_millis(1)).await;

        // Task should be removed
        assert!(!registry.exists("test_cleanup_1"));
    }

    #[tokio::test]
    async fn test_registry_get_output() {
        let registry = BackgroundTaskRegistry::new();
        let cancel = CancellationToken::new();

        let task = BackgroundShellTask::spawn(
            "test_output_1".to_string(),
            "echo 'hello'",
            &PathBuf::from("/tmp"),
            cancel,
        )
        .await
        .unwrap();

        registry.register(Arc::new(task));

        // Wait for completion
        tokio::time::sleep(Duration::from_millis(100)).await;

        let output = registry.get_output("test_output_1").await;
        assert!(output.is_some());
        let (stdout, _) = output.unwrap();
        assert!(stdout.contains("hello"));
    }

    #[tokio::test]
    async fn test_registry_get_status() {
        let registry = BackgroundTaskRegistry::new();
        let cancel = CancellationToken::new();

        let task = BackgroundShellTask::spawn(
            "test_status_1".to_string(),
            "echo 'test'",
            &PathBuf::from("/tmp"),
            cancel,
        )
        .await
        .unwrap();

        registry.register(Arc::new(task));

        // Wait for completion
        tokio::time::sleep(Duration::from_millis(100)).await;

        let status = registry.get_status("test_status_1").await;
        assert!(status.is_some());
        assert!(matches!(
            status.unwrap(),
            BackgroundTaskStatus::Completed { exit_code: 0 }
        ));

        // Nonexistent task
        let status = registry.get_status("nonexistent").await;
        assert!(status.is_none());
    }
}
