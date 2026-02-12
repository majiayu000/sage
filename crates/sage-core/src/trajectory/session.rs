//! Session-based trajectory recording with JSONL storage
//!
//! Storage structure:
//! ```text
//! ~/.sage/projects/{escaped-cwd}/
//! ├── {session-id}.jsonl
//! └── ...
//! ```
//!
//! Each entry is appended immediately for crash safety.

use crate::error::{SageError, SageResult};
use crate::trajectory::entry::SessionEntry;
use crate::types::TokenUsage;
use chrono::Utc;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

/// Session recorder for JSONL trajectory storage
pub struct SessionRecorder {
    /// Session ID
    session_id: Uuid,
    /// Working directory
    working_dir: PathBuf,
    /// Full path to the JSONL file
    file_path: PathBuf,
    /// Last entry UUID for threading
    last_uuid: Option<Uuid>,
    /// Git branch
    git_branch: Option<String>,
    /// Session start time
    start_time: Instant,
    /// Provider name
    provider: String,
    /// Model name
    model: String,
    /// Step counter
    step_count: u32,
    /// Whether session has started
    started: bool,
}

impl SessionRecorder {
    /// Create a new session recorder for the given working directory
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
            file_path,
            last_uuid: None,
            git_branch,
            start_time: Instant::now(),
            provider: String::new(),
            model: String::new(),
            step_count: 0,
            started: false,
        })
    }

    /// Create recorder with a specific session ID (for resuming)
    pub fn with_session_id(working_dir: impl AsRef<Path>, session_id: Uuid) -> SageResult<Self> {
        let mut recorder = Self::new(working_dir)?;
        recorder.session_id = session_id;

        // Regenerate file path with the given session ID
        let escaped_path = Self::escape_path(&recorder.working_dir);
        let sage_dir = dirs::home_dir()
            .ok_or_else(|| SageError::config("Could not determine home directory"))?
            .join(".sage");
        let project_dir = sage_dir.join("projects").join(&escaped_path);
        recorder.file_path = project_dir.join(format!("{}.jsonl", session_id));

        Ok(recorder)
    }

    /// Escape a path for use as a directory name
    fn escape_path(path: &Path) -> String {
        path.to_string_lossy()
            .replace(['/', '\\'], "-")
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

    /// Ensure directory exists
    async fn ensure_dir(&self) -> SageResult<()> {
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                SageError::config(format!("Failed to create project directory: {}", e))
            })?;
        }
        Ok(())
    }

    /// Append an entry to the JSONL file
    async fn append(&mut self, entry: SessionEntry) -> SageResult<()> {
        self.ensure_dir().await?;

        let json_line = serde_json::to_string(&entry)?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)
            .await
            .map_err(|e| SageError::io(format!("Failed to open session file: {}", e)))?;

        file.write_all(json_line.as_bytes()).await?;
        file.write_all(b"\n").await?;
        file.flush().await?;

        self.last_uuid = Some(entry.uuid());

        Ok(())
    }

    /// Record session start
    pub async fn record_session_start(
        &mut self,
        task: &str,
        provider: &str,
        model: &str,
    ) -> SageResult<()> {
        self.provider = provider.to_string();
        self.model = model.to_string();
        self.start_time = Instant::now();
        self.started = true;

        let entry = SessionEntry::SessionStart {
            session_id: self.session_id,
            task: task.to_string(),
            provider: provider.to_string(),
            model: model.to_string(),
            cwd: self.working_dir.to_string_lossy().to_string(),
            git_branch: self.git_branch.clone(),
            timestamp: Utc::now().to_rfc3339(),
        };

        self.append(entry).await
    }

    /// Record a user message
    pub async fn record_user_message(&mut self, content: serde_json::Value) -> SageResult<Uuid> {
        let uuid = Uuid::new_v4();
        let entry = SessionEntry::User {
            uuid,
            parent_uuid: self.last_uuid,
            content,
            timestamp: Utc::now().to_rfc3339(),
        };

        self.append(entry).await?;
        Ok(uuid)
    }

    /// Record LLM request (before sending)
    pub async fn record_llm_request(
        &mut self,
        messages: Vec<serde_json::Value>,
        tools: Option<Vec<String>>,
    ) -> SageResult<Uuid> {
        let uuid = Uuid::new_v4();
        let entry = SessionEntry::LlmRequest {
            uuid,
            parent_uuid: self.last_uuid,
            messages,
            tools,
            timestamp: Utc::now().to_rfc3339(),
        };

        self.append(entry).await?;
        Ok(uuid)
    }

    /// Record LLM response
    pub async fn record_llm_response(
        &mut self,
        content: &str,
        model: &str,
        usage: Option<TokenUsage>,
        tool_calls: Option<Vec<serde_json::Value>>,
    ) -> SageResult<Uuid> {
        let uuid = Uuid::new_v4();
        self.step_count += 1;

        let entry = SessionEntry::LlmResponse {
            uuid,
            parent_uuid: self.last_uuid,
            content: content.to_string(),
            model: model.to_string(),
            usage,
            tool_calls,
            timestamp: Utc::now().to_rfc3339(),
        };

        self.append(entry).await?;
        Ok(uuid)
    }

    /// Record tool call (before execution)
    pub async fn record_tool_call(
        &mut self,
        tool_name: &str,
        tool_input: serde_json::Value,
    ) -> SageResult<Uuid> {
        let uuid = Uuid::new_v4();
        let entry = SessionEntry::ToolCall {
            uuid,
            parent_uuid: self.last_uuid,
            tool_name: tool_name.to_string(),
            tool_input,
            timestamp: Utc::now().to_rfc3339(),
        };

        self.append(entry).await?;
        Ok(uuid)
    }

    /// Record tool result
    pub async fn record_tool_result(
        &mut self,
        tool_name: &str,
        success: bool,
        output: Option<String>,
        error: Option<String>,
        execution_time_ms: u64,
    ) -> SageResult<Uuid> {
        let uuid = Uuid::new_v4();
        let entry = SessionEntry::ToolResult {
            uuid,
            parent_uuid: self.last_uuid,
            tool_name: tool_name.to_string(),
            success,
            output,
            error,
            execution_time_ms,
            timestamp: Utc::now().to_rfc3339(),
        };

        self.append(entry).await?;
        Ok(uuid)
    }

    /// Record an error
    pub async fn record_error(&mut self, error_type: &str, message: &str) -> SageResult<Uuid> {
        let uuid = Uuid::new_v4();
        let entry = SessionEntry::Error {
            uuid,
            parent_uuid: self.last_uuid,
            error_type: error_type.to_string(),
            message: message.to_string(),
            timestamp: Utc::now().to_rfc3339(),
        };

        self.append(entry).await?;
        Ok(uuid)
    }

    /// Record session end
    pub async fn record_session_end(
        &mut self,
        success: bool,
        final_result: Option<String>,
    ) -> SageResult<Uuid> {
        let uuid = Uuid::new_v4();
        let execution_time = self.start_time.elapsed().as_secs_f64();

        let entry = SessionEntry::SessionEnd {
            uuid,
            parent_uuid: self.last_uuid,
            success,
            final_result,
            total_steps: self.step_count,
            execution_time_secs: execution_time,
            timestamp: Utc::now().to_rfc3339(),
        };

        self.append(entry).await?;
        Ok(uuid)
    }

    /// Check if session has been started
    pub fn is_started(&self) -> bool {
        self.started
    }

    /// Get current step count
    pub fn step_count(&self) -> u32 {
        self.step_count
    }

    /// Load all entries from a session file
    pub async fn load_entries(path: impl AsRef<Path>) -> SageResult<Vec<SessionEntry>> {
        let content = fs::read_to_string(path.as_ref()).await?;
        let mut entries = Vec::new();

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let entry: SessionEntry = serde_json::from_str(line)?;
            entries.push(entry);
        }

        Ok(entries)
    }

    /// List all sessions for a project
    pub async fn list_sessions(working_dir: impl AsRef<Path>) -> SageResult<Vec<SessionInfo>> {
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
            if path.extension().is_some_and(|ext| ext == "jsonl") {
                if let Some(stem) = path.file_stem() {
                    if let Ok(session_id) = Uuid::parse_str(&stem.to_string_lossy()) {
                        // Get file metadata for timestamp
                        if let Ok(metadata) = fs::metadata(&path).await {
                            let modified = metadata
                                .modified()
                                .ok()
                                .and_then(|t| t.elapsed().ok())
                                .map(|d| {
                                    chrono::Utc::now()
                                        - chrono::Duration::from_std(d).unwrap_or_default()
                                });

                            sessions.push(SessionInfo {
                                session_id,
                                file_path: path,
                                modified,
                            });
                        }
                    }
                }
            }
        }

        // Sort by modified time, newest first
        sessions.sort_by(|a, b| b.modified.cmp(&a.modified));

        Ok(sessions)
    }
}

/// Information about a session
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub session_id: Uuid,
    pub file_path: PathBuf,
    pub modified: Option<chrono::DateTime<chrono::Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_session_recorder() {
        let temp_dir = TempDir::new().unwrap();

        // Create a fake home directory
        let fake_home = temp_dir.path().join("home");
        std::fs::create_dir_all(&fake_home).unwrap();
        // SAFETY: This test runs in isolation and does not access HOME concurrently
        unsafe {
            std::env::set_var("HOME", &fake_home);
        }

        let working_dir = temp_dir.path().join("project");
        std::fs::create_dir_all(&working_dir).unwrap();

        let mut recorder = SessionRecorder::new(&working_dir).unwrap();

        // Start session
        recorder
            .record_session_start("Test task", "glm", "glm-4.7")
            .await
            .unwrap();

        // Record user message
        recorder
            .record_user_message(serde_json::json!({"role": "user", "content": "Hello"}))
            .await
            .unwrap();

        // Record LLM response
        recorder
            .record_llm_response(
                "Hi there!",
                "glm-4.7",
                Some(TokenUsage {
                    input_tokens: 10,
                    output_tokens: 5,
                    cache_read_tokens: None,
                    cache_write_tokens: None,
                }),
                None,
            )
            .await
            .unwrap();

        // Record session end
        recorder
            .record_session_end(true, Some("Done".to_string()))
            .await
            .unwrap();

        // Verify file exists
        assert!(recorder.file_path().exists());

        // Load and verify entries
        let entries = SessionRecorder::load_entries(recorder.file_path())
            .await
            .unwrap();

        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].entry_type(), "session_start");
        assert_eq!(entries[1].entry_type(), "user");
        assert_eq!(entries[2].entry_type(), "llm_response");
        assert_eq!(entries[3].entry_type(), "session_end");
    }

    #[test]
    fn test_escape_path() {
        let path = Path::new("/Users/test/code/project");
        let escaped = SessionRecorder::escape_path(path);
        assert_eq!(escaped, "Users-test-code-project");
    }
}
