//! Utility functions for the unified command

use crate::console::CliConsole;
use sage_core::error::{SageError, SageResult};

/// Load task description from argument (might be a file path)
pub async fn load_task_from_arg(task: &str, console: &CliConsole) -> SageResult<String> {
    if let Ok(task_path) = std::path::Path::new(task).canonicalize() {
        if task_path.is_file() {
            console.info(&format!("Loading task from file: {}", task_path.display()));
            return tokio::fs::read_to_string(&task_path)
                .await
                .map_err(|e| SageError::config(format!("Failed to read task file: {e}")));
        }
    }
    Ok(task.to_string())
}

// Additional utility helpers belong here as needed.
