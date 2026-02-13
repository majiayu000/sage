//! Kill/terminate background shell processes

use async_trait::async_trait;
use sage_core::tools::BACKGROUND_REGISTRY;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Legacy shell registry for backwards compatibility
/// New code should use BACKGROUND_REGISTRY from sage-core
static SHELL_REGISTRY: once_cell::sync::Lazy<Arc<Mutex<HashMap<String, u32>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

/// Tool for killing/terminating background shell processes
pub struct KillShellTool {
    /// Optional shell registry override for testing
    shell_registry: Option<Arc<Mutex<HashMap<String, u32>>>>,
}

impl KillShellTool {
    /// Create a new KillShell tool
    pub fn new() -> Self {
        Self {
            shell_registry: None,
        }
    }

    /// Create a KillShell tool with custom registry (for testing)
    #[cfg(test)]
    pub fn with_registry(registry: Arc<Mutex<HashMap<String, u32>>>) -> Self {
        Self {
            shell_registry: Some(registry),
        }
    }

    /// Get the shell registry
    fn get_registry(&self) -> Arc<Mutex<HashMap<String, u32>>> {
        self.shell_registry
            .clone()
            .unwrap_or_else(|| SHELL_REGISTRY.clone())
    }

    /// Kill a background shell process by ID
    async fn kill_shell(&self, shell_id: &str) -> Result<ToolResult, ToolError> {
        // First, check the new BACKGROUND_REGISTRY
        if let Some(task) = BACKGROUND_REGISTRY.get(shell_id) {
            let pid = task.pid;

            // Kill the task
            task.kill().await.map_err(|e| {
                ToolError::ExecutionFailed(format!("Failed to kill shell {}: {}", shell_id, e))
            })?;

            // Remove from registry
            BACKGROUND_REGISTRY.remove(shell_id);

            return Ok(ToolResult::success(
                "",
                self.name(),
                format!(
                    "Successfully terminated background shell '{}' (PID {:?})",
                    shell_id, pid
                ),
            ));
        }

        // Fall back to legacy registry for backwards compatibility
        let registry = self.get_registry();
        let mut shells = registry.lock().await;

        // Check if shell exists in legacy registry
        let pid = shells.remove(shell_id).ok_or_else(|| {
            ToolError::NotFound(format!("Background shell '{}' not found", shell_id))
        })?;

        // Attempt to kill the process
        #[cfg(unix)]
        {
            use nix::sys::signal::{Signal, kill};
            use nix::unistd::Pid;

            let pid_i32 = i32::try_from(pid).map_err(|_| {
                ToolError::ExecutionFailed(format!(
                    "PID {} exceeds i32 range for shell {}",
                    pid, shell_id
                ))
            })?;
            let pid_val = Pid::from_raw(pid_i32);

            // Try graceful termination first (SIGTERM)
            match kill(pid_val, Signal::SIGTERM) {
                Ok(_) => {
                    // Wait a bit for graceful shutdown
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                    // Check if process is still running
                    if kill(pid_val, None).is_ok() {
                        // Process still running, force kill (SIGKILL)
                        if let Err(e) = kill(pid_val, Signal::SIGKILL) {
                            return Err(ToolError::ExecutionFailed(format!(
                                "Failed to force kill shell {}: {}",
                                shell_id, e
                            )));
                        }
                    }
                }
                Err(nix::errno::Errno::ESRCH) => {
                    // Process already dead
                    return Ok(ToolResult::success(
                        "",
                        self.name(),
                        format!("Shell '{}' (PID {}) was already terminated", shell_id, pid),
                    ));
                }
                Err(e) => {
                    return Err(ToolError::ExecutionFailed(format!(
                        "Failed to terminate shell {}: {}",
                        shell_id, e
                    )));
                }
            }
        }

        #[cfg(windows)]
        {
            use std::process::Command;

            let output = Command::new("taskkill")
                .args(&["/PID", &pid.to_string(), "/F"])
                .output()
                .map_err(|e| {
                    ToolError::ExecutionFailed(format!("Failed to execute taskkill: {}", e))
                })?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(ToolError::ExecutionFailed(format!(
                    "Failed to kill shell {}: {}",
                    shell_id, stderr
                )));
            }
        }

        Ok(ToolResult::success(
            "",
            self.name(),
            format!(
                "Successfully terminated background shell '{}' (PID {})",
                shell_id, pid
            ),
        ))
    }
}

impl Default for KillShellTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for KillShellTool {
    fn name(&self) -> &str {
        "KillShell"
    }

    fn description(&self) -> &str {
        "Kill/terminate a running background shell process by its ID.

Use this tool to stop background shell processes that were started and are no longer needed.
The tool will attempt graceful termination (SIGTERM) first, and force kill (SIGKILL) if needed.

Parameters:
- shell_id: The unique identifier of the background shell to terminate

Example: kill_shell(shell_id=\"shell_1\")"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![ToolParameter::string(
                "shell_id",
                "The unique identifier of the background shell to kill",
            )],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let shell_id = call.get_string("shell_id").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'shell_id' parameter".to_string())
        })?;

        if shell_id.trim().is_empty() {
            return Err(ToolError::InvalidArguments(
                "shell_id cannot be empty".to_string(),
            ));
        }

        let start_time = std::time::Instant::now();
        let mut result = self.kill_shell(&shell_id).await?;

        result.call_id = call.id.clone();
        result.execution_time_ms =
            Some(u64::try_from(start_time.elapsed().as_millis()).unwrap_or(u64::MAX));

        Ok(result)
    }

    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        let shell_id = call.get_string("shell_id").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'shell_id' parameter".to_string())
        })?;

        if shell_id.trim().is_empty() {
            return Err(ToolError::InvalidArguments(
                "shell_id cannot be empty".to_string(),
            ));
        }

        // Validate shell_id format (alphanumeric, underscores, hyphens only)
        if !shell_id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(ToolError::InvalidArguments(
                "shell_id must contain only alphanumeric characters, underscores, and hyphens"
                    .to_string(),
            ));
        }

        Ok(())
    }

    fn max_execution_duration(&self) -> Option<std::time::Duration> {
        Some(std::time::Duration::from_secs(30)) // 30 seconds should be enough to kill a process
    }

    fn supports_parallel_execution(&self) -> bool {
        true // Can kill multiple shells in parallel
    }

    fn is_read_only(&self) -> bool {
        false // Modifies system state
    }
}

/// Helper function to register a shell (for testing and integration)
pub async fn register_shell(shell_id: String, pid: u32) {
    let mut shells = SHELL_REGISTRY.lock().await;
    shells.insert(shell_id, pid);
}

/// Helper function to unregister a shell (for testing and integration)
pub async fn unregister_shell(shell_id: &str) -> Option<u32> {
    let mut shells = SHELL_REGISTRY.lock().await;
    shells.remove(shell_id)
}

#[cfg(test)]
#[path = "kill_shell_tests.rs"]
mod tests;
