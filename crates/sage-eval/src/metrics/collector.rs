//! Metrics collector for tracking evaluation progress
//!
//! Collects metrics during task execution.

use std::collections::HashMap;
use std::time::Instant;

use super::types::{TaskResult, TaskStatus};
use crate::tasks::{Difficulty, TaskCategory};

/// Collector for gathering metrics during evaluation
pub struct MetricsCollector {
    /// Results collected so far
    results: Vec<TaskResult>,

    /// Current task being executed
    current_task: Option<CurrentTask>,

    /// Start time of evaluation
    start_time: Instant,
}

/// State for the currently executing task
struct CurrentTask {
    task_id: String,
    task_name: String,
    category: TaskCategory,
    difficulty: Difficulty,
    attempt: u32,
    start_time: Instant,
    turns: u32,
    input_tokens: u64,
    output_tokens: u64,
    tool_usage: HashMap<String, u32>,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
            current_task: None,
            start_time: Instant::now(),
        }
    }

    /// Start tracking a new task
    pub fn start_task(
        &mut self,
        task_id: impl Into<String>,
        task_name: impl Into<String>,
        category: TaskCategory,
        difficulty: Difficulty,
        attempt: u32,
    ) {
        self.current_task = Some(CurrentTask {
            task_id: task_id.into(),
            task_name: task_name.into(),
            category,
            difficulty,
            attempt,
            start_time: Instant::now(),
            turns: 0,
            input_tokens: 0,
            output_tokens: 0,
            tool_usage: HashMap::new(),
        });
    }

    /// Record a turn/step completion
    pub fn record_turn(&mut self, input_tokens: u64, output_tokens: u64) {
        if let Some(task) = &mut self.current_task {
            task.turns += 1;
            task.input_tokens += input_tokens;
            task.output_tokens += output_tokens;
        }
    }

    /// Record tool usage
    pub fn record_tool_use(&mut self, tool_name: &str) {
        if let Some(task) = &mut self.current_task {
            *task.tool_usage.entry(tool_name.to_string()).or_insert(0) += 1;
        }
    }

    /// Complete the current task with a result
    pub fn complete_task(
        &mut self,
        status: TaskStatus,
        error_message: Option<String>,
        verifier_output: Option<String>,
    ) -> Option<TaskResult> {
        let task = self.current_task.take()?;

        let mut result = TaskResult::new(
            task.task_id,
            task.task_name,
            task.category,
            task.difficulty,
            status,
        );

        result.turns = task.turns;
        result.input_tokens = task.input_tokens;
        result.output_tokens = task.output_tokens;
        result.total_tokens = task.input_tokens + task.output_tokens;
        result.execution_time_secs = task.start_time.elapsed().as_secs_f64();
        result.attempt = task.attempt;
        result.error_message = error_message;
        result.verifier_output = verifier_output;
        result.tool_usage = task.tool_usage;

        self.results.push(result.clone());
        Some(result)
    }

    /// Get all collected results
    pub fn results(&self) -> &[TaskResult] {
        &self.results
    }

    /// Take all collected results
    pub fn take_results(self) -> Vec<TaskResult> {
        self.results
    }

    /// Get total elapsed time
    pub fn elapsed_secs(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }

    /// Get count of passed tasks
    pub fn passed_count(&self) -> usize {
        self.results.iter().filter(|r| r.passed()).count()
    }

    /// Get count of failed tasks
    pub fn failed_count(&self) -> usize {
        self.results.iter().filter(|r| !r.passed()).count()
    }

    /// Check if there's a task in progress
    pub fn has_active_task(&self) -> bool {
        self.current_task.is_some()
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collector_basic_flow() {
        let mut collector = MetricsCollector::new();

        collector.start_task(
            "test-001",
            "Test Task",
            TaskCategory::CodeGeneration,
            Difficulty::Easy,
            1,
        );

        collector.record_turn(100, 50);
        collector.record_turn(80, 40);
        collector.record_tool_use("bash");
        collector.record_tool_use("edit");
        collector.record_tool_use("bash");

        let result = collector.complete_task(TaskStatus::Passed, None, None);

        assert!(result.is_some());
        let result = result.unwrap();
        assert!(result.passed());
        assert_eq!(result.turns, 2);
        assert_eq!(result.input_tokens, 180);
        assert_eq!(result.output_tokens, 90);
        assert_eq!(result.tool_usage.get("bash"), Some(&2));
        assert_eq!(result.tool_usage.get("edit"), Some(&1));
    }

    #[test]
    fn test_collector_multiple_tasks() {
        let mut collector = MetricsCollector::new();

        // Task 1 - passed
        collector.start_task("t1", "Task 1", TaskCategory::CodeGeneration, Difficulty::Easy, 1);
        collector.record_turn(100, 50);
        collector.complete_task(TaskStatus::Passed, None, None);

        // Task 2 - failed
        collector.start_task("t2", "Task 2", TaskCategory::BugFixing, Difficulty::Medium, 1);
        collector.record_turn(200, 100);
        collector.complete_task(
            TaskStatus::Failed,
            Some("Test failed".to_string()),
            None,
        );

        assert_eq!(collector.passed_count(), 1);
        assert_eq!(collector.failed_count(), 1);
        assert_eq!(collector.results().len(), 2);
    }
}
