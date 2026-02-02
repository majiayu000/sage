//! Golden session recorder
//!
//! Records successful sessions as "golden" references for regression testing.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// A golden session recording
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoldenSession {
    /// Unique identifier
    pub id: String,

    /// Human-readable name/description
    pub name: String,

    /// Original session ID this was recorded from
    pub source_session_id: String,

    /// Task description
    pub task: String,

    /// Expected outcome (passed/failed)
    pub expected_passed: bool,

    /// Expected files to exist after execution
    pub expected_files: Vec<String>,

    /// Expected file contents (path -> content substring)
    pub expected_contents: std::collections::HashMap<String, String>,

    /// Recording timestamp
    pub recorded_at: chrono::DateTime<Utc>,

    /// Model used during recording
    pub model: String,

    /// Provider used
    pub provider: String,

    /// Notes about this golden session
    pub notes: Option<String>,
}

/// Recorder for creating golden sessions
pub struct GoldenRecorder {
    /// Output directory for golden sessions
    output_dir: PathBuf,
}

impl GoldenRecorder {
    /// Create a new golden recorder
    pub fn new(output_dir: impl AsRef<Path>) -> Self {
        Self {
            output_dir: output_dir.as_ref().to_path_buf(),
        }
    }

    /// Record a session as golden
    pub async fn record(
        &self,
        session_id: &str,
        name: &str,
        task: &str,
        expected_passed: bool,
        expected_files: Vec<String>,
        model: &str,
        provider: &str,
    ) -> Result<GoldenSession> {
        let golden = GoldenSession {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            source_session_id: session_id.to_string(),
            task: task.to_string(),
            expected_passed,
            expected_files,
            expected_contents: std::collections::HashMap::new(),
            recorded_at: Utc::now(),
            model: model.to_string(),
            provider: provider.to_string(),
            notes: None,
        };

        self.save(&golden).await?;
        Ok(golden)
    }

    /// Save a golden session to disk
    async fn save(&self, golden: &GoldenSession) -> Result<()> {
        tokio::fs::create_dir_all(&self.output_dir).await?;

        let filename = format!("{}.golden.json", golden.id);
        let path = self.output_dir.join(filename);

        let json = serde_json::to_string_pretty(golden)?;
        tokio::fs::write(&path, json)
            .await
            .with_context(|| format!("Failed to save golden session to {:?}", path))?;

        tracing::info!("Saved golden session: {} -> {:?}", golden.name, path);
        Ok(())
    }

    /// Load a golden session from disk
    pub async fn load(&self, id: &str) -> Result<GoldenSession> {
        let filename = format!("{}.golden.json", id);
        let path = self.output_dir.join(filename);

        let content = tokio::fs::read_to_string(&path)
            .await
            .with_context(|| format!("Failed to read golden session from {:?}", path))?;

        let golden: GoldenSession = serde_json::from_str(&content)?;
        Ok(golden)
    }

    /// List all golden sessions
    pub async fn list(&self) -> Result<Vec<GoldenSession>> {
        let mut sessions = Vec::new();

        if !self.output_dir.exists() {
            return Ok(sessions);
        }

        let mut entries = tokio::fs::read_dir(&self.output_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json")
                && path
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().contains(".golden."))
            {
                let content = tokio::fs::read_to_string(&path).await?;
                if let Ok(golden) = serde_json::from_str::<GoldenSession>(&content) {
                    sessions.push(golden);
                }
            }
        }

        sessions.sort_by(|a, b| b.recorded_at.cmp(&a.recorded_at));
        Ok(sessions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_golden_recorder() {
        let temp_dir = TempDir::new().unwrap();
        let recorder = GoldenRecorder::new(temp_dir.path());

        let golden = recorder
            .record(
                "session-123",
                "Test Golden",
                "Create a hello world file",
                true,
                vec!["hello.txt".to_string()],
                "test-model",
                "test-provider",
            )
            .await
            .unwrap();

        assert_eq!(golden.name, "Test Golden");
        assert!(golden.expected_passed);

        // Load it back
        let loaded = recorder.load(&golden.id).await.unwrap();
        assert_eq!(loaded.name, golden.name);
    }

    #[tokio::test]
    async fn test_list_golden_sessions() {
        let temp_dir = TempDir::new().unwrap();
        let recorder = GoldenRecorder::new(temp_dir.path());

        // Record two sessions
        recorder
            .record("s1", "First", "Task 1", true, vec![], "model", "provider")
            .await
            .unwrap();
        recorder
            .record("s2", "Second", "Task 2", true, vec![], "model", "provider")
            .await
            .unwrap();

        let sessions = recorder.list().await.unwrap();
        assert_eq!(sessions.len(), 2);
    }
}
