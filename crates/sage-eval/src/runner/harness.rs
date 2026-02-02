//! Test harness for running individual evaluation tasks
//!
//! Provides the infrastructure for executing a single task in a sandbox.

use std::time::Duration;

use anyhow::{Context, Result};
use tokio::time::timeout;

use super::Sandbox;
use crate::metrics::{MetricsCollector, TaskStatus};
use crate::tasks::{EvalTask, VerifierResult};

/// Test harness for running a single evaluation task
pub struct TestHarness {
    /// The task to run
    task: EvalTask,

    /// Sandbox environment
    sandbox: Sandbox,

    /// Metrics collector
    collector: MetricsCollector,

    /// Timeout duration
    timeout: Duration,
}

impl TestHarness {
    /// Create a new test harness for a task
    pub async fn new(task: EvalTask, timeout_secs: u64) -> Result<Self> {
        let sandbox = Sandbox::new().context("Failed to create sandbox")?;

        // Set up initial files
        sandbox
            .setup_files(&task.setup_files)
            .await
            .context("Failed to setup task files")?;

        Ok(Self {
            task,
            sandbox,
            collector: MetricsCollector::new(),
            timeout: Duration::from_secs(timeout_secs),
        })
    }

    /// Get the sandbox root path
    pub fn sandbox_root(&self) -> &std::path::Path {
        self.sandbox.root()
    }

    /// Get the task
    pub fn task(&self) -> &EvalTask {
        &self.task
    }

    /// Start tracking metrics for an attempt
    pub fn start_attempt(&mut self, attempt: u32) {
        self.collector.start_task(
            &self.task.id,
            &self.task.name,
            self.task.category,
            self.task.difficulty,
            attempt,
        );
    }

    /// Record a turn/step
    pub fn record_turn(&mut self, input_tokens: u64, output_tokens: u64) {
        self.collector.record_turn(input_tokens, output_tokens);
    }

    /// Record tool usage
    pub fn record_tool_use(&mut self, tool_name: &str) {
        self.collector.record_tool_use(tool_name);
    }

    /// Verify the task completion
    pub async fn verify(&self) -> VerifierResult {
        // Run verification with timeout
        let verify_timeout = Duration::from_secs(60); // 1 minute for verification

        match timeout(verify_timeout, self.task.verifier.verify(self.sandbox.root())).await {
            Ok(result) => result,
            Err(_) => VerifierResult::fail("Verification timed out"),
        }
    }

    /// Complete the current attempt with verification
    pub async fn complete_attempt(&mut self) -> crate::metrics::TaskResult {
        let verify_result = self.verify().await;

        let status = if verify_result.passed {
            TaskStatus::Passed
        } else {
            TaskStatus::Failed
        };

        self.collector
            .complete_task(status, None, verify_result.details)
            .expect("No active task to complete")
    }

    /// Complete the current attempt with an error
    pub fn complete_with_error(&mut self, error: String) -> crate::metrics::TaskResult {
        self.collector
            .complete_task(TaskStatus::Error, Some(error), None)
            .expect("No active task to complete")
    }

    /// Complete the current attempt with timeout
    pub fn complete_with_timeout(&mut self) -> crate::metrics::TaskResult {
        self.collector
            .complete_task(
                TaskStatus::Timeout,
                Some("Task execution timed out".to_string()),
                None,
            )
            .expect("No active task to complete")
    }

    /// Get the timeout duration
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Take the metrics collector
    pub fn take_collector(self) -> MetricsCollector {
        self.collector
    }

    /// Clean up the sandbox
    pub async fn cleanup(mut self) -> Result<()> {
        self.sandbox.cleanup().await
    }

    /// Preserve the sandbox for debugging
    pub fn preserve_sandbox(&mut self) {
        self.sandbox.set_preserve(true);
    }
}

/// Result of running a task through the harness
#[derive(Debug)]
#[allow(dead_code)]
pub struct HarnessResult {
    /// Task ID
    pub task_id: String,

    /// Whether the task passed
    pub passed: bool,

    /// Number of attempts made
    pub attempts: u32,

    /// Verification result
    pub verification: VerifierResult,

    /// Error message if any
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::{Difficulty, TaskCategory, Verifier};

    #[tokio::test]
    async fn test_harness_creation() {
        let task = EvalTask::new(
            "test-001",
            "Test Task",
            "Create a file",
            TaskCategory::CodeGeneration,
            Difficulty::Easy,
            Verifier::FileExists {
                path: "output.txt".to_string(),
            },
        );

        let harness = TestHarness::new(task, 60).await.unwrap();
        assert!(harness.sandbox_root().exists());
    }

    #[tokio::test]
    async fn test_harness_with_setup_files() {
        let task = EvalTask::new(
            "test-002",
            "Test with Setup",
            "Modify the file",
            TaskCategory::CodeEditing,
            Difficulty::Easy,
            Verifier::FileContains {
                path: "test.txt".to_string(),
                contains: "modified".to_string(),
                ignore_case: false,
            },
        )
        .with_setup_file("test.txt", "original content");

        let harness = TestHarness::new(task, 60).await.unwrap();

        // Check setup file exists
        assert!(harness.sandbox_root().join("test.txt").exists());
    }

    #[tokio::test]
    async fn test_harness_verification() {
        let task = EvalTask::new(
            "test-003",
            "Verify Test",
            "Check file exists",
            TaskCategory::CodeGeneration,
            Difficulty::Easy,
            Verifier::FileExists {
                path: "created.txt".to_string(),
            },
        );

        let harness = TestHarness::new(task, 60).await.unwrap();

        // Verification should fail (file doesn't exist)
        let result = harness.verify().await;
        assert!(!result.passed);

        // Create the file
        tokio::fs::write(harness.sandbox_root().join("created.txt"), "content")
            .await
            .unwrap();

        // Verification should pass now
        let result = harness.verify().await;
        assert!(result.passed);
    }
}
