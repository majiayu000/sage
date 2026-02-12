//! Task completion verification
//!
//! Provides multi-dimensional verification of task completion,
//! inspired by Claude Code's approach.

use crate::llm::messages::LlmResponse;
use crate::tools::types::ToolResult;
use std::collections::HashSet;

/// Task type classification for completion verification
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionTaskType {
    /// Task that requires code output (create, implement, build)
    CodeImplementation,
    /// Task that requires fixing existing code
    BugFix,
    /// Task that only requires research/analysis
    Research,
    /// Task that explicitly requests documentation
    Documentation,
    /// Unknown or general task
    General,
}

impl CompletionTaskType {
    /// Determine task type from description
    pub fn from_description(description: &str) -> Self {
        let lower = description.to_lowercase();

        // Check for documentation-specific keywords
        if lower.contains("文档")
            || lower.contains("readme")
            || lower.contains("document")
            || lower.contains("write doc")
        {
            return CompletionTaskType::Documentation;
        }

        // Check for research keywords
        if lower.contains("分析")
            || lower.contains("研究")
            || lower.contains("调查")
            || lower.contains("investigate")
            || lower.contains("analyze")
            || lower.contains("research")
            || lower.contains("explain")
            || lower.contains("what is")
        {
            return CompletionTaskType::Research;
        }

        // Check for bug fix keywords
        if lower.contains("修复")
            || lower.contains("fix")
            || lower.contains("bug")
            || lower.contains("error")
            || lower.contains("issue")
            || lower.contains("problem")
        {
            return CompletionTaskType::BugFix;
        }

        // Check for code implementation keywords
        if lower.contains("设计")
            || lower.contains("创建")
            || lower.contains("实现")
            || lower.contains("开发")
            || lower.contains("做")
            || lower.contains("写")
            || lower.contains("design")
            || lower.contains("create")
            || lower.contains("implement")
            || lower.contains("build")
            || lower.contains("make")
            || lower.contains("develop")
            || lower.contains("add")
            || lower.contains("网站")
            || lower.contains("website")
            || lower.contains("app")
            || lower.contains("应用")
        {
            return CompletionTaskType::CodeImplementation;
        }

        CompletionTaskType::General
    }

    /// Check if this task type requires code files to be created/modified
    pub fn requires_code(&self) -> bool {
        matches!(
            self,
            CompletionTaskType::CodeImplementation | CompletionTaskType::BugFix
        )
    }
}

/// Tracks file operations for completion verification
#[derive(Debug, Clone, Default)]
pub struct FileOperationTracker {
    /// Files created via Write tool
    pub created_files: HashSet<String>,
    /// Files modified via Edit tool
    pub modified_files: HashSet<String>,
    /// Files read via Read tool
    pub read_files: HashSet<String>,
}

impl FileOperationTracker {
    /// Create a new tracker
    pub fn new() -> Self {
        Self::default()
    }

    /// Track a tool result
    pub fn track_tool_result(&mut self, tool_name: &str, result: &ToolResult) {
        if !result.success {
            return;
        }

        // Extract file path from metadata or result
        let file_path = result
            .metadata
            .get("file_path")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        use crate::tools::names::file_ops;
        match tool_name {
            file_ops::WRITE => {
                if let Some(path) = file_path {
                    self.created_files.insert(path);
                }
            }
            file_ops::EDIT | file_ops::MULTI_EDIT => {
                if let Some(path) = file_path {
                    self.modified_files.insert(path);
                }
            }
            file_ops::READ => {
                if let Some(path) = file_path {
                    self.read_files.insert(path);
                }
            }
            _ => {}
        }
    }

    /// Check if any file operations were performed
    pub fn has_file_operations(&self) -> bool {
        !self.created_files.is_empty() || !self.modified_files.is_empty()
    }

    /// Get count of files created
    pub fn created_count(&self) -> usize {
        self.created_files.len()
    }

    /// Get count of files modified
    pub fn modified_count(&self) -> usize {
        self.modified_files.len()
    }

    /// Get all affected files
    pub fn all_affected_files(&self) -> Vec<&String> {
        self.created_files
            .iter()
            .chain(self.modified_files.iter())
            .collect()
    }

    /// Reset the tracker
    pub fn reset(&mut self) {
        self.created_files.clear();
        self.modified_files.clear();
        self.read_files.clear();
    }
}

/// Completion status
#[derive(Debug, Clone)]
pub enum CompletionStatus {
    /// Task is complete
    Completed {
        summary: String,
        files_created: usize,
        files_modified: usize,
    },
    /// Task should continue
    Continue { reason: String },
    /// Task was blocked (e.g., by hook)
    Blocked { reason: String },
    /// Reached execution limits
    LimitReached { limit_type: LimitType },
    /// Warning - task marked complete but concerns exist
    CompletedWithWarning { summary: String, warning: String },
}

/// Types of limits that can be reached
#[derive(Debug, Clone)]
pub enum LimitType {
    MaxSteps,
    TokenBudget,
    Timeout,
}

/// Completion checker for verifying task completion
#[derive(Debug)]
pub struct CompletionChecker {
    task_type: CompletionTaskType,
    file_tracker: FileOperationTracker,
    strict_mode: bool,
}

impl CompletionChecker {
    /// Create a new completion checker
    pub fn new(task_description: &str) -> Self {
        Self {
            task_type: CompletionTaskType::from_description(task_description),
            file_tracker: FileOperationTracker::new(),
            strict_mode: true,
        }
    }

    /// Create with explicit task type
    pub fn with_task_type(task_type: CompletionTaskType) -> Self {
        Self {
            task_type,
            file_tracker: FileOperationTracker::new(),
            strict_mode: true,
        }
    }

    /// Disable strict mode (allows completion without file operations)
    pub fn disable_strict_mode(&mut self) {
        self.strict_mode = false;
    }

    /// Get the detected task type
    pub fn task_type(&self) -> &CompletionTaskType {
        &self.task_type
    }

    /// Get the file tracker
    pub fn file_tracker(&self) -> &FileOperationTracker {
        &self.file_tracker
    }

    /// Get mutable file tracker
    pub fn file_tracker_mut(&mut self) -> &mut FileOperationTracker {
        &mut self.file_tracker
    }

    /// Track tool results
    pub fn track_tool_results(&mut self, results: &[ToolResult]) {
        for result in results {
            self.file_tracker
                .track_tool_result(&result.tool_name, result);
        }
    }

    /// Check if task_done was called in tool results
    fn find_task_done_summary(&self, results: &[ToolResult]) -> Option<String> {
        use crate::tools::names::task_mgmt;
        results
            .iter()
            .find(|r| r.tool_name == task_mgmt::TASK_DONE && r.success)
            .and_then(|r| r.output.clone())
    }

    /// Check completion status
    pub fn check(&self, response: &LlmResponse, tool_results: &[ToolResult]) -> CompletionStatus {
        // Check if task_done was called
        if let Some(summary) = self.find_task_done_summary(tool_results) {
            // For code tasks, verify file operations were performed
            if self.task_type.requires_code()
                && !self.file_tracker.has_file_operations()
                && self.strict_mode
            {
                return CompletionStatus::CompletedWithWarning {
                    summary: summary.clone(),
                    warning: "Task marked complete but no code files were created or modified. \
                             This may indicate the task was not fully implemented."
                        .to_string(),
                };
            }

            return CompletionStatus::Completed {
                summary,
                files_created: self.file_tracker.created_count(),
                files_modified: self.file_tracker.modified_count(),
            };
        }

        // Check for natural completion (no tool calls, natural end)
        // Support multiple LLM providers with different finish_reason values:
        // - Anthropic: "end_turn"
        // - OpenAI/GLM/others: "stop"
        // - Google: "STOP"
        let is_natural_end = matches!(
            response.finish_reason.as_deref(),
            Some("end_turn") | Some("stop") | Some("STOP")
        );
        if response.tool_calls.is_empty() && is_natural_end {
            // This might be a conversational response, allow it to continue
            return CompletionStatus::Continue {
                reason: "Response ended without tool calls".to_string(),
            };
        }

        // Continue execution
        CompletionStatus::Continue {
            reason: "Task not yet complete".to_string(),
        }
    }

    /// Quick check if we should continue
    pub fn should_continue(&self, response: &LlmResponse, tool_results: &[ToolResult]) -> bool {
        matches!(
            self.check(response, tool_results),
            CompletionStatus::Continue { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_type_detection() {
        assert_eq!(
            CompletionTaskType::from_description("设计一个天气网站"),
            CompletionTaskType::CodeImplementation
        );
        assert_eq!(
            CompletionTaskType::from_description("Create a weather app"),
            CompletionTaskType::CodeImplementation
        );
        assert_eq!(
            CompletionTaskType::from_description("Fix the bug in login"),
            CompletionTaskType::BugFix
        );
        assert_eq!(
            CompletionTaskType::from_description("修复登录问题"),
            CompletionTaskType::BugFix
        );
        assert_eq!(
            CompletionTaskType::from_description("分析这个代码的性能"),
            CompletionTaskType::Research
        );
        assert_eq!(
            CompletionTaskType::from_description("Write documentation for API"),
            CompletionTaskType::Documentation
        );
    }

    #[test]
    fn test_task_type_requires_code() {
        assert!(CompletionTaskType::CodeImplementation.requires_code());
        assert!(CompletionTaskType::BugFix.requires_code());
        assert!(!CompletionTaskType::Research.requires_code());
        assert!(!CompletionTaskType::Documentation.requires_code());
    }

    #[test]
    fn test_file_operation_tracker() {
        let mut tracker = FileOperationTracker::new();
        assert!(!tracker.has_file_operations());

        // Simulate a Write operation
        let write_result = ToolResult {
            call_id: "1".to_string(),
            tool_name: "Write".to_string(),
            success: true,
            output: Some("File created".to_string()),
            error: None,
            exit_code: None,
            execution_time_ms: Some(100),
            metadata: {
                let mut m = std::collections::HashMap::new();
                m.insert("file_path".to_string(), serde_json::json!("/test/file.rs"));
                m
            },
        };

        tracker.track_tool_result("Write", &write_result);
        assert!(tracker.has_file_operations());
        assert_eq!(tracker.created_count(), 1);
    }

    #[test]
    fn test_completion_checker_requires_code() {
        let checker = CompletionChecker::new("设计一个天气网站");
        assert!(checker.task_type().requires_code());

        let checker = CompletionChecker::new("分析代码");
        assert!(!checker.task_type().requires_code());
    }
}
