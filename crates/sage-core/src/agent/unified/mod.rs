//! Unified executor for the agent execution loop
//!
//! This module implements the Claude Code style unified execution loop where:
//! - There's no distinction between "run" and "interactive" modes at the core level
//! - User input is handled via InputChannel which blocks within the loop
//! - The loop never exits for user input - it waits inline
//!
//! # Design
//!
//! ```text
//! User Input → execute_task(options, input_channel) → Execution Loop
//!     → Tool calls (including ask_user_question) → Block on InputChannel
//!     → User responds → Loop continues (no exit/resume)
//!     → Task completes → Return ExecutionOutcome
//! ```

mod builder;
mod constructor;
mod execution_loop;
mod executor;
mod input_channel;
mod message_builder;
mod session;
mod step_execution;
mod user_interaction;

#[cfg(test)]
mod tests;

pub use builder::UnifiedExecutorBuilder;

use crate::agent::subagent::init_global_runner_from_config;
use crate::error::{SageError, SageResult};
use crate::input::{InputChannel, InputRequest, InputResponse};
use crate::session::{
    FileSnapshotTracker, JsonlSessionStorage, MessageChainTracker,
};
use crate::tools::executor::ToolExecutor;
use crate::trajectory::recorder::TrajectoryRecorder;
use crate::types::Id;
use crate::ui::AnimationManager;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::instrument;

// Re-export types for convenience
use crate::agent::ExecutionOptions;
use crate::config::model::Config;
use crate::llm::client::LlmClient;

/// Unified executor that implements the Claude Code style execution loop
pub struct UnifiedExecutor {
    /// Unique identifier
    id: Id,
    /// Configuration
    config: Config,
    /// LLM client for model interactions
    llm_client: LlmClient,
    /// Tool executor for running tools
    tool_executor: ToolExecutor,
    /// Execution options
    options: ExecutionOptions,
    /// Input channel for blocking user input (None for batch mode)
    input_channel: Option<InputChannel>,
    /// Trajectory recorder
    trajectory_recorder: Option<Arc<Mutex<TrajectoryRecorder>>>,
    /// Animation manager
    animation_manager: AnimationManager,
    /// JSONL session storage for enhanced messages
    jsonl_storage: Option<Arc<JsonlSessionStorage>>,
    /// Message chain tracker for building message relationships
    message_tracker: MessageChainTracker,
    /// Current session ID
    current_session_id: Option<String>,
    /// File snapshot tracker for undo capability
    file_tracker: FileSnapshotTracker,
}

impl UnifiedExecutor {
    /// Set the input channel for interactive mode
    pub fn set_input_channel(&mut self, channel: InputChannel) {
        self.input_channel = Some(channel);
    }

    /// Set trajectory recorder
    pub fn set_trajectory_recorder(&mut self, recorder: Arc<Mutex<TrajectoryRecorder>>) {
        self.trajectory_recorder = Some(recorder);
    }

    /// Get current session ID
    pub fn current_session_id(&self) -> Option<&str> {
        self.current_session_id.as_deref()
    }

    /// Get the file tracker for external file tracking
    pub fn file_tracker_mut(&mut self) -> &mut FileSnapshotTracker {
        &mut self.file_tracker
    }

    /// Register a tool with the executor
    pub fn register_tool(&mut self, tool: Arc<dyn crate::tools::base::Tool>) {
        self.tool_executor.register_tool(tool);
    }

    /// Register multiple tools with the executor
    pub fn register_tools(&mut self, tools: Vec<Arc<dyn crate::tools::base::Tool>>) {
        for tool in tools {
            self.tool_executor.register_tool(tool);
        }
    }

    /// Initialize sub-agent support
    ///
    /// This should be called after all tools are registered to enable
    /// the Task tool to execute sub-agents (Explore, Plan, etc.)
    pub fn init_subagent_support(&self) -> SageResult<()> {
        // Get all registered tools from the executor
        let tool_names = self.tool_executor.tool_names();
        let tools: Vec<Arc<dyn crate::tools::base::Tool>> = tool_names
            .iter()
            .filter_map(|name| self.tool_executor.get_tool(name).cloned())
            .collect();

        tracing::info!("Initializing sub-agent support with {} tools", tools.len());

        init_global_runner_from_config(&self.config, tools)
    }

    /// Get the executor ID
    pub fn id(&self) -> Id {
        self.id
    }

    /// Get configuration
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get execution options
    pub fn options(&self) -> &ExecutionOptions {
        &self.options
    }

    /// Graceful shutdown - cleanup resources and save state
    ///
    /// This method should be called when the executor is shutting down
    /// to ensure all resources are properly cleaned up.
    #[instrument(skip(self))]
    pub async fn shutdown(&mut self) -> SageResult<()> {
        tracing::info!("Initiating graceful shutdown of UnifiedExecutor");

        // Stop any animations
        self.animation_manager.stop_animation().await;

        // Finalize trajectory recording if present
        if let Some(recorder) = &self.trajectory_recorder {
            tracing::debug!("Finalizing trajectory recording");
            let mut recorder_guard = recorder.lock().await;
            if let Err(e) = recorder_guard
                .finalize_recording(false, Some("Shutdown".to_string()))
                .await
            {
                tracing::warn!("Failed to finalize trajectory recording: {}", e);
            }
        }

        // Log session cleanup
        if let Some(session_id) = &self.current_session_id {
            tracing::debug!("Session {} shutdown complete", session_id);
        }

        tracing::info!("Graceful shutdown complete");
        Ok(())
    }

    /// Request user input via the input channel
    ///
    /// This method blocks until the user responds (or auto-responds in non-interactive mode).
    /// If no input channel is set (batch mode without channel), returns an error.
    #[instrument(skip(self, request), fields(request_id = %request.id))]
    pub async fn request_user_input(&mut self, request: InputRequest) -> SageResult<InputResponse> {
        match &mut self.input_channel {
            Some(channel) => channel.request_input(request).await,
            None => Err(SageError::agent(
                "No input channel configured - cannot request user input",
            )),
        }
    }
}
