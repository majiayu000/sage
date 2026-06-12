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
            let result = Self::apply_blocking_policy(&hook_config, result);

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

    fn apply_blocking_policy(
        hook_config: &HookConfig,
        result: HookExecutionResult,
    ) -> HookExecutionResult {
        match result {
            HookExecutionResult::Success(mut output)
                if !output.should_continue && !hook_config.can_block =>
            {
                warn!(
                    "Non-blocking hook requested to block execution; continuing: {}",
                    hook_config
                );
                output.should_continue = true;
                HookExecutionResult::Success(output)
            }
            other => other,
        }
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
        // Prompt hooks require LLM client integration (planned feature)
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
#[path = "hook_executor_tests.rs"]
mod tests;
