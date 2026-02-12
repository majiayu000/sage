//! Execution and post-execution phases: tool running with supervision and hooks

use super::ToolOrchestrator;
use super::config::ToolExecutionContext;
use crate::checkpoints::RestoreOptions;
use crate::error::{SageError, SageResult};
use crate::hooks::{HookEvent, HookInput};
use crate::recovery::supervisor::{SupervisionResult, TaskSupervisor};
use crate::tools::types::{ToolCall, ToolResult};
use tokio_util::sync::CancellationToken;

impl ToolOrchestrator {
    /// Execute the tool (execution phase) with cancellation and supervision support
    pub async fn execution_phase(
        &self,
        tool_call: &ToolCall,
        cancel_token: CancellationToken,
    ) -> ToolResult {
        // If supervision is disabled, execute directly
        if !self.supervision_config.enabled {
            return self.execute_tool_direct(tool_call, cancel_token).await;
        }

        // Create a supervisor for this tool execution
        let mut supervisor = TaskSupervisor::new(format!("tool_{}", tool_call.name))
            .with_policy(self.supervision_config.policy.clone())
            .with_cancel_token(cancel_token.clone());

        let tool_call_clone = tool_call.clone();
        let executor = &self.tool_executor;

        let supervision_result = supervisor
            .supervise(|| {
                let call = tool_call_clone.clone();
                async move {
                    let result = executor.execute_tool(&call).await;
                    if result.success {
                        Ok(result)
                    } else {
                        Err(SageError::tool(
                            &call.name,
                            result
                                .output
                                .clone()
                                .unwrap_or_else(|| "Tool failed".to_string()),
                        ))
                    }
                }
            })
            .await;

        // Convert supervision result back to ToolResult
        match supervision_result {
            SupervisionResult::Completed => self.execute_tool_direct(tool_call, cancel_token).await,
            SupervisionResult::Restarted { attempt } => {
                tracing::info!(
                    tool = %tool_call.name,
                    attempt = attempt,
                    "Tool execution restarted, attempting again"
                );
                self.execute_tool_direct(tool_call, cancel_token).await
            }
            SupervisionResult::Resumed { error } => {
                tracing::warn!(
                    tool = %tool_call.name,
                    error = %error.message,
                    "Tool execution resumed after error"
                );
                ToolResult::error(&tool_call.id, &tool_call.name, error.message)
            }
            SupervisionResult::Stopped { error } => {
                tracing::warn!(
                    tool = %tool_call.name,
                    error = %error.message,
                    "Tool execution stopped by supervisor"
                );
                ToolResult::error(&tool_call.id, &tool_call.name, error.message)
            }
            SupervisionResult::Escalated { error } => {
                tracing::error!(
                    tool = %tool_call.name,
                    error = %error.message,
                    "Tool execution failure escalated"
                );
                ToolResult::error(
                    &tool_call.id,
                    &tool_call.name,
                    format!("Escalated failure: {}", error.message),
                )
            }
        }
    }

    /// Execute tool directly without supervision
    async fn execute_tool_direct(
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

    /// Execute post-execution phase: run PostToolUse/PostToolUseFailure hooks and handle rollback
    pub async fn post_execution_phase(
        &self,
        tool_call: &ToolCall,
        tool_result: &ToolResult,
        context: &ToolExecutionContext,
        cancel_token: CancellationToken,
    ) -> SageResult<()> {
        // Handle potential rollback on failure
        if !tool_result.success && self.checkpoint_config.auto_rollback_on_failure {
            if let Some(manager) = &self.checkpoint_manager {
                let last_id = self.last_checkpoint_id.read().await;
                if let Some(checkpoint_id) = last_id.as_ref() {
                    match manager
                        .restore(checkpoint_id, RestoreOptions::files_only())
                        .await
                    {
                        Ok(result) => {
                            tracing::info!(
                                tool = %tool_call.name,
                                checkpoint_id = %checkpoint_id.short(),
                                restored = result.restored_count(),
                                "Auto-rolled back after tool failure"
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                tool = %tool_call.name,
                                checkpoint_id = %checkpoint_id.short(),
                                error = %e,
                                "Failed to rollback after tool failure"
                            );
                        }
                    }
                }
            }
        }

        // Clear checkpoint ID after post-execution
        if self.checkpoint_config.enabled {
            let mut last_id = self.last_checkpoint_id.write().await;
            *last_id = None;
        }

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
}
