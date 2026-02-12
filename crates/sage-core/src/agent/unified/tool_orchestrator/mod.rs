//! Tool orchestration with three-phase execution model:
//! Pre-execution (hooks), Execution (tool), Post-execution (result hooks)

mod config;
mod execution;
mod pre_execution;

pub use config::{CheckpointConfig, PreExecutionResult, SupervisionConfig, ToolExecutionContext};

use crate::checkpoints::{CheckpointId, CheckpointManager, RestoreOptions};
use crate::error::SageResult;
use crate::hooks::HookExecutor;
use crate::tools::executor::ToolExecutor;
use crate::tools::types::{ToolCall, ToolResult};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// Orchestrates tool execution through three phases
pub struct ToolOrchestrator {
    pub(super) tool_executor: ToolExecutor,
    pub(super) hook_executor: HookExecutor,
    pub(super) supervision_config: SupervisionConfig,
    pub(super) checkpoint_manager: Option<Arc<CheckpointManager>>,
    pub(super) checkpoint_config: CheckpointConfig,
    /// Track the last checkpoint ID for potential rollback
    pub(super) last_checkpoint_id: tokio::sync::RwLock<Option<CheckpointId>>,
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

    /// Rollback to the last checkpoint (manual rollback)
    pub async fn rollback_last_checkpoint(&self) -> SageResult<bool> {
        if let Some(manager) = &self.checkpoint_manager {
            let last_id = self.last_checkpoint_id.read().await;
            if let Some(checkpoint_id) = last_id.as_ref() {
                let result = manager
                    .restore(checkpoint_id, RestoreOptions::files_only())
                    .await?;
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
