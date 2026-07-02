//! Eval report types.

use crate::metrics::{TaskToolMetrics, ToolMetricsSummary};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvalReport {
    pub suite_name: String,
    pub task_count: u32,
    pub passed_count: u32,
    pub pass_at_1: f64,
    pub tool_metrics: ToolMetricsSummary,
    pub tasks: Vec<TaskReport>,
}

impl EvalReport {
    pub fn new(suite_name: String, tasks: Vec<TaskReport>) -> Self {
        let task_count = tasks.len() as u32;
        let passed_count = tasks.iter().filter(|task| task.passed).count() as u32;
        let pass_at_1 = if task_count == 0 {
            0.0
        } else {
            passed_count as f64 / task_count as f64
        };
        let task_metrics = tasks
            .iter()
            .map(|task| task.tool_metrics.clone())
            .collect::<Vec<_>>();

        Self {
            suite_name,
            task_count,
            passed_count,
            pass_at_1,
            tool_metrics: ToolMetricsSummary::from_tasks(&task_metrics),
            tasks,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskReport {
    pub task_id: String,
    pub passed: bool,
    pub assertion_results: Vec<bool>,
    pub trajectory_path: PathBuf,
    pub tool_metrics: TaskToolMetrics,
}
