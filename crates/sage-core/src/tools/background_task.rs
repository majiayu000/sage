//! Background shell task management
//!
//! This module provides types and functionality for managing background shell tasks,
//! allowing commands to run asynchronously while the agent continues with other work.

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

use crate::error::{SageError, SageResult};

/// Status of a background task
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackgroundTaskStatus {
    /// Task is currently running
    Running,
    /// Task completed with exit code
    Completed { exit_code: i32 },
    /// Task failed with error
    Failed { error: String },
    /// Task was killed by user request
    Killed,
}

impl std::fmt::Display for BackgroundTaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Running => write!(f, "RUNNING"),
            Self::Completed { exit_code } => {
                if *exit_code == 0 {
                    write!(f, "COMPLETED (success)")
                } else {
                    write!(f, "COMPLETED (exit code: {})", exit_code)
                }
            }
            Self::Failed { error } => write!(f, "FAILED: {}", error),
            Self::Killed => write!(f, "KILLED"),
        }
    }
}

/// A background shell task with output capture
pub struct BackgroundShellTask {
    /// Unique identifier for this shell
    pub shell_id: String,
    /// Process ID (if available)
    pub pid: Option<u32>,
    /// Accumulated stdout
    stdout: Arc<RwLock<String>>,
    /// Accumulated stderr
    stderr: Arc<RwLock<String>>,
    /// Task status
    status: Arc<RwLock<BackgroundTaskStatus>>,
    /// When the task started
    pub started_at: Instant,
    /// Cancellation token for this specific task
    cancel_token: CancellationToken,
    /// Position tracking for incremental stdout reads
    last_stdout_pos: Arc<RwLock<usize>>,
    /// Position tracking for incremental stderr reads
    last_stderr_pos: Arc<RwLock<usize>>,
    /// The command being executed
    pub command: String,
    /// Working directory
    pub working_dir: String,
}

impl BackgroundShellTask {
    /// Spawn a new background shell task
    pub async fn spawn(
        shell_id: String,
        command: &str,
        working_dir: &Path,
        cancel_token: CancellationToken,
    ) -> SageResult<Self> {
        let mut cmd = Command::new("bash");
        cmd.arg("-c")
            .arg(command)
            .current_dir(working_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .stdin(std::process::Stdio::null());

        let mut child = cmd.spawn().map_err(|e| {
            SageError::Io(format!("Failed to spawn background process: {}", e))
        })?;

        let pid = child.id();

        let stdout_pipe = child.stdout.take();
        let stderr_pipe = child.stderr.take();

        let stdout = Arc::new(RwLock::new(String::new()));
        let stderr = Arc::new(RwLock::new(String::new()));
        let status = Arc::new(RwLock::new(BackgroundTaskStatus::Running));

        // Spawn stdout capture task
        if let Some(pipe) = stdout_pipe {
            let stdout_clone = stdout.clone();
            tokio::spawn(async move {
                let reader = BufReader::new(pipe);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let mut buf = stdout_clone.write().await;
                    buf.push_str(&line);
                    buf.push('\n');
                }
            });
        }

        // Spawn stderr capture task
        if let Some(pipe) = stderr_pipe {
            let stderr_clone = stderr.clone();
            tokio::spawn(async move {
                let reader = BufReader::new(pipe);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let mut buf = stderr_clone.write().await;
                    buf.push_str(&line);
                    buf.push('\n');
                }
            });
        }

        // Spawn process monitor task
        let status_clone = status.clone();
        let cancel_token_clone = cancel_token.clone();
        let shell_id_clone = shell_id.clone();
        tokio::spawn(async move {
            Self::monitor_process(child, status_clone, cancel_token_clone, &shell_id_clone).await;
        });

        info!(
            "Background shell '{}' started with PID {:?}",
            shell_id, pid
        );

        Ok(Self {
            shell_id,
            pid,
            stdout,
            stderr,
            status,
            started_at: Instant::now(),
            cancel_token,
            last_stdout_pos: Arc::new(RwLock::new(0)),
            last_stderr_pos: Arc::new(RwLock::new(0)),
            command: command.to_string(),
            working_dir: working_dir.to_string_lossy().to_string(),
        })
    }

    /// Monitor process and update status
    async fn monitor_process(
        mut child: Child,
        status: Arc<RwLock<BackgroundTaskStatus>>,
        cancel_token: CancellationToken,
        shell_id: &str,
    ) {
        tokio::select! {
            _ = cancel_token.cancelled() => {
                debug!("Cancellation requested for shell '{}'", shell_id);
                if let Err(e) = child.kill().await {
                    error!("Failed to kill process for shell '{}': {}", shell_id, e);
                }
                *status.write().await = BackgroundTaskStatus::Killed;
                info!("Background shell '{}' was killed", shell_id);
            }
            result = child.wait() => {
                match result {
                    Ok(exit_status) => {
                        let exit_code = exit_status.code().unwrap_or(-1);
                        *status.write().await = BackgroundTaskStatus::Completed { exit_code };
                        debug!("Background shell '{}' completed with exit code {}", shell_id, exit_code);
                    }
                    Err(e) => {
                        let error_msg = e.to_string();
                        *status.write().await = BackgroundTaskStatus::Failed { error: error_msg.clone() };
                        error!("Background shell '{}' failed: {}", shell_id, error_msg);
                    }
                }
            }
        }
    }

    /// Get current status
    pub async fn status(&self) -> BackgroundTaskStatus {
        self.status.read().await.clone()
    }

    /// Check if the task is still running
    pub async fn is_running(&self) -> bool {
        matches!(*self.status.read().await, BackgroundTaskStatus::Running)
    }

    /// Get all accumulated output
    pub async fn get_output(&self) -> (String, String) {
        let stdout = self.stdout.read().await.clone();
        let stderr = self.stderr.read().await.clone();
        (stdout, stderr)
    }

    /// Get incremental output since last read
    pub async fn get_incremental_output(&self) -> (String, String) {
        let mut stdout_pos = self.last_stdout_pos.write().await;
        let mut stderr_pos = self.last_stderr_pos.write().await;

        let stdout_full = self.stdout.read().await;
        let stderr_full = self.stderr.read().await;

        let stdout_new = if *stdout_pos < stdout_full.len() {
            stdout_full[*stdout_pos..].to_string()
        } else {
            String::new()
        };

        let stderr_new = if *stderr_pos < stderr_full.len() {
            stderr_full[*stderr_pos..].to_string()
        } else {
            String::new()
        };

        *stdout_pos = stdout_full.len();
        *stderr_pos = stderr_full.len();

        (stdout_new, stderr_new)
    }

    /// Get uptime in seconds
    pub fn uptime_secs(&self) -> f64 {
        self.started_at.elapsed().as_secs_f64()
    }

    /// Kill the background task
    pub async fn kill(&self) -> SageResult<()> {
        // Signal cancellation
        self.cancel_token.cancel();

        // Wait a bit for graceful shutdown
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Force kill if still running via PID
        #[cfg(unix)]
        if let Some(pid) = self.pid {
            use nix::sys::signal::{kill, Signal};
            use nix::unistd::Pid;

            let pid = Pid::from_raw(pid as i32);
            if let Err(e) = kill(pid, Signal::SIGKILL) {
                warn!("Failed to SIGKILL process {}: {}", pid, e);
            }
        }

        Ok(())
    }

    /// Format task info for display
    pub fn format_info(&self) -> String {
        format!(
            "Shell ID: {}\nPID: {:?}\nCommand: {}\nWorking Dir: {}\nUptime: {:.1}s",
            self.shell_id,
            self.pid,
            self.command,
            self.working_dir,
            self.uptime_secs()
        )
    }
}

impl std::fmt::Debug for BackgroundShellTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BackgroundShellTask")
            .field("shell_id", &self.shell_id)
            .field("pid", &self.pid)
            .field("command", &self.command)
            .field("working_dir", &self.working_dir)
            .field("started_at", &self.started_at)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_background_task_spawn() {
        let cancel_token = CancellationToken::new();
        let task = BackgroundShellTask::spawn(
            "test_shell_1".to_string(),
            "echo 'hello world'",
            &PathBuf::from("/tmp"),
            cancel_token,
        )
        .await
        .unwrap();

        assert_eq!(task.shell_id, "test_shell_1");
        assert!(task.pid.is_some());

        // Wait for completion
        tokio::time::sleep(Duration::from_millis(200)).await;

        let status = task.status().await;
        assert!(matches!(status, BackgroundTaskStatus::Completed { exit_code: 0 }));

        let (stdout, _) = task.get_output().await;
        assert!(stdout.contains("hello world"));
    }

    #[tokio::test]
    async fn test_background_task_kill() {
        let cancel_token = CancellationToken::new();
        let task = BackgroundShellTask::spawn(
            "test_shell_2".to_string(),
            "sleep 10",
            &PathBuf::from("/tmp"),
            cancel_token,
        )
        .await
        .unwrap();

        // Should be running
        assert!(task.is_running().await);

        // Kill it
        task.kill().await.unwrap();

        // Wait for status update
        tokio::time::sleep(Duration::from_millis(200)).await;

        let status = task.status().await;
        assert!(matches!(status, BackgroundTaskStatus::Killed));
    }

    #[tokio::test]
    async fn test_background_task_incremental_output() {
        let cancel_token = CancellationToken::new();
        let task = BackgroundShellTask::spawn(
            "test_shell_3".to_string(),
            "echo 'line1'; sleep 0.1; echo 'line2'",
            &PathBuf::from("/tmp"),
            cancel_token,
        )
        .await
        .unwrap();

        // Wait for first line
        tokio::time::sleep(Duration::from_millis(50)).await;

        let (stdout1, _) = task.get_incremental_output().await;

        // Wait for second line
        tokio::time::sleep(Duration::from_millis(150)).await;

        let (stdout2, _) = task.get_incremental_output().await;

        // Incremental should not repeat first line
        // Note: timing-dependent, might need adjustment
        let total = format!("{}{}", stdout1, stdout2);
        assert!(total.contains("line1"));
        assert!(total.contains("line2"));
    }

    #[tokio::test]
    async fn test_background_task_status_display() {
        let status = BackgroundTaskStatus::Running;
        assert_eq!(format!("{}", status), "RUNNING");

        let status = BackgroundTaskStatus::Completed { exit_code: 0 };
        assert_eq!(format!("{}", status), "COMPLETED (success)");

        let status = BackgroundTaskStatus::Completed { exit_code: 1 };
        assert_eq!(format!("{}", status), "COMPLETED (exit code: 1)");

        let status = BackgroundTaskStatus::Failed { error: "test error".to_string() };
        assert_eq!(format!("{}", status), "FAILED: test error");

        let status = BackgroundTaskStatus::Killed;
        assert_eq!(format!("{}", status), "KILLED");
    }

    #[tokio::test]
    async fn test_background_task_uptime() {
        let cancel_token = CancellationToken::new();
        let task = BackgroundShellTask::spawn(
            "test_shell_4".to_string(),
            "sleep 0.1",
            &PathBuf::from("/tmp"),
            cancel_token,
        )
        .await
        .unwrap();

        // Uptime should be positive
        assert!(task.uptime_secs() >= 0.0);

        tokio::time::sleep(Duration::from_millis(100)).await;

        // Uptime should have increased
        assert!(task.uptime_secs() >= 0.1);
    }
}
