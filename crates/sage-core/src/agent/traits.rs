//! Core trait abstractions for UnifiedExecutor
//!
//! These traits enable dependency injection, improve testability, and provide
//! extension points for customizing agent behavior.
//!
//! # Traits
//!
//! - [`LlmService`] - LLM chat operations abstraction
//! - [`ToolService`] - Tool execution abstraction
//! - [`SessionRecorderService`] - Session recording abstraction
//! - [`UserInteractionService`] - User input handling abstraction
//! - [`ProgressReporter`] - Progress/UI reporting abstraction

use crate::error::SageResult;
use crate::input::{InputRequest, InputResponse};
use crate::llm::messages::{LlmMessage, LlmResponse};
use crate::llm::streaming::LlmStream;
use crate::tools::base::Tool;
use crate::tools::types::{ToolCall, ToolResult, ToolSchema};
use async_trait::async_trait;
use std::sync::Arc;

// ============================================================================
// LLM Service Trait
// ============================================================================

/// LLM service abstraction for chat operations
///
/// This trait abstracts the LLM client, enabling:
/// - Easy mocking for unit tests
/// - Swapping LLM providers without changing agent code
/// - Adding middleware (logging, caching, rate limiting)
///
/// # Example
///
/// ```ignore
/// struct MockLlmService;
///
/// #[async_trait]
/// impl LlmService for MockLlmService {
///     async fn chat(&self, _messages: &[LlmMessage], _tools: Option<&[ToolSchema]>) -> SageResult<LlmResponse> {
///         Ok(LlmResponse::new("Mock response"))
///     }
///     // ... other methods
/// }
/// ```
#[async_trait]
pub trait LlmService: Send + Sync {
    /// Send a chat completion request
    async fn chat(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LlmResponse>;

    /// Send a streaming chat completion request
    async fn chat_stream(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LlmStream>;

    /// Get the provider name
    fn provider(&self) -> &str;

    /// Get the model name
    fn model(&self) -> &str;
}

// ============================================================================
// Tool Service Trait
// ============================================================================

/// Tool service abstraction for tool execution
///
/// This trait abstracts the tool executor, enabling:
/// - Easy mocking for unit tests
/// - Custom tool execution strategies
/// - Middleware for tool calls (logging, permission checks)
#[async_trait]
pub trait ToolService: Send + Sync {
    /// Execute a single tool call
    async fn execute_tool(&self, call: &ToolCall) -> ToolResult;

    /// Execute multiple tool calls (may run in parallel)
    async fn execute_tools(&self, calls: &[ToolCall]) -> Vec<ToolResult>;

    /// Get schemas for all registered tools
    fn get_tool_schemas(&self) -> Vec<ToolSchema>;

    /// Register a new tool
    fn register_tool(&mut self, tool: Arc<dyn Tool>);

    /// Check if a tool is registered
    fn has_tool(&self, name: &str) -> bool;
}

// ============================================================================
// Session Recorder Service Trait
// ============================================================================

/// Session recording abstraction for trajectory tracking
///
/// This trait abstracts session recording, enabling:
/// - Easy mocking for tests (no file I/O)
/// - Alternative storage backends (database, cloud)
/// - Replay and debugging capabilities
#[async_trait]
pub trait SessionRecorderService: Send + Sync {
    /// Record session start
    async fn record_session_start(
        &mut self,
        task: &str,
        provider: &str,
        model: &str,
    ) -> SageResult<()>;

    /// Record a user message
    async fn record_user_message(&mut self, content: &str) -> SageResult<()>;

    /// Record an assistant message with optional tool calls
    async fn record_assistant_message(
        &mut self,
        content: &str,
        tool_calls: Option<&[ToolCall]>,
    ) -> SageResult<()>;

    /// Record a tool execution result
    async fn record_tool_result(&mut self, result: &ToolResult) -> SageResult<()>;

    /// Record session end
    async fn record_session_end(
        &mut self,
        success: bool,
        final_result: Option<String>,
    ) -> SageResult<()>;
}

// ============================================================================
// User Interaction Service Trait
// ============================================================================

/// User interaction service abstraction
///
/// This trait abstracts user input handling, enabling:
/// - Easy mocking for automated tests
/// - Different UI backends (CLI, TUI, web)
/// - Scripted/automated responses
#[async_trait]
pub trait UserInteractionService: Send + Sync {
    /// Request input from the user
    ///
    /// Blocks until the user responds or the request is cancelled.
    async fn request_input(&mut self, request: InputRequest) -> SageResult<InputResponse>;

    /// Check if interactive input is available
    fn is_interactive(&self) -> bool;
}

// ============================================================================
// Progress Reporter Trait
// ============================================================================

/// Progress reporting abstraction for UI/animations
///
/// This trait abstracts progress reporting, enabling:
/// - Easy mocking for tests (no UI output)
/// - Different UI implementations
/// - Structured logging of execution progress
#[async_trait]
pub trait ProgressReporter: Send + Sync {
    /// Report the start of a new step
    async fn report_step_start(&self, step_number: u32);

    /// Report tool execution start
    async fn report_tool_start(&self, tool_name: &str, params_summary: &str);

    /// Report tool execution end
    async fn report_tool_end(&self, tool_name: &str, success: bool);

    /// Report a message (assistant content)
    async fn report_message(&self, content: &str);

    /// Report thinking/processing state
    async fn report_thinking(&self);

    /// Stop all animations/progress indicators
    async fn stop(&self);
}

// ============================================================================
// No-op Implementations for Testing
// ============================================================================

/// A no-op session recorder that does nothing (for testing)
pub struct NoopSessionRecorder;

#[async_trait]
impl SessionRecorderService for NoopSessionRecorder {
    async fn record_session_start(&mut self, _: &str, _: &str, _: &str) -> SageResult<()> {
        Ok(())
    }
    async fn record_user_message(&mut self, _: &str) -> SageResult<()> {
        Ok(())
    }
    async fn record_assistant_message(&mut self, _: &str, _: Option<&[ToolCall]>) -> SageResult<()> {
        Ok(())
    }
    async fn record_tool_result(&mut self, _: &ToolResult) -> SageResult<()> {
        Ok(())
    }
    async fn record_session_end(&mut self, _: bool, _: Option<String>) -> SageResult<()> {
        Ok(())
    }
}

/// A no-op progress reporter that does nothing (for testing)
pub struct NoopProgressReporter;

#[async_trait]
impl ProgressReporter for NoopProgressReporter {
    async fn report_step_start(&self, _: u32) {}
    async fn report_tool_start(&self, _: &str, _: &str) {}
    async fn report_tool_end(&self, _: &str, _: bool) {}
    async fn report_message(&self, _: &str) {}
    async fn report_thinking(&self) {}
    async fn stop(&self) {}
}
