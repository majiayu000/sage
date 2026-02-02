//! Core evaluation task types
//!
//! Defines the structure of evaluation tasks including categories,
//! difficulty levels, and task metadata.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::Verifier;

/// Category of evaluation task
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskCategory {
    /// Generate new code from scratch
    CodeGeneration,
    /// Edit existing code
    CodeEditing,
    /// Fix bugs in code
    BugFixing,
    /// Refactor code for better structure
    Refactoring,
    /// Tasks involving multiple files
    MultiFile,
}

impl TaskCategory {
    /// Get the directory name for this category
    pub fn dir_name(&self) -> &'static str {
        match self {
            TaskCategory::CodeGeneration => "code_generation",
            TaskCategory::CodeEditing => "code_editing",
            TaskCategory::BugFixing => "bug_fixing",
            TaskCategory::Refactoring => "refactoring",
            TaskCategory::MultiFile => "multi_file",
        }
    }

    /// Get display name for this category
    pub fn display_name(&self) -> &'static str {
        match self {
            TaskCategory::CodeGeneration => "Code Generation",
            TaskCategory::CodeEditing => "Code Editing",
            TaskCategory::BugFixing => "Bug Fixing",
            TaskCategory::Refactoring => "Refactoring",
            TaskCategory::MultiFile => "Multi-File",
        }
    }

    /// Get all categories
    pub fn all() -> &'static [TaskCategory] {
        &[
            TaskCategory::CodeGeneration,
            TaskCategory::CodeEditing,
            TaskCategory::BugFixing,
            TaskCategory::Refactoring,
            TaskCategory::MultiFile,
        ]
    }
}

impl std::fmt::Display for TaskCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Difficulty level of a task
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

impl Difficulty {
    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Difficulty::Easy => "Easy",
            Difficulty::Medium => "Medium",
            Difficulty::Hard => "Hard",
        }
    }

    /// Get expected turn multiplier for scoring
    pub fn turn_multiplier(&self) -> f64 {
        match self {
            Difficulty::Easy => 1.0,
            Difficulty::Medium => 1.5,
            Difficulty::Hard => 2.0,
        }
    }
}

impl std::fmt::Display for Difficulty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// An evaluation task definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalTask {
    /// Unique identifier for the task
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Description/prompt given to the agent
    pub description: String,

    /// Task category
    pub category: TaskCategory,

    /// Difficulty level
    pub difficulty: Difficulty,

    /// Initial files to set up in the sandbox
    /// Key: relative file path, Value: file content
    #[serde(default)]
    pub setup_files: HashMap<String, String>,

    /// Verifier to check task completion
    pub verifier: Verifier,

    /// Expected maximum turns for this task (for scoring)
    #[serde(default)]
    pub expected_max_turns: Option<u32>,

    /// Timeout in seconds (default: 300)
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,

    /// Tags for filtering
    #[serde(default)]
    pub tags: Vec<String>,

    /// Language(s) involved
    #[serde(default)]
    pub languages: Vec<String>,
}

fn default_timeout() -> u64 {
    300
}

impl EvalTask {
    /// Create a new evaluation task
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        category: TaskCategory,
        difficulty: Difficulty,
        verifier: Verifier,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            category,
            difficulty,
            setup_files: HashMap::new(),
            verifier,
            expected_max_turns: None,
            timeout_secs: default_timeout(),
            tags: Vec::new(),
            languages: Vec::new(),
        }
    }

    /// Add a setup file
    pub fn with_setup_file(mut self, path: impl Into<String>, content: impl Into<String>) -> Self {
        self.setup_files.insert(path.into(), content.into());
        self
    }

    /// Set expected max turns
    pub fn with_expected_max_turns(mut self, turns: u32) -> Self {
        self.expected_max_turns = Some(turns);
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// Add a tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add a language
    pub fn with_language(mut self, lang: impl Into<String>) -> Self {
        self.languages.push(lang.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_category_dir_name() {
        assert_eq!(TaskCategory::CodeGeneration.dir_name(), "code_generation");
        assert_eq!(TaskCategory::BugFixing.dir_name(), "bug_fixing");
    }

    #[test]
    fn test_difficulty_ordering() {
        assert!(Difficulty::Easy < Difficulty::Medium);
        assert!(Difficulty::Medium < Difficulty::Hard);
    }

    #[test]
    fn test_eval_task_builder() {
        let task = EvalTask::new(
            "test-001",
            "Test Task",
            "Do something",
            TaskCategory::CodeGeneration,
            Difficulty::Easy,
            Verifier::FileExists {
                path: "output.txt".to_string(),
            },
        )
        .with_setup_file("input.txt", "hello")
        .with_expected_max_turns(5)
        .with_tag("test");

        assert_eq!(task.id, "test-001");
        assert_eq!(task.setup_files.len(), 1);
        assert_eq!(task.expected_max_turns, Some(5));
        assert_eq!(task.tags, vec!["test"]);
    }
}
