//! Tool orchestration with three-phase execution model:
//! Pre-execution (hooks), Execution (tool), Post-execution (result hooks)

use crate::error::SageResult;
use crate::hooks::{HookEvent, HookExecutor, HookInput};
use crate::tools::executor::ToolExecutor;
use crate::tools::types::{ToolCall, ToolResult};
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;

/// Context for tool execution, providing session and environment info
#[derive(Clone)]
pub struct ToolExecutionContext {
    /// Session ID for hook input
    pub session_id: String,
    /// Working directory for hook execution
    pub working_dir: PathBuf,
}

impl ToolExecutionContext {
    /// Create a new execution context
    pub fn new(session_id: impl Into<String>, working_dir: PathBuf) -> Self {
        Self {
            session_id: session_id.into(),
            working_dir,
        }
    }
}

/// Result of the pre-execution phase
pub enum PreExecutionResult {
    /// Continue with execution
    Continue,
    /// Blocked by hook with reason
    Blocked(String),
}

impl PreExecutionResult {
    /// Check if execution should continue
    pub fn should_continue(&self) -> bool {
        matches!(self, PreExecutionResult::Continue)
    }

    /// Get block reason if blocked
    pub fn block_reason(&self) -> Option<&str> {
        match self {
            PreExecutionResult::Blocked(reason) => Some(reason),
            PreExecutionResult::Continue => None,
        }
    }
}

/// Orchestrates tool execution through three phases
pub struct ToolOrchestrator {
    tool_executor: ToolExecutor,
    hook_executor: HookExecutor,
}

impl ToolOrchestrator {
    /// Create a new tool orchestrator
    pub fn new(tool_executor: ToolExecutor, hook_executor: HookExecutor) -> Self {
        Self {
            tool_executor,
            hook_executor,
        }
    }

    /// Get a reference to the tool executor
    pub fn tool_executor(&self) -> &ToolExecutor {
        &self.tool_executor
    }

    /// Get a mutable reference to the tool executor
    pub fn tool_executor_mut(&mut self) -> &mut ToolExecutor {
        &mut self.tool_executor
    }

    /// Get a reference to the hook executor
    pub fn hook_executor(&self) -> &HookExecutor {
        &self.hook_executor
    }

    /// Set a new hook executor
    pub fn set_hook_executor(&mut self, hook_executor: HookExecutor) {
        self.hook_executor = hook_executor;
    }

    /// Execute pre-execution phase: run PreToolUse hooks
    pub async fn pre_execution_phase(
        &self,
        tool_call: &ToolCall,
        context: &ToolExecutionContext,
        cancel_token: CancellationToken,
    ) -> SageResult<PreExecutionResult> {
        let hook_input = HookInput::new(HookEvent::PreToolUse, &context.session_id)
            .with_cwd(context.working_dir.clone())
            .with_tool_name(&tool_call.name)
            .with_tool_input(serde_json::to_value(&tool_call.arguments).unwrap_or_default());

        let results = self
            .hook_executor
            .execute(
                HookEvent::PreToolUse,
                &tool_call.name,
                hook_input,
                cancel_token,
            )
            .await
            .unwrap_or_default();

        // Check if any hook blocked execution
        for result in &results {
            if !result.should_continue() {
                let reason = result
                    .message()
                    .unwrap_or("Blocked by hook")
                    .to_string();
                tracing::warn!(
                    tool = %tool_call.name,
                    reason = %reason,
                    "PreToolUse hook blocked tool execution"
                );
                return Ok(PreExecutionResult::Blocked(reason));
            }
        }

        Ok(PreExecutionResult::Continue)
    }

    /// Execute the tool (execution phase) with cancellation support
    pub async fn execution_phase(
        &self,
        tool_call: &ToolCall,
        cancel_token: CancellationToken,
    ) -> ToolResult {
        tokio::select! {
            result = self.tool_executor.execute_tool(tool_call) => {
                result
            }
            _ = cancel_token.cancelled() => {
                ToolResult::error(
                    &tool_call.id,
                    &tool_call.name,
                    "Tool execution cancelled by user",
                )
            }
        }
    }

    /// Execute post-execution phase: run PostToolUse/PostToolUseFailure hooks
    pub async fn post_execution_phase(
        &self,
        tool_call: &ToolCall,
        tool_result: &ToolResult,
        context: &ToolExecutionContext,
        cancel_token: CancellationToken,
    ) -> SageResult<()> {
        let event = if tool_result.success {
            HookEvent::PostToolUse
        } else {
            HookEvent::PostToolUseFailure
        };

        let hook_input = HookInput::new(event, &context.session_id)
            .with_cwd(context.working_dir.clone())
            .with_tool_name(&tool_call.name)
            .with_tool_input(serde_json::to_value(&tool_call.arguments).unwrap_or_default())
            .with_tool_result(serde_json::to_value(tool_result).unwrap_or_default());

        if let Err(e) = self
            .hook_executor
            .execute(event, &tool_call.name, hook_input, cancel_token)
            .await
        {
            tracing::warn!(
                error = %e,
                tool_name = %tool_call.name,
                event = ?event,
                "Post-execution hook failed (non-fatal)"
            );
        }

        Ok(())
    }

    /// Check if a tool requires user interaction
    pub fn requires_user_interaction(&self, tool_name: &str) -> bool {
        self.tool_executor
            .get_tool(tool_name)
            .map(|t| t.requires_user_interaction())
            .unwrap_or(false)
    }

    /// Execute a tool call with three-phase model (all phases in sequence)
    pub async fn execute_tool_call(
        &self,
        tool_call: &ToolCall,
        context: &ToolExecutionContext,
        cancel_token: CancellationToken,
    ) -> SageResult<ToolResult> {
        // Pre-execution phase
        let pre_result = self
            .pre_execution_phase(tool_call, context, cancel_token.clone())
            .await?;
        if let Some(reason) = pre_result.block_reason() {
            return Ok(ToolResult::error(
                &tool_call.id,
                &tool_call.name,
                format!("Tool execution blocked by hook: {}", reason),
            ));
        }

        // Execution phase (with cancellation support)
        let tool_result = self.execution_phase(tool_call, cancel_token.clone()).await;

        // Post-execution phase
        self.post_execution_phase(tool_call, &tool_result, context, cancel_token)
            .await?;

        Ok(tool_result)
    }
}
