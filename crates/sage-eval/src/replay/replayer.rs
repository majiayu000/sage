//! Session replayer for regression testing
//!
//! Replays golden sessions and compares results.

use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::recorder::GoldenSession;

/// Result of replaying a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayResult {
    /// Golden session ID
    pub golden_id: String,

    /// Golden session name
    pub golden_name: String,

    /// Whether the replay matched expectations
    pub matched: bool,

    /// Whether the task passed
    pub passed: bool,

    /// Expected to pass
    pub expected_passed: bool,

    /// Missing expected files
    pub missing_files: Vec<String>,

    /// Missing expected content
    pub missing_content: Vec<String>,

    /// Differences found
    pub differences: Vec<String>,

    /// Execution time in seconds
    pub execution_time_secs: f64,
}

impl ReplayResult {
    /// Check if this is a regression
    pub fn is_regression(&self) -> bool {
        !self.matched
    }
}

/// Replayer for golden sessions
pub struct SessionReplayer {
    /// Working directory for replay
    working_dir: std::path::PathBuf,
}

impl SessionReplayer {
    /// Create a new session replayer
    pub fn new(working_dir: impl AsRef<Path>) -> Self {
        Self {
            working_dir: working_dir.as_ref().to_path_buf(),
        }
    }

    /// Replay a golden session
    pub async fn replay(&self, golden: &GoldenSession) -> Result<ReplayResult> {
        let start = std::time::Instant::now();

        // TODO: Implement actual replay using UnifiedExecutor
        // For now, return a placeholder result

        let mut result = ReplayResult {
            golden_id: golden.id.clone(),
            golden_name: golden.name.clone(),
            matched: false,
            passed: false,
            expected_passed: golden.expected_passed,
            missing_files: Vec::new(),
            missing_content: Vec::new(),
            differences: Vec::new(),
            execution_time_secs: 0.0,
        };

        // Check expected files
        for file in &golden.expected_files {
            let path = self.working_dir.join(file);
            if !path.exists() {
                result.missing_files.push(file.clone());
            }
        }

        // Check expected content
        for (file, expected_content) in &golden.expected_contents {
            let path = self.working_dir.join(file);
            if path.exists() {
                let content = tokio::fs::read_to_string(&path).await?;
                if !content.contains(expected_content) {
                    result.missing_content.push(format!(
                        "{}: expected to contain '{}'",
                        file, expected_content
                    ));
                }
            }
        }

        // Determine if matched
        result.matched = result.missing_files.is_empty()
            && result.missing_content.is_empty()
            && result.passed == golden.expected_passed;

        result.execution_time_secs = start.elapsed().as_secs_f64();

        Ok(result)
    }

    /// Replay multiple golden sessions
    pub async fn replay_all(&self, sessions: &[GoldenSession]) -> Result<Vec<ReplayResult>> {
        let mut results = Vec::new();

        for session in sessions {
            let result = self.replay(session).await?;
            results.push(result);
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_replay_with_missing_files() {
        let temp_dir = TempDir::new().unwrap();
        let replayer = SessionReplayer::new(temp_dir.path());

        let golden = GoldenSession {
            id: "test-1".to_string(),
            name: "Test Session".to_string(),
            source_session_id: "source-1".to_string(),
            task: "Create files".to_string(),
            expected_passed: true,
            expected_files: vec!["missing.txt".to_string()],
            expected_contents: std::collections::HashMap::new(),
            recorded_at: chrono::Utc::now(),
            model: "test".to_string(),
            provider: "test".to_string(),
            notes: None,
        };

        let result = replayer.replay(&golden).await.unwrap();
        assert!(!result.matched);
        assert_eq!(result.missing_files, vec!["missing.txt"]);
    }

    #[tokio::test]
    async fn test_replay_with_existing_files() {
        let temp_dir = TempDir::new().unwrap();

        // Create expected file
        tokio::fs::write(temp_dir.path().join("exists.txt"), "content")
            .await
            .unwrap();

        let replayer = SessionReplayer::new(temp_dir.path());

        let golden = GoldenSession {
            id: "test-2".to_string(),
            name: "Test Session".to_string(),
            source_session_id: "source-2".to_string(),
            task: "Check files".to_string(),
            expected_passed: false,
            expected_files: vec!["exists.txt".to_string()],
            expected_contents: std::collections::HashMap::new(),
            recorded_at: chrono::Utc::now(),
            model: "test".to_string(),
            provider: "test".to_string(),
            notes: None,
        };

        let result = replayer.replay(&golden).await.unwrap();
        assert!(result.missing_files.is_empty());
    }
}
