//! Execution and post-execution phases: tool running with supervision and hooks

use super::ToolOrchestrator;
use super::config::ToolExecutionContext;
use crate::checkpoints::RestoreOptions;
use crate::error::{SageError, SageResult};
use crate::hooks::{HookEvent, HookInput};
use crate::recovery::supervisor::{SupervisionResult, TaskSupervisor};
use crate::tools::types::{ToolCall, ToolResult};
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

impl ToolOrchestrator {
    /// Execute the tool (execution phase) with cancellation and supervision support
    pub async fn execution_phase(
        &self,
        tool_call: &ToolCall,
        context: &ToolExecutionContext,
        cancel_token: CancellationToken,
    ) -> ToolResult {
        // If supervision is disabled, execute directly
        if !self.supervision_config.enabled {
            return self
                .execute_tool_direct(tool_call, context, cancel_token)
                .await;
        }

        // Create a supervisor for this tool execution
        let mut supervisor = TaskSupervisor::new(format!("tool_{}", tool_call.name))
            .with_policy(self.supervision_config.policy.clone())
            .with_cancel_token(cancel_token.clone());

        let tool_call_clone = tool_call.clone();
        let executor = &self.tool_executor;
        let tool_context = context.to_tool_context();
        let completed_tool_result: Arc<Mutex<Option<ToolResult>>> = Arc::new(Mutex::new(None));

        let supervision_result = supervisor
            .supervise(|| {
                let call = tool_call_clone.clone();
                let context = tool_context.clone();
                let completed_tool_result = completed_tool_result.clone();
                async move {
                    let result = executor.execute_tool_with_context(&call, &context).await;
                    if result.success {
                        if let Ok(mut slot) = completed_tool_result.lock() {
                            *slot = Some(result.clone());
                        }
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
            SupervisionResult::Completed => completed_tool_result
                .lock()
                .ok()
                .and_then(|mut slot| slot.take())
                .unwrap_or_else(|| {
                    ToolResult::error(
                        &tool_call.id,
                        &tool_call.name,
                        "Tool supervision completed without a result",
                    )
                }),
            SupervisionResult::Restarted { attempt } => {
                tracing::info!(
                    tool = %tool_call.name,
                    attempt = attempt,
                    "Tool execution restarted, attempting again"
                );
                self.execute_tool_direct(tool_call, context, cancel_token)
                    .await
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
        context: &ToolExecutionContext,
        cancel_token: CancellationToken,
    ) -> ToolResult {
        let tool_context = context.to_tool_context();
        tokio::select! {
            result = self.tool_executor.execute_tool_with_context(tool_call, &tool_context) => {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::{HookExecutor, HookRegistry};
    use crate::tools::base::{Tool, ToolError};
    use crate::tools::executor::ToolExecutor;
    use crate::tools::permission::ToolContext;
    use crate::tools::types::{ToolCall, ToolSchema};
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingContextTool {
        calls: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl Tool for CountingContextTool {
        fn name(&self) -> &str {
            "counting_context"
        }

        fn description(&self) -> &str {
            "Counts context-aware executions"
        }

        fn schema(&self) -> ToolSchema {
            ToolSchema::new(self.name(), self.description(), vec![])
        }

        async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(ToolResult::success(&call.id, self.name(), "no-context"))
        }

        async fn execute_with_context(
            &self,
            call: &ToolCall,
            context: &ToolContext,
        ) -> Result<ToolResult, ToolError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(ToolResult::success(
                &call.id,
                self.name(),
                context.session_id.as_deref().unwrap_or("missing-session"),
            ))
        }
    }

    #[tokio::test]
    async fn supervision_success_returns_result_without_second_tool_execution() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut tool_executor = ToolExecutor::new();
        tool_executor.register_tool(Arc::new(CountingContextTool {
            calls: calls.clone(),
        }));
        let orchestrator =
            ToolOrchestrator::new(tool_executor, HookExecutor::new(HookRegistry::new()));
        let context =
            ToolExecutionContext::new("parent-thread", std::env::current_dir().unwrap_or_default());
        let call = ToolCall::new("call-1", "counting_context", HashMap::new());

        let result = orchestrator
            .execution_phase(&call, &context, CancellationToken::new())
            .await;

        assert!(result.success);
        assert_eq!(result.output.as_deref(), Some("parent-thread"));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
