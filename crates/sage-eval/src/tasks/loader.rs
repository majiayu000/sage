//! Task loading from YAML/JSON files
//!
//! Loads evaluation tasks from the built-in tasks directory or custom paths.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use walkdir::WalkDir;

use super::{EvalTask, TaskCategory};

/// Loader for evaluation tasks
pub struct TaskLoader {
    /// Base directory for tasks
    tasks_dir: PathBuf,
}

impl TaskLoader {
    /// Create a new task loader with the given tasks directory
    pub fn new(tasks_dir: impl AsRef<Path>) -> Self {
        Self {
            tasks_dir: tasks_dir.as_ref().to_path_buf(),
        }
    }

    /// Create a loader for built-in tasks
    pub fn builtin() -> Self {
        // The tasks directory is relative to the crate root
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        Self::new(Path::new(manifest_dir).join("tasks"))
    }

    /// Load all tasks from the tasks directory
    pub fn load_all(&self) -> Result<Vec<EvalTask>> {
        let mut tasks = Vec::new();

        if !self.tasks_dir.exists() {
            return Ok(tasks);
        }

        for entry in WalkDir::new(&self.tasks_dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if Self::is_task_file(path) {
                match self.load_task_file(path) {
                    Ok(task) => tasks.push(task),
                    Err(e) => {
                        tracing::warn!("Failed to load task from {:?}: {}", path, e);
                    }
                }
            }
        }

        // Sort by category, then difficulty, then id
        tasks.sort_by(|a, b| {
            a.category
                .dir_name()
                .cmp(b.category.dir_name())
                .then_with(|| a.difficulty.cmp(&b.difficulty))
                .then_with(|| a.id.cmp(&b.id))
        });

        Ok(tasks)
    }

    /// Load tasks for specific categories
    pub fn load_categories(&self, categories: &[TaskCategory]) -> Result<Vec<EvalTask>> {
        let all_tasks = self.load_all()?;
        Ok(all_tasks
            .into_iter()
            .filter(|t| categories.contains(&t.category))
            .collect())
    }

    /// Load a single task by ID
    pub fn load_by_id(&self, id: &str) -> Result<Option<EvalTask>> {
        let all_tasks = self.load_all()?;
        Ok(all_tasks.into_iter().find(|t| t.id == id))
    }

    /// Load tasks matching a tag
    pub fn load_by_tag(&self, tag: &str) -> Result<Vec<EvalTask>> {
        let all_tasks = self.load_all()?;
        Ok(all_tasks
            .into_iter()
            .filter(|t| t.tags.iter().any(|t| t == tag))
            .collect())
    }

    /// Load a task from a file path
    fn load_task_file(&self, path: &Path) -> Result<EvalTask> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read task file: {:?}", path))?;

        let task: EvalTask = if path.extension().is_some_and(|ext| ext == "yaml" || ext == "yml") {
            serde_yaml::from_str(&content)
                .with_context(|| format!("Failed to parse YAML task: {:?}", path))?
        } else {
            serde_json::from_str(&content)
                .with_context(|| format!("Failed to parse JSON task: {:?}", path))?
        };

        Ok(task)
    }

    /// Check if a path is a task file
    fn is_task_file(path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }

        match path.extension().and_then(|e| e.to_str()) {
            Some("yaml") | Some("yml") | Some("json") => true,
            _ => false,
        }
    }

    /// List available task IDs
    pub fn list_task_ids(&self) -> Result<Vec<String>> {
        let tasks = self.load_all()?;
        Ok(tasks.into_iter().map(|t| t.id).collect())
    }

    /// Get task count by category
    pub fn count_by_category(&self) -> Result<std::collections::HashMap<TaskCategory, usize>> {
        let tasks = self.load_all()?;
        let mut counts = std::collections::HashMap::new();

        for task in tasks {
            *counts.entry(task.category).or_insert(0) += 1;
        }

        Ok(counts)
    }
}

/// Load tasks from a YAML string (useful for testing)
pub fn load_tasks_from_yaml(yaml: &str) -> Result<Vec<EvalTask>> {
    let tasks: Vec<EvalTask> = serde_yaml::from_str(yaml)?;
    Ok(tasks)
}

/// Load a single task from a YAML string
pub fn load_task_from_yaml(yaml: &str) -> Result<EvalTask> {
    let task: EvalTask = serde_yaml::from_str(yaml)?;
    Ok(task)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::{Difficulty, Verifier};

    #[test]
    fn test_load_task_from_yaml() {
        let yaml = r#"
id: test-001
name: Test Task
description: A test task
category: code_generation
difficulty: easy
verifier:
  type: file_exists
  path: output.txt
"#;

        let task = load_task_from_yaml(yaml).unwrap();
        assert_eq!(task.id, "test-001");
        assert_eq!(task.category, TaskCategory::CodeGeneration);
        assert_eq!(task.difficulty, Difficulty::Easy);
    }

    #[test]
    fn test_load_task_with_setup_files() {
        let yaml = r#"
id: test-002
name: Test with Setup
description: A test with setup files
category: code_editing
difficulty: medium
setup_files:
  main.py: |
    def hello():
        pass
  test_main.py: |
    from main import hello
    def test_hello():
        assert hello() == "Hello"
verifier:
  type: python_test
"#;

        let task = load_task_from_yaml(yaml).unwrap();
        assert_eq!(task.setup_files.len(), 2);
        assert!(task.setup_files.contains_key("main.py"));
    }

    #[test]
    fn test_load_task_with_all_verifier() {
        let yaml = r#"
id: test-003
name: Multi-verifier Task
description: Task with multiple verifiers
category: multi_file
difficulty: hard
verifier:
  type: all
  verifiers:
    - type: file_exists
      path: src/main.rs
    - type: file_exists
      path: Cargo.toml
    - type: rust_test
"#;

        let task = load_task_from_yaml(yaml).unwrap();
        match task.verifier {
            Verifier::All { verifiers } => {
                assert_eq!(verifiers.len(), 3);
            }
            _ => panic!("Expected All verifier"),
        }
    }
}
