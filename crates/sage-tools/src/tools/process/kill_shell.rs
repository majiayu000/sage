//! Kill/terminate background shell processes

use async_trait::async_trait;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Shared registry for tracking background shells
/// In a real implementation, this would be managed by the agent runtime
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
        self.shell_registry.clone().unwrap_or_else(|| SHELL_REGISTRY.clone())
    }

    /// Kill a background shell process by ID
    async fn kill_shell(&self, shell_id: &str) -> Result<ToolResult, ToolError> {
        let registry = self.get_registry();
        let mut shells = registry.lock().await;

        // Check if shell exists
        let pid = shells.remove(shell_id).ok_or_else(|| {
            ToolError::NotFound(format!("Background shell '{}' not found", shell_id))
        })?;

        // Attempt to kill the process
        #[cfg(unix)]
        {
            use nix::sys::signal::{kill, Signal};
            use nix::unistd::Pid;

            let pid_val = Pid::from_raw(pid as i32);

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
                    ToolError::ExecutionFailed(format!(
                        "Failed to execute taskkill: {}",
                        e
                    ))
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
            format!("Successfully terminated background shell '{}' (PID {})", shell_id, pid),
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
        "kill_shell"
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
        result.execution_time_ms = Some(start_time.elapsed().as_millis() as u64);

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

    fn max_execution_time(&self) -> Option<u64> {
        Some(30) // 30 seconds should be enough to kill a process
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

/// Helper function to list all registered shells (for debugging)
#[allow(dead_code)]
pub async fn list_shells() -> Vec<(String, u32)> {
    let shells = SHELL_REGISTRY.lock().await;
    shells.iter().map(|(k, v)| (k.clone(), *v)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_tool_call(id: &str, name: &str, args: serde_json::Value) -> ToolCall {
        let arguments = if let serde_json::Value::Object(map) = args {
            map.into_iter().collect()
        } else {
            HashMap::new()
        };

        ToolCall {
            id: id.to_string(),
            name: name.to_string(),
            arguments,
            call_id: None,
        }
    }

    #[tokio::test]
    async fn test_kill_shell_not_found() {
        let registry = Arc::new(Mutex::new(HashMap::new()));
        let tool = KillShellTool::with_registry(registry);

        let call = create_tool_call(
            "test-1",
            "kill_shell",
            json!({
                "shell_id": "nonexistent_shell"
            }),
        );

        let result = tool.execute(&call).await;
        assert!(result.is_err());

        match result {
            Err(ToolError::NotFound(msg)) => {
                assert!(msg.contains("nonexistent_shell"));
                assert!(msg.contains("not found"));
            }
            _ => panic!("Expected NotFound error"),
        }
    }

    #[tokio::test]
    async fn test_kill_shell_missing_parameter() {
        let tool = KillShellTool::new();
        let call = create_tool_call("test-2", "kill_shell", json!({}));

        let result = tool.execute(&call).await;
        assert!(result.is_err());

        match result {
            Err(ToolError::InvalidArguments(msg)) => {
                assert!(msg.contains("shell_id"));
            }
            _ => panic!("Expected InvalidArguments error"),
        }
    }

    #[tokio::test]
    async fn test_kill_shell_empty_id() {
        let tool = KillShellTool::new();
        let call = create_tool_call(
            "test-3",
            "kill_shell",
            json!({
                "shell_id": ""
            }),
        );

        let result = tool.execute(&call).await;
        assert!(result.is_err());

        match result {
            Err(ToolError::InvalidArguments(msg)) => {
                assert!(msg.contains("empty"));
            }
            _ => panic!("Expected InvalidArguments error"),
        }
    }

    #[tokio::test]
    async fn test_kill_shell_invalid_id_format() {
        let tool = KillShellTool::new();
        let call = create_tool_call(
            "test-4",
            "kill_shell",
            json!({
                "shell_id": "invalid shell!"
            }),
        );

        let result = tool.validate(&call);
        assert!(result.is_err());

        match result {
            Err(ToolError::InvalidArguments(msg)) => {
                assert!(msg.contains("alphanumeric"));
            }
            _ => panic!("Expected InvalidArguments error"),
        }
    }

    #[tokio::test]
    async fn test_kill_shell_validation_success() {
        let tool = KillShellTool::new();

        // Valid shell IDs
        let valid_ids = vec!["shell_1", "background-shell-2", "shell123", "SHELL_ABC"];

        for id in valid_ids {
            let call = create_tool_call(
                "test-5",
                "kill_shell",
                json!({
                    "shell_id": id
                }),
            );

            let result = tool.validate(&call);
            assert!(result.is_ok(), "Failed to validate ID: {}", id);
        }
    }

    #[test]
    fn test_kill_shell_schema() {
        let tool = KillShellTool::new();
        let schema = tool.schema();

        assert_eq!(schema.name, "kill_shell");
        assert!(!schema.description.is_empty());

        // Check that shell_id parameter exists
        let params = schema.parameters;
        assert!(params.get("properties").is_some());
        assert!(params.get("required").is_some());
    }

    #[test]
    fn test_kill_shell_tool_properties() {
        let tool = KillShellTool::new();

        assert_eq!(tool.name(), "kill_shell");
        assert!(!tool.description().is_empty());
        assert_eq!(tool.max_execution_time(), Some(30));
        assert!(tool.supports_parallel_execution());
        assert!(!tool.is_read_only());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_kill_shell_with_mock_process() {
        use std::process::Command;

        // Start a simple sleep process
        let mut child = Command::new("sleep")
            .arg("60")
            .spawn()
            .expect("Failed to spawn sleep process");

        let pid = child.id();

        // Register the shell
        let registry = Arc::new(Mutex::new(HashMap::new()));
        {
            let mut shells = registry.lock().await;
            shells.insert("test_shell".to_string(), pid);
        }

        let tool = KillShellTool::with_registry(registry.clone());

        let call = create_tool_call(
            "test-6",
            "kill_shell",
            json!({
                "shell_id": "test_shell"
            }),
        );

        let result = tool.execute(&call).await;
        assert!(result.is_ok(), "Failed to execute: {:?}", result);

        let tool_result = result.unwrap();
        assert!(tool_result.success);
        assert!(tool_result.output.unwrap().contains("Successfully terminated"));

        // Verify the shell was removed from registry
        let shells = registry.lock().await;
        assert!(!shells.contains_key("test_shell"));

        // Clean up: ensure child is killed
        let _ = child.kill();
        let _ = child.wait();
    }
}
