//! Pre-execution phase: hooks and checkpoint creation

use super::ToolOrchestrator;
use super::config::{PreExecutionResult, ToolExecutionContext};
use crate::error::SageResult;
use crate::hooks::{HookEvent, HookInput};
use crate::tools::types::ToolCall;
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;

impl ToolOrchestrator {
    /// Execute pre-execution phase: run PreToolUse hooks and create checkpoint
    pub async fn pre_execution_phase(
        &self,
        tool_call: &ToolCall,
        context: &ToolExecutionContext,
        cancel_token: CancellationToken,
    ) -> SageResult<PreExecutionResult> {
        // Create checkpoint for file-modifying tools before execution
        if self.checkpoint_config.enabled {
            if let Some(manager) = &self.checkpoint_manager {
                if manager.should_checkpoint_for_tool(&tool_call.name) {
                    let affected_files = self.extract_affected_files(tool_call);
                    if !affected_files.is_empty() {
                        match manager
                            .create_pre_tool_checkpoint(&tool_call.name, &affected_files)
                            .await
                        {
                            Ok(checkpoint) => {
                                tracing::debug!(
                                    tool = %tool_call.name,
                                    checkpoint_id = %checkpoint.short_id(),
                                    files = ?affected_files,
                                    "Created pre-tool checkpoint"
                                );
                                let mut last_id = self.last_checkpoint_id.write().await;
                                *last_id = Some(checkpoint.id);
                            }
                            Err(e) => {
                                tracing::warn!(
                                    tool = %tool_call.name,
                                    error = %e,
                                    "Failed to create pre-tool checkpoint (continuing anyway)"
                                );
                            }
                        }
                    }
                }
            }
        }

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
                let reason = result.message().unwrap_or("Blocked by hook").to_string();
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

    /// Extract file paths affected by a tool call
    pub(super) fn extract_affected_files(&self, tool_call: &ToolCall) -> Vec<PathBuf> {
        let mut files = Vec::new();

        // Write/Edit tools use file_path or path
        if let Some(path) = tool_call
            .arguments
            .get("file_path")
            .or_else(|| tool_call.arguments.get("path"))
            .and_then(|v| v.as_str())
        {
            files.push(PathBuf::from(path));
        }

        // MultiEdit may have multiple files
        if let Some(edits) = tool_call.arguments.get("edits").and_then(|v| v.as_array()) {
            for edit in edits {
                if let Some(path) = edit.get("file_path").and_then(|v| v.as_str()) {
                    files.push(PathBuf::from(path));
                }
            }
        }

        files
    }
}
