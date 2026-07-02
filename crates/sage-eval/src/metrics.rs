//! Tool-use metrics with recognition separated from execution.

use crate::task::EvalTask;
use sage_core::trajectory::SessionEntry;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskToolMetrics {
    pub task_id: String,
    pub recognition_required: bool,
    pub execution_required: bool,
    pub recognition_correct: bool,
    pub execution_correct: bool,
    pub recognition_correct_execution_missing: bool,
    pub execution_without_recognition: bool,
    pub recognition_source_entry_ids: Vec<String>,
    pub execution_source_entry_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ToolMetricsSummary {
    pub recognition_total: u32,
    pub recognition_correct: u32,
    pub execution_total: u32,
    pub execution_correct: u32,
    pub recognition_correct_execution_missing: u32,
    pub execution_without_recognition: u32,
    pub recognition_accuracy: Option<f64>,
    pub execution_accuracy: Option<f64>,
}

pub fn evaluate_tool_metrics(task: &EvalTask, entries: &[SessionEntry]) -> TaskToolMetrics {
    let required_categories = normalized_set(&task.required_tool_categories);
    let expected_tools = normalized_set(&task.expected_tool_names);
    let mut recognized_categories = HashSet::new();
    let mut executed_tools = HashSet::new();
    let mut recognition_source_entry_ids = Vec::new();
    let mut execution_source_entry_ids = Vec::new();

    for entry in entries {
        match entry {
            SessionEntry::ToolIntent {
                uuid,
                tool_category,
                ..
            } => {
                recognized_categories.insert(normalize(tool_category));
                recognition_source_entry_ids.push(uuid.to_string());
            }
            SessionEntry::ToolCall {
                uuid, tool_name, ..
            } => {
                executed_tools.insert(normalize(tool_name));
                execution_source_entry_ids.push(uuid.to_string());
            }
            _ => {}
        }
    }

    let recognition_required = !required_categories.is_empty();
    let execution_required = !expected_tools.is_empty();
    let recognition_correct =
        !recognition_required || required_categories.is_subset(&recognized_categories);
    let execution_correct = !execution_required || expected_tools.is_subset(&executed_tools);

    TaskToolMetrics {
        task_id: task.id.clone(),
        recognition_required,
        execution_required,
        recognition_correct,
        execution_correct,
        recognition_correct_execution_missing: recognition_required
            && execution_required
            && recognition_correct
            && !execution_correct,
        execution_without_recognition: recognition_required
            && execution_required
            && execution_correct
            && !recognition_correct,
        recognition_source_entry_ids,
        execution_source_entry_ids,
    }
}

impl ToolMetricsSummary {
    pub fn from_tasks(tasks: &[TaskToolMetrics]) -> Self {
        let mut summary = Self::default();
        for task in tasks {
            if task.recognition_required {
                summary.recognition_total += 1;
                if task.recognition_correct {
                    summary.recognition_correct += 1;
                }
            }
            if task.execution_required {
                summary.execution_total += 1;
                if task.execution_correct {
                    summary.execution_correct += 1;
                }
            }
            if task.recognition_correct_execution_missing {
                summary.recognition_correct_execution_missing += 1;
            }
            if task.execution_without_recognition {
                summary.execution_without_recognition += 1;
            }
        }
        summary.recognition_accuracy =
            ratio(summary.recognition_correct, summary.recognition_total);
        summary.execution_accuracy = ratio(summary.execution_correct, summary.execution_total);
        summary
    }
}

fn normalized_set(values: &[String]) -> HashSet<String> {
    values.iter().map(|value| normalize(value)).collect()
}

fn normalize(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn ratio(numerator: u32, denominator: u32) -> Option<f64> {
    (denominator > 0).then(|| numerator as f64 / denominator as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use uuid::Uuid;

    fn task() -> EvalTask {
        EvalTask {
            id: "read-marker".to_string(),
            prompt: "Read marker".to_string(),
            required_tool_categories: vec!["file_read".to_string()],
            expected_tool_names: vec!["Read".to_string()],
            workspace_files: Vec::new(),
            assertions: Vec::new(),
            offline: None,
        }
    }

    #[test]
    fn separates_recognition_correct_from_missing_execution() {
        let entries = vec![SessionEntry::ToolIntent {
            uuid: Uuid::from_u128(1),
            parent_uuid: None,
            tool_category: "file_read".to_string(),
            tool_name: Some("Read".to_string()),
            reason: "Need file".to_string(),
            timestamp: "1970-01-01T00:00:00Z".to_string(),
        }];

        let metrics = evaluate_tool_metrics(&task(), &entries);

        assert!(metrics.recognition_correct);
        assert!(!metrics.execution_correct);
        assert!(metrics.recognition_correct_execution_missing);
        assert!(!metrics.execution_without_recognition);
    }

    #[test]
    fn separates_execution_without_recognition() {
        let entries = vec![SessionEntry::ToolCall {
            uuid: Uuid::from_u128(2),
            parent_uuid: None,
            tool_name: "Read".to_string(),
            tool_input: json!({"file_path": "marker.txt"}),
            timestamp: "1970-01-01T00:00:00Z".to_string(),
        }];

        let metrics = evaluate_tool_metrics(&task(), &entries);

        assert!(!metrics.recognition_correct);
        assert!(metrics.execution_correct);
        assert!(metrics.execution_without_recognition);
    }
}
