//! Hook execution engine
//!
//! Executes hooks registered with the hooks system. Supports command hooks,
//! prompt hooks, and callback hooks with timeout and cancellation support.

use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

use super::events::HookEvent;
use super::registry::HookRegistry;
use super::types::{
    CommandHook, HookConfig, HookImplementation, HookInput, HookOutput, PromptHook,
};
use crate::error::SageResult;

/// Hook execution result
#[derive(Debug, Clone)]
pub enum HookExecutionResult {
    /// Hook executed successfully
    Success(HookOutput),

    /// Hook execution failed
    Error(String),

    /// Hook execution timed out
    Timeout,

    /// Hook execution was cancelled
    Cancelled,
}

impl HookExecutionResult {
    /// Check if the hook result allows continuing execution
    pub fn should_continue(&self) -> bool {
        match self {
            HookExecutionResult::Success(output) => output.should_continue,
            HookExecutionResult::Error(_) => true, // Non-blocking hooks continue on error
            HookExecutionResult::Timeout => true,
            HookExecutionResult::Cancelled => false,
        }
    }

    /// Get the message from the result, if any
    pub fn message(&self) -> Option<&str> {
        match self {
            HookExecutionResult::Success(output) => output.reason.as_deref(),
            HookExecutionResult::Error(msg) => Some(msg),
            HookExecutionResult::Timeout => Some("Hook execution timed out"),
            HookExecutionResult::Cancelled => Some("Hook execution was cancelled"),
        }
    }
}

/// Hook executor
pub struct HookExecutor {
    registry: HookRegistry,
    #[allow(dead_code)]
    default_timeout: Duration,
}

impl HookExecutor {
    /// Create a new hook executor with the given registry
    pub fn new(registry: HookRegistry) -> Self {
        Self {
            registry,
            default_timeout: Duration::from_secs(60),
        }
    }

    /// Create a new hook executor with a custom timeout
    pub fn with_timeout(registry: HookRegistry, timeout: Duration) -> Self {
        Self {
            registry,
            default_timeout: timeout,
        }
    }

    /// Execute all matching hooks for an event with a query value
    pub async fn execute(
        &self,
        event: HookEvent,
        query: &str,
        input: HookInput,
        cancel: CancellationToken,
    ) -> SageResult<Vec<HookExecutionResult>> {
        let hooks = self.registry.get_matching(event, query);

        if hooks.is_empty() {
            debug!(
                "No hooks registered for event: {} with query: {}",
                event, query
            );
            return Ok(Vec::new());
        }

        info!(
            "Executing {} hook(s) for event: {} (query: {})",
            hooks.len(),
            event.description(),
            query
        );

        let mut results = Vec::new();

        for hook_config in hooks {
            if cancel.is_cancelled() {
                warn!("Hook execution cancelled before hook");
                results.push(HookExecutionResult::Cancelled);
                break;
            }

            debug!("Executing hook: {}", hook_config);
            let result = self.execute_hook(&hook_config, &input, &cancel).await;

            // Log the result
            match &result {
                HookExecutionResult::Success(output) if !output.should_continue => {
                    warn!("Hook blocked execution: {}", hook_config);
                }
                HookExecutionResult::Success(_) => debug!("Hook succeeded: {}", hook_config),
                HookExecutionResult::Error(msg) => error!("Hook failed: {}: {}", hook_config, msg),
                HookExecutionResult::Timeout => warn!("Hook timed out: {}", hook_config),
                HookExecutionResult::Cancelled => warn!("Hook was cancelled: {}", hook_config),
            }

            // Check if we should stop execution
            if !result.should_continue() {
                info!("Hook blocked execution: {}", hook_config);
                results.push(result);
                break;
            }

            results.push(result);
        }

        Ok(results)
    }

    /// Execute a single hook configuration
    async fn execute_hook(
        &self,
        hook_config: &HookConfig,
        input: &HookInput,
        cancel: &CancellationToken,
    ) -> HookExecutionResult {
        match &hook_config.implementation {
            HookImplementation::Command(cmd) => self.execute_command(cmd, input, cancel).await,
            HookImplementation::Prompt(prompt) => self.execute_prompt(prompt, input, cancel).await,
        }
    }

    /// Execute a command hook
    async fn execute_command(
        &self,
        hook: &CommandHook,
        input: &HookInput,
        cancel: &CancellationToken,
    ) -> HookExecutionResult {
        let timeout_duration = hook.timeout();

        match timeout(timeout_duration, self.run_command(hook, input, cancel)).await {
            Ok(result) => result,
            Err(_) => {
                warn!("Command hook timed out after {:?}", timeout_duration);
                HookExecutionResult::Timeout
            }
        }
    }

    /// Run the actual command
    async fn run_command(
        &self,
        hook: &CommandHook,
        input: &HookInput,
        cancel: &CancellationToken,
    ) -> HookExecutionResult {
        // Serialize input to JSON
        let input_json = match serde_json::to_string(input) {
            Ok(json) => json,
            Err(e) => {
                return HookExecutionResult::Error(format!(
                    "Failed to serialize hook input: {}",
                    e
                ));
            }
        };

        // Build the command
        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = Command::new("cmd");
            c.args(["/C", &hook.command]);
            c
        } else {
            let mut c = Command::new("sh");
            c.args(["-c", &hook.command]);
            c
        };

        // Set environment variable with hook input
        cmd.env("CLAUDE_HOOK_INPUT", input_json);

        // Configure stdio
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Spawn the command
        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) => {
                return HookExecutionResult::Error(format!("Failed to spawn command: {}", e));
            }
        };

        // Take stdout and stderr before using them in futures
        let stdout_handle = child.stdout.take();
        let stderr_handle = child.stderr.take();

        // Read output asynchronously
        let stdout_future = async move {
            if let Some(mut handle) = stdout_handle {
                let mut output = String::new();
                handle.read_to_string(&mut output).await.ok();
                output
            } else {
                String::new()
            }
        };

        let stderr_future = async move {
            if let Some(mut handle) = stderr_handle {
                let mut output = String::new();
                handle.read_to_string(&mut output).await.ok();
                output
            } else {
                String::new()
            }
        };

        // Wait for completion or cancellation
        tokio::select! {
            _ = cancel.cancelled() => {
                // Kill the process
                let _ = child.kill().await;
                HookExecutionResult::Cancelled
            }
            wait_result = child.wait() => {
                let (stdout, stderr) = tokio::join!(stdout_future, stderr_future);

                match wait_result {
                    Ok(status) => {
                        if status.success() {
                            let output = Self::parse_output(&stdout);
                            HookExecutionResult::Success(output)
                        } else {
                            let error_msg = if stderr.is_empty() {
                                format!("Command failed with exit code: {:?}", status.code())
                            } else {
                                stderr
                            };
                            HookExecutionResult::Error(error_msg)
                        }
                    }
                    Err(e) => {
                        HookExecutionResult::Error(format!("Failed to wait for command: {}", e))
                    }
                }
            }
        }
    }

    /// Execute a prompt hook (placeholder - needs LLM client integration)
    async fn execute_prompt(
        &self,
        _hook: &PromptHook,
        _input: &HookInput,
        _cancel: &CancellationToken,
    ) -> HookExecutionResult {
        // TODO: Integrate with LLM client when available
        warn!("Prompt hooks are not yet implemented");
        HookExecutionResult::Error("Prompt hooks are not yet implemented".to_string())
    }

    /// Parse hook output from stdout
    fn parse_output(stdout: &str) -> HookOutput {
        let trimmed = stdout.trim();

        // Try to parse as JSON if it starts with '{'
        if trimmed.starts_with('{') {
            if let Ok(output) = serde_json::from_str::<HookOutput>(trimmed) {
                return output;
            }
        }

        // Otherwise, return default allow with the output as a message
        if trimmed.is_empty() {
            HookOutput::allow()
        } else {
            HookOutput::allow().with_reason(trimmed.to_string())
        }
    }
}

impl Default for HookExecutor {
    fn default() -> Self {
        Self::new(HookRegistry::new())
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::HookType;
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_parse_output_empty() {
        let output = HookExecutor::parse_output("");
        assert!(output.should_continue);
        assert_eq!(output.reason, None);
    }

    #[test]
    fn test_parse_output_plain_text() {
        let output = HookExecutor::parse_output("Hello, world!");
        assert!(output.should_continue);
        assert_eq!(output.reason.as_deref(), Some("Hello, world!"));
    }

    #[test]
    fn test_parse_output_json() {
        let json = r#"{"should_continue": false, "reason": "Blocked"}"#;
        let output = HookExecutor::parse_output(json);
        assert!(!output.should_continue);
        assert_eq!(output.reason.as_deref(), Some("Blocked"));
    }

    #[test]
    fn test_parse_output_json_with_data() {
        let json = r#"{
            "should_continue": true,
            "reason": "Success",
            "additional_context": ["context1"]
        }"#;
        let output = HookExecutor::parse_output(json);
        assert!(output.should_continue);
        assert_eq!(output.reason.as_deref(), Some("Success"));
        assert_eq!(output.additional_context.len(), 1);
    }

    fn create_test_hook_config(command: &str) -> HookConfig {
        HookConfig {
            name: "test_hook".to_string(),
            hook_type: HookType::PreToolExecution,
            implementation: HookImplementation::Command(CommandHook {
                command: command.to_string(),
                timeout_secs: 60,
                status_message: None,
                working_dir: None,
                env: HashMap::new(),
            }),
            can_block: false,
            timeout_secs: 60,
            enabled: true,
        }
    }

    #[tokio::test]
    async fn test_execute_command_success() {
        let registry = HookRegistry::new();
        let hook_config = create_test_hook_config("echo test");

        let executor = HookExecutor::new(registry);
        let input = HookInput::new(HookEvent::PreToolUse, "test-session");
        let cancel = CancellationToken::new();

        let cmd = match &hook_config.implementation {
            HookImplementation::Command(cmd) => cmd,
            _ => panic!("Expected command hook"),
        };

        let result = executor.execute_command(cmd, &input, &cancel).await;

        match result {
            HookExecutionResult::Success(output) => {
                assert!(output.should_continue);
                assert_eq!(output.reason.as_deref(), Some("test"));
            }
            _ => panic!("Expected success result, got: {:?}", result),
        }
    }

    #[tokio::test]
    async fn test_execute_command_failure() {
        let registry = HookRegistry::new();
        let hook_config = create_test_hook_config("exit 1");

        let executor = HookExecutor::new(registry);
        let input = HookInput::new(HookEvent::PreToolUse, "test-session");
        let cancel = CancellationToken::new();

        let cmd = match &hook_config.implementation {
            HookImplementation::Command(cmd) => cmd,
            _ => panic!("Expected command hook"),
        };

        let result = executor.execute_command(cmd, &input, &cancel).await;

        match result {
            HookExecutionResult::Error(_) => {
                // Expected
            }
            _ => panic!("Expected error result, got: {:?}", result),
        }
    }

    #[test]
    fn test_hook_result_should_continue() {
        let success = HookExecutionResult::Success(HookOutput::allow());
        assert!(success.should_continue());

        let blocked = HookExecutionResult::Success(HookOutput::deny("Not allowed"));
        assert!(!blocked.should_continue());

        let error = HookExecutionResult::Error("Failed".to_string());
        assert!(error.should_continue()); // Non-blocking errors continue

        let timeout = HookExecutionResult::Timeout;
        assert!(timeout.should_continue());

        let cancelled = HookExecutionResult::Cancelled;
        assert!(!cancelled.should_continue());
    }

    #[test]
    fn test_hook_result_message() {
        let success = HookExecutionResult::Success(HookOutput::deny("Custom message"));
        assert_eq!(success.message(), Some("Custom message"));

        let error = HookExecutionResult::Error("Failed".to_string());
        assert_eq!(error.message(), Some("Failed"));

        let timeout = HookExecutionResult::Timeout;
        assert!(timeout.message().is_some());
    }
}
