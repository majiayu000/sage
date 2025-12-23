//! Core Tool trait definition

use super::concurrency::ConcurrencyMode;
use super::error::ToolError;
use crate::tools::permission::{PermissionResult, RiskLevel, ToolContext};
use crate::tools::types::{ToolCall, ToolResult, ToolSchema};
use async_trait::async_trait;
use std::time::{Duration, Instant};

/// Base trait for all tools
///
/// Tools are capabilities that agents can use to interact with the environment.
/// Each tool has a schema for validation, permission checking, and execution logic.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Get the tool's unique name (e.g., "read_file")
    fn name(&self) -> &str;

    /// Get the tool's description for LLM understanding
    fn description(&self) -> &str;

    /// Get the tool's JSON schema for input parameters
    fn schema(&self) -> ToolSchema;

    /// Execute the tool with the given arguments
    ///
    /// # Errors
    ///
    /// Returns `ToolError` if arguments are invalid, execution fails,
    /// resources are unavailable, or permissions are denied.
    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError>;

    /// Validate the tool call arguments
    ///
    /// Called before `execute()` to verify arguments are valid.
    /// Default implementation accepts all arguments.
    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        let _ = call;
        Ok(())
    }

    /// Check if the tool call is permitted in the current context
    ///
    /// Default: allow all operations
    async fn check_permission(&self, _call: &ToolCall, _context: &ToolContext) -> PermissionResult {
        PermissionResult::Allow
    }

    /// Get the risk level for this tool (used for permission checking)
    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Medium
    }

    /// Get the concurrency mode (determines parallel execution)
    fn concurrency_mode(&self) -> ConcurrencyMode {
        ConcurrencyMode::Parallel
    }

    /// Get the maximum execution time as Duration (default: 5 minutes)
    fn max_execution_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs(300))
    }

    /// Get the maximum execution time in seconds (backwards compatibility)
    fn max_execution_time(&self) -> Option<u64> {
        self.max_execution_duration().map(|d| d.as_secs())
    }

    /// Whether this tool only reads data without side effects
    fn is_read_only(&self) -> bool {
        false
    }

    /// Whether this tool can be called in parallel with other tools
    fn supports_parallel_execution(&self) -> bool {
        matches!(
            self.concurrency_mode(),
            ConcurrencyMode::Parallel | ConcurrencyMode::Limited(_)
        )
    }

    /// Render the tool call for display to the user
    ///
    /// Default shows tool name and JSON-formatted arguments.
    fn render_call(&self, call: &ToolCall) -> String {
        format!(
            "{}({})",
            self.name(),
            serde_json::to_string(&call.arguments).unwrap_or_default()
        )
    }

    /// Render the tool result for display to the user
    ///
    /// Default shows output for success, error message for failures.
    fn render_result(&self, result: &ToolResult) -> String {
        if result.success {
            result.output.clone().unwrap_or_default()
        } else {
            format!("Error: {}", result.error.clone().unwrap_or_default())
        }
    }

    /// Whether this tool requires user interaction to complete
    ///
    /// When true, the execution loop blocks and waits for user input
    /// via the InputChannel (e.g., `ask_user_question` tool).
    fn requires_user_interaction(&self) -> bool {
        false
    }

    /// Execute the tool with timing and error handling
    ///
    /// This wraps `execute()` with automatic validation, timing measurement,
    /// and error conversion. Always returns a `ToolResult`.
    ///
    /// Execution flow:
    /// 1. Validates arguments using `validate()`
    /// 2. Executes the tool using `execute()`
    /// 3. Measures execution time
    /// 4. Converts errors to `ToolResult::error`
    async fn execute_with_timing(&self, call: &ToolCall) -> ToolResult {
        let start_time = Instant::now();

        // Validate arguments first
        if let Err(err) = self.validate(call) {
            return ToolResult::error(&call.id, self.name(), err.to_string())
                .with_execution_time(start_time.elapsed().as_millis() as u64);
        }

        // Execute the tool
        match self.execute(call).await {
            Ok(mut result) => {
                result.execution_time_ms = Some(start_time.elapsed().as_millis() as u64);
                result
            }
            Err(err) => ToolResult::error(&call.id, self.name(), err.to_string())
                .with_execution_time(start_time.elapsed().as_millis() as u64),
        }
    }
}
