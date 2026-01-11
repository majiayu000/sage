//! Tool orchestration with three-phase execution model:
//! Pre-execution (hooks), Execution (tool), Post-execution (result hooks)

use crate::checkpoints::{CheckpointManager, CheckpointId, RestoreOptions};
use crate::error::{SageError, SageResult};
use crate::hooks::{HookEvent, HookExecutor, HookInput};
use crate::recovery::supervisor::{SupervisionPolicy, SupervisionResult, TaskSupervisor};
use crate::tools::executor::ToolExecutor;
use crate::tools::types::{ToolCall, ToolResult};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
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

/// Configuration for tool execution supervision
#[derive(Debug, Clone)]
pub struct SupervisionConfig {
    /// Whether supervision is enabled
    pub enabled: bool,
    /// Supervision policy for tool failures
    pub policy: SupervisionPolicy,
}

impl Default for SupervisionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            policy: SupervisionPolicy::Restart {
                max_restarts: 2,
                window: Duration::from_secs(60),
            },
        }
    }
}

impl SupervisionConfig {
    /// Create supervision config with no retries (for tools that shouldn't retry)
    pub fn no_retry() -> Self {
        Self {
            enabled: true,
            policy: SupervisionPolicy::Stop,
        }
    }

    /// Disable supervision entirely
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            policy: SupervisionPolicy::Stop,
        }
    }
}

/// Configuration for checkpoint behavior
#[derive(Debug, Clone)]
pub struct CheckpointConfig {
    /// Whether checkpointing is enabled
    pub enabled: bool,
    /// Whether to auto-rollback on tool failure
    pub auto_rollback_on_failure: bool,
}

impl Default for CheckpointConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_rollback_on_failure: false, // Disabled by default for safety
        }
    }
}

impl CheckpointConfig {
    /// Create config with auto-rollback enabled
    pub fn with_auto_rollback() -> Self {
        Self {
            enabled: true,
            auto_rollback_on_failure: true,
        }
    }

    /// Disable checkpointing entirely
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            auto_rollback_on_failure: false,
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
    supervision_config: SupervisionConfig,
    checkpoint_manager: Option<Arc<CheckpointManager>>,
    checkpoint_config: CheckpointConfig,
    /// Track the last checkpoint ID for potential rollback
    last_checkpoint_id: tokio::sync::RwLock<Option<CheckpointId>>,
}

impl ToolOrchestrator {
    /// Create a new tool orchestrator
    pub fn new(tool_executor: ToolExecutor, hook_executor: HookExecutor) -> Self {
        Self {
            tool_executor,
            hook_executor,
            supervision_config: SupervisionConfig::default(),
            checkpoint_manager: None,
            checkpoint_config: CheckpointConfig::default(),
            last_checkpoint_id: tokio::sync::RwLock::new(None),
        }
    }

    /// Create a new tool orchestrator with custom supervision config
    pub fn with_supervision(
        tool_executor: ToolExecutor,
        hook_executor: HookExecutor,
        supervision_config: SupervisionConfig,
    ) -> Self {
        Self {
            tool_executor,
            hook_executor,
            supervision_config,
            checkpoint_manager: None,
            checkpoint_config: CheckpointConfig::default(),
            last_checkpoint_id: tokio::sync::RwLock::new(None),
        }
    }

    /// Set the checkpoint manager for automatic checkpointing
    pub fn set_checkpoint_manager(&mut self, manager: Arc<CheckpointManager>) {
        self.checkpoint_manager = Some(manager);
    }

    /// Set the checkpoint configuration
    pub fn set_checkpoint_config(&mut self, config: CheckpointConfig) {
        self.checkpoint_config = config;
    }

    /// Get the checkpoint configuration
    pub fn checkpoint_config(&self) -> &CheckpointConfig {
        &self.checkpoint_config
    }

    /// Set the supervision configuration
    pub fn set_supervision_config(&mut self, config: SupervisionConfig) {
        self.supervision_config = config;
    }

    /// Get the supervision configuration
    pub fn supervision_config(&self) -> &SupervisionConfig {
        &self.supervision_config
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
                        match manager.create_pre_tool_checkpoint(&tool_call.name, &affected_files).await {
                            Ok(checkpoint) => {
                                tracing::debug!(
                                    tool = %tool_call.name,
                                    checkpoint_id = %checkpoint.short_id(),
                                    files = ?affected_files,
                                    "Created pre-tool checkpoint"
                                );
                                // Store checkpoint ID for potential rollback
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

    /// Extract file paths affected by a tool call
    fn extract_affected_files(&self, tool_call: &ToolCall) -> Vec<PathBuf> {
        let mut files = Vec::new();

        // Write/Edit tools use file_path or path
        if let Some(path) = tool_call.arguments.get("file_path")
            .or_else(|| tool_call.arguments.get("path"))
            .and_then(|v| v.as_str())
        {
            files.push(PathBuf::from(path));
        }

        // MultiEdit may have multiple files
        if let Some(edits) = tool_call.arguments.get("edits")
            .and_then(|v| v.as_array())
        {
            for edit in edits {
                if let Some(path) = edit.get("file_path").and_then(|v| v.as_str()) {
                    files.push(PathBuf::from(path));
                }
            }
        }

        files
    }

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

        // Track the final result across potential restarts
        let tool_call_clone = tool_call.clone();
        let executor = &self.tool_executor;

        // Execute with supervision - we need to handle the fact that
        // ToolResult doesn't implement Err, so we convert success/failure
        let supervision_result = supervisor
            .supervise(|| {
                let call = tool_call_clone.clone();
                async move {
                    let result = executor.execute_tool(&call).await;
                    // Convert ToolResult to Result for supervision
                    if result.success {
                        Ok(result)
                    } else {
                        // Create an error from the tool failure for supervision to handle
                        Err(SageError::tool(
                            &call.name,
                            result.output.clone().unwrap_or_else(|| "Tool failed".to_string()),
                        ))
                    }
                }
            })
            .await;

        // Convert supervision result back to ToolResult
        match supervision_result {
            SupervisionResult::Completed => {
                // Re-execute to get the actual result (supervision doesn't return it)
                self.execute_tool_direct(tool_call, cancel_token).await
            }
            SupervisionResult::Restarted { attempt } => {
                tracing::info!(
                    tool = %tool_call.name,
                    attempt = attempt,
                    "Tool execution restarted, attempting again"
                );
                // After restart signal, try execution again
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
                    match manager.restore(checkpoint_id, RestoreOptions::files_only()).await {
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

    /// Rollback to the last checkpoint (manual rollback)
    pub async fn rollback_last_checkpoint(&self) -> SageResult<bool> {
        if let Some(manager) = &self.checkpoint_manager {
            let last_id = self.last_checkpoint_id.read().await;
            if let Some(checkpoint_id) = last_id.as_ref() {
                let result = manager.restore(checkpoint_id, RestoreOptions::files_only()).await?;
                tracing::info!(
                    checkpoint_id = %checkpoint_id.short(),
                    restored = result.restored_count(),
                    "Rolled back to checkpoint"
                );
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Get the last checkpoint ID (if any)
    pub async fn last_checkpoint_id(&self) -> Option<CheckpointId> {
        self.last_checkpoint_id.read().await.clone()
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
