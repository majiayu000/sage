//! Project-based trajectory storage following Claude Code pattern
//!
//! Storage structure:
//! ```text
//! ~/.sage/projects/{escaped-cwd}/
//! ├── {session-id}.jsonl
//! └── ...
//! ```

use crate::error::{SageError, SageResult};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

/// A single entry in the trajectory JSONL file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryEntry {
    /// Session ID for this trajectory
    pub session_id: Uuid,
    /// Parent entry UUID (for conversation threading)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_uuid: Option<Uuid>,
    /// Entry UUID
    pub uuid: Uuid,
    /// Working directory
    pub cwd: String,
    /// Git branch (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,
    /// Entry type: "user", "assistant", "tool_use", "tool_result"
    #[serde(rename = "type")]
    pub entry_type: String,
    /// Step number (for agent steps)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_number: Option<u32>,
    /// Message content
    pub message: serde_json::Value,
    /// Timestamp in ISO 8601 format
    pub timestamp: String,
    /// Agent state
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    /// Tool calls made
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<serde_json::Value>>,
    /// Tool results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_results: Option<Vec<serde_json::Value>>,
    /// Token usage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsageEntry>,
    /// Model used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Provider used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

/// Token usage entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageEntry {
    pub input_tokens: u64,
    pub output_tokens: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_write_tokens: Option<u64>,
}

/// Project-based trajectory storage
pub struct ProjectStorage {
    /// Session ID
    session_id: Uuid,
    /// Working directory (for path generation)
    working_dir: PathBuf,
    /// Base sage directory (~/.sage)
    sage_dir: PathBuf,
    /// Full path to the JSONL file
    file_path: PathBuf,
    /// Last parent UUID for threading
    last_uuid: Option<Uuid>,
    /// Git branch
    git_branch: Option<String>,
}

impl ProjectStorage {
    /// Create a new project storage for the given working directory
    pub fn new(working_dir: impl AsRef<Path>) -> SageResult<Self> {
        let working_dir = working_dir.as_ref().to_path_buf();
        let session_id = Uuid::new_v4();

        // Get sage directory (~/.sage)
        let sage_dir = dirs::home_dir()
            .ok_or_else(|| SageError::config("Could not determine home directory"))?
            .join(".sage");

        // Generate escaped path for project directory
        let escaped_path = Self::escape_path(&working_dir);
        let project_dir = sage_dir.join("projects").join(&escaped_path);

        // Generate JSONL file path
        let file_path = project_dir.join(format!("{}.jsonl", session_id));

        // Detect git branch
        let git_branch = Self::detect_git_branch(&working_dir);

        Ok(Self {
            session_id,
            working_dir,
            sage_dir,
            file_path,
            last_uuid: None,
            git_branch,
        })
    }

    /// Create storage with a specific session ID (for resuming)
    pub fn with_session_id(working_dir: impl AsRef<Path>, session_id: Uuid) -> SageResult<Self> {
        let mut storage = Self::new(working_dir)?;
        storage.session_id = session_id;

        // Regenerate file path with the given session ID
        let escaped_path = Self::escape_path(&storage.working_dir);
        let project_dir = storage.sage_dir.join("projects").join(&escaped_path);
        storage.file_path = project_dir.join(format!("{}.jsonl", session_id));

        Ok(storage)
    }

    /// Escape a path for use as a directory name (replace / with -)
    fn escape_path(path: &Path) -> String {
        path.to_string_lossy()
            .replace('/', "-")
            .replace('\\', "-")
            .trim_start_matches('-')
            .to_string()
    }

    /// Detect the current git branch
    fn detect_git_branch(working_dir: &Path) -> Option<String> {
        std::process::Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(working_dir)
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
                } else {
                    None
                }
            })
    }

    /// Get the session ID
    pub fn session_id(&self) -> Uuid {
        self.session_id
    }

    /// Get the file path
    pub fn file_path(&self) -> &Path {
        &self.file_path
    }

    /// Initialize the storage (create directories)
    pub async fn init(&self) -> SageResult<()> {
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                SageError::config(format!("Failed to create project directory: {}", e))
            })?;
        }
        Ok(())
    }

    /// Append an entry to the JSONL file
    pub async fn append(&mut self, entry: &TrajectoryEntry) -> SageResult<()> {
        // Ensure directory exists
        self.init().await?;

        // Serialize entry to JSON line
        let json_line = serde_json::to_string(entry)?;

        // Append to file
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)
            .await
            .map_err(|e| SageError::Io(format!("Failed to open trajectory file: {}", e)))?;

        file.write_all(json_line.as_bytes()).await?;
        file.write_all(b"\n").await?;

        // Update last UUID for threading
        self.last_uuid = Some(entry.uuid);

        Ok(())
    }

    /// Record a user message
    pub async fn record_user_message(&mut self, content: &str) -> SageResult<Uuid> {
        let uuid = Uuid::new_v4();
        let entry = TrajectoryEntry {
            session_id: self.session_id,
            parent_uuid: self.last_uuid,
            uuid,
            cwd: self.working_dir.to_string_lossy().to_string(),
            git_branch: self.git_branch.clone(),
            entry_type: "user".to_string(),
            step_number: None,
            message: serde_json::json!({
                "role": "user",
                "content": content
            }),
            timestamp: Utc::now().to_rfc3339(),
            state: None,
            tool_calls: None,
            tool_results: None,
            usage: None,
            model: None,
            provider: None,
        };

        self.append(&entry).await?;
        Ok(uuid)
    }

    /// Record an assistant response
    pub async fn record_assistant_response(
        &mut self,
        content: &str,
        model: Option<String>,
        provider: Option<String>,
        usage: Option<TokenUsageEntry>,
        tool_calls: Option<Vec<serde_json::Value>>,
    ) -> SageResult<Uuid> {
        let uuid = Uuid::new_v4();
        let entry = TrajectoryEntry {
            session_id: self.session_id,
            parent_uuid: self.last_uuid,
            uuid,
            cwd: self.working_dir.to_string_lossy().to_string(),
            git_branch: self.git_branch.clone(),
            entry_type: "assistant".to_string(),
            step_number: None,
            message: serde_json::json!({
                "role": "assistant",
                "content": content
            }),
            timestamp: Utc::now().to_rfc3339(),
            state: None,
            tool_calls,
            tool_results: None,
            usage,
            model,
            provider,
        };

        self.append(&entry).await?;
        Ok(uuid)
    }

    /// Record an agent step
    pub async fn record_step(
        &mut self,
        step_number: u32,
        state: &str,
        content: &str,
        model: Option<String>,
        provider: Option<String>,
        usage: Option<TokenUsageEntry>,
        tool_calls: Option<Vec<serde_json::Value>>,
        tool_results: Option<Vec<serde_json::Value>>,
    ) -> SageResult<Uuid> {
        let uuid = Uuid::new_v4();
        let entry = TrajectoryEntry {
            session_id: self.session_id,
            parent_uuid: self.last_uuid,
            uuid,
            cwd: self.working_dir.to_string_lossy().to_string(),
            git_branch: self.git_branch.clone(),
            entry_type: "step".to_string(),
            step_number: Some(step_number),
            message: serde_json::json!({
                "role": "assistant",
                "content": content
            }),
            timestamp: Utc::now().to_rfc3339(),
            state: Some(state.to_string()),
            tool_calls,
            tool_results,
            usage,
            model,
            provider,
        };

        self.append(&entry).await?;
        Ok(uuid)
    }

    /// Record completion/finalization
    pub async fn record_completion(
        &mut self,
        success: bool,
        final_result: Option<String>,
    ) -> SageResult<Uuid> {
        let uuid = Uuid::new_v4();
        let entry = TrajectoryEntry {
            session_id: self.session_id,
            parent_uuid: self.last_uuid,
            uuid,
            cwd: self.working_dir.to_string_lossy().to_string(),
            git_branch: self.git_branch.clone(),
            entry_type: "completion".to_string(),
            step_number: None,
            message: serde_json::json!({
                "success": success,
                "final_result": final_result
            }),
            timestamp: Utc::now().to_rfc3339(),
            state: Some(if success { "Completed" } else { "Failed" }.to_string()),
            tool_calls: None,
            tool_results: None,
            usage: None,
            model: None,
            provider: None,
        };

        self.append(&entry).await?;
        Ok(uuid)
    }

    /// Load all entries from a JSONL file
    pub async fn load_entries(path: impl AsRef<Path>) -> SageResult<Vec<TrajectoryEntry>> {
        let content = fs::read_to_string(path.as_ref()).await?;
        let mut entries = Vec::new();

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let entry: TrajectoryEntry = serde_json::from_str(line)?;
            entries.push(entry);
        }

        Ok(entries)
    }

    /// List all sessions for a project
    pub async fn list_sessions(working_dir: impl AsRef<Path>) -> SageResult<Vec<Uuid>> {
        let sage_dir = dirs::home_dir()
            .ok_or_else(|| SageError::config("Could not determine home directory"))?
            .join(".sage");

        let escaped_path = Self::escape_path(working_dir.as_ref());
        let project_dir = sage_dir.join("projects").join(&escaped_path);

        if !project_dir.exists() {
            return Ok(Vec::new());
        }

        let mut sessions = Vec::new();
        let mut entries = fs::read_dir(&project_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "jsonl") {
                if let Some(stem) = path.file_stem() {
                    if let Ok(uuid) = Uuid::parse_str(&stem.to_string_lossy()) {
                        sessions.push(uuid);
                    }
                }
            }
        }

        Ok(sessions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_escape_path() {
        let path = Path::new("/Users/test/code/project");
        let escaped = ProjectStorage::escape_path(path);
        assert_eq!(escaped, "Users-test-code-project");
    }

    #[tokio::test]
    async fn test_project_storage() {
        let temp_dir = TempDir::new().unwrap();
        let working_dir = temp_dir.path().to_path_buf();

        // Override home dir for testing
        unsafe {
            std::env::set_var("HOME", temp_dir.path());
        }

        let mut storage = ProjectStorage::new(&working_dir).unwrap();
        storage.init().await.unwrap();

        // Record a user message
        let uuid1 = storage.record_user_message("Hello").await.unwrap();
        assert!(!uuid1.is_nil());

        // Record an assistant response
        let uuid2 = storage
            .record_assistant_response(
                "Hi there!",
                Some("gpt-4".to_string()),
                Some("openai".to_string()),
                None,
                None,
            )
            .await
            .unwrap();
        assert!(!uuid2.is_nil());

        // Verify file exists
        assert!(storage.file_path().exists());

        // Load and verify entries
        let entries = ProjectStorage::load_entries(storage.file_path())
            .await
            .unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].entry_type, "user");
        assert_eq!(entries[1].entry_type, "assistant");
    }
}
