//! Core Tool trait definitions
//!
//! This module provides a modular trait hierarchy for building tools:
//!
//! - [`Tool`] - Core trait with name, description, schema, and execute (required)
//! - [`ToolValidator`] - Optional validation before execution
//! - [`ToolPermission`] - Optional permission checking and risk levels
//! - [`ToolConcurrency`] - Optional concurrency configuration
//! - [`ToolTiming`] - Optional execution timing limits
//! - [`ToolRenderer`] - Optional rendering for display
//! - [`ToolMetadata`] - Optional metadata flags
//!
//! Tools implement the core `Tool` trait and optionally implement additional
//! traits for extended functionality. Default implementations are provided
//! where sensible.

use super::concurrency::ConcurrencyMode;
use super::error::ToolError;
use crate::tools::permission::{RiskLevel, ToolContext, ToolPermissionResult};
use crate::tools::types::{ToolCall, ToolResult, ToolSchema};
use async_trait::async_trait;
use std::time::{Duration, Instant};

// ============================================================================
// Core Tool Trait (Required)
// ============================================================================

/// Base trait for all tools - defines core functionality
///
/// Tools are capabilities that agents can use to interact with the environment.
/// Each tool has a schema for validation, permission checking, and execution logic.
///
/// # Example
///
/// ```no_run
/// use sage_core::tools::{Tool, ToolSchema};
/// use sage_core::tools::base::ToolError;
/// use sage_core::tools::types::{ToolCall, ToolResult};
/// use async_trait::async_trait;
///
/// struct MyTool;
///
/// #[async_trait]
/// impl Tool for MyTool {
///     fn name(&self) -> &str { "my_tool" }
///     fn description(&self) -> &str { "A custom tool" }
///     fn schema(&self) -> ToolSchema {
///         ToolSchema::new(self.name(), self.description(), vec![])
///     }
///     async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
///         Ok(ToolResult::success(&call.id, self.name(), "done"))
///     }
/// }
/// ```
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

    // ========================================================================
    // Validation (from ToolValidator)
    // ========================================================================

    /// Validate the tool call arguments
    ///
    /// Called before `execute()` to verify arguments are valid.
    /// Default implementation accepts all arguments.
    fn validate(&self, _call: &ToolCall) -> Result<(), ToolError> {
        Ok(())
    }

    // ========================================================================
    // Permission (from ToolPermission)
    // ========================================================================

    /// Get the risk level for this tool (used for permission checking)
    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Medium
    }

    /// Check if the tool call is permitted in the current context
    ///
    /// Default: allow all operations
    async fn check_permission(
        &self,
        _call: &ToolCall,
        _context: &ToolContext,
    ) -> ToolPermissionResult {
        ToolPermissionResult::Allow
    }

    // ========================================================================
    // Concurrency (from ToolConcurrency)
    // ========================================================================

    /// Get the concurrency mode (determines parallel execution)
    fn concurrency_mode(&self) -> ConcurrencyMode {
        ConcurrencyMode::Parallel
    }

    /// Whether this tool can be called in parallel with other tools
    fn supports_parallel_execution(&self) -> bool {
        matches!(
            self.concurrency_mode(),
            ConcurrencyMode::Parallel | ConcurrencyMode::Limited(_)
        )
    }

    // ========================================================================
    // Timing (from ToolTiming)
    // ========================================================================

    /// Get the maximum execution time as Duration (default: 5 minutes)
    fn max_execution_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs(300))
    }

    // ========================================================================
    // Rendering (from ToolRenderer)
    // ========================================================================

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

    // ========================================================================
    // Metadata (from ToolMetadata)
    // ========================================================================

    /// Whether this tool only reads data without side effects
    fn is_read_only(&self) -> bool {
        false
    }

    /// Whether this tool requires user interaction to complete
    ///
    /// When true, the execution loop blocks and waits for user input
    /// via the InputChannel (e.g., `ask_user_question` tool).
    fn requires_user_interaction(&self) -> bool {
        false
    }

    // ========================================================================
    // Execution Helper (from ToolExt)
    // ========================================================================

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

// ============================================================================
// Focused Extension Traits (for specialized implementations)
// ============================================================================

/// Optional trait for tool input validation
///
/// This trait exists for documentation and future specialization purposes.
/// The default implementation is provided in the `Tool` trait itself.
/// Implement this trait alongside `Tool` if you want to clearly separate
/// validation logic in your code organization.
pub trait ToolValidator: Tool {
    /// Validate the tool call arguments (mirrors Tool::validate)
    fn validate_call(&self, call: &ToolCall) -> Result<(), ToolError> {
        self.validate(call)
    }
}

/// Blanket implementation for all tools
impl<T: Tool + ?Sized> ToolValidator for T {}

/// Optional trait for permission checking
///
/// This trait exists for documentation and future specialization purposes.
/// The default implementation is provided in the `Tool` trait itself.
#[async_trait]
pub trait ToolPermission: Tool {
    /// Get the risk level for this tool (mirrors Tool::risk_level)
    fn get_risk_level(&self) -> RiskLevel {
        self.risk_level()
    }

    /// Check if the tool call is permitted (mirrors Tool::check_permission)
    async fn check_tool_permission(
        &self,
        call: &ToolCall,
        context: &ToolContext,
    ) -> ToolPermissionResult {
        self.check_permission(call, context).await
    }
}

/// Blanket implementation for all tools
#[async_trait]
impl<T: Tool + ?Sized> ToolPermission for T {}

/// Optional trait for concurrency configuration
///
/// This trait exists for documentation and future specialization purposes.
/// The default implementation is provided in the `Tool` trait itself.
pub trait ToolConcurrency: Tool {
    /// Get the concurrency mode (mirrors Tool::concurrency_mode)
    fn get_concurrency_mode(&self) -> ConcurrencyMode {
        self.concurrency_mode()
    }

    /// Check if tool supports parallel execution (mirrors Tool::supports_parallel_execution)
    fn can_run_parallel(&self) -> bool {
        self.supports_parallel_execution()
    }
}

/// Blanket implementation for all tools
impl<T: Tool + ?Sized> ToolConcurrency for T {}

/// Optional trait for execution timing
///
/// This trait exists for documentation and future specialization purposes.
/// The default implementation is provided in the `Tool` trait itself.
pub trait ToolTiming: Tool {
    /// Get the maximum execution duration (mirrors Tool::max_execution_duration)
    fn get_max_duration(&self) -> Option<Duration> {
        self.max_execution_duration()
    }
}

/// Blanket implementation for all tools
impl<T: Tool + ?Sized> ToolTiming for T {}

/// Optional trait for rendering tool calls and results
///
/// This trait exists for documentation and future specialization purposes.
/// The default implementation is provided in the `Tool` trait itself.
pub trait ToolRenderer: Tool {
    /// Render the tool call for display (mirrors Tool::render_call)
    fn render_tool_call(&self, call: &ToolCall) -> String {
        self.render_call(call)
    }

    /// Render the tool result for display (mirrors Tool::render_result)
    fn render_tool_result(&self, result: &ToolResult) -> String {
        self.render_result(result)
    }
}

/// Blanket implementation for all tools
impl<T: Tool + ?Sized> ToolRenderer for T {}

/// Optional trait for tool metadata
///
/// This trait exists for documentation and future specialization purposes.
/// The default implementation is provided in the `Tool` trait itself.
pub trait ToolMetadata: Tool {
    /// Check if tool is read-only (mirrors Tool::is_read_only)
    fn is_tool_read_only(&self) -> bool {
        self.is_read_only()
    }

    /// Check if tool requires user interaction (mirrors Tool::requires_user_interaction)
    fn needs_user_interaction(&self) -> bool {
        self.requires_user_interaction()
    }
}

/// Blanket implementation for all tools
impl<T: Tool + ?Sized> ToolMetadata for T {}

// ============================================================================
// FullTool Trait - Convenience Trait Combining All Traits
// ============================================================================

/// Convenience trait that combines all tool traits
///
/// This trait is automatically implemented for any type that implements
/// the core `Tool` trait. It provides access to all functionality
/// from both the core trait and extension traits.
///
/// Use this as a trait bound when you need access to all tool functionality.
pub trait FullTool:
    Tool + ToolValidator + ToolPermission + ToolConcurrency + ToolTiming + ToolRenderer + ToolMetadata
{
}

/// Blanket implementation for FullTool
impl<T> FullTool for T where
    T: Tool
        + ToolValidator
        + ToolPermission
        + ToolConcurrency
        + ToolTiming
        + ToolRenderer
        + ToolMetadata
{
}
