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
mod context_builder;
mod event_manager;
mod execution_loop;
mod executor;
mod input_channel;
mod llm_orchestrator;
mod message_builder;
mod permission_handler;
mod recording;
mod session;
mod session_manager;
mod step_execution;
mod tool_display;
mod tool_orchestrator;
mod user_interaction;

#[cfg(test)]
mod tests;

pub use builder::UnifiedExecutorBuilder;
pub use context_builder::{ContextBuilder, GitInfo, ProjectContext};
pub use event_manager::{EventManager, ExecutionEvent};
pub use llm_orchestrator::LlmOrchestrator;
pub use session_manager::SessionManager;
pub use tool_orchestrator::{PreExecutionResult, ToolExecutionContext, ToolOrchestrator};

use crate::agent::subagent::init_global_runner_from_config;
use crate::context::AutoCompact;
use crate::error::{SageError, SageResult};
use crate::hooks::HookRegistry;
use crate::input::{InputChannel, InputRequest, InputResponse};
use crate::skills::SkillRegistry;
use crate::trajectory::SessionRecorder;
use crate::types::Id;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::instrument;

// Re-export types for convenience
use crate::agent::ExecutionOptions;
use crate::config::model::Config;

/// Unified executor that implements the Claude Code style execution loop
pub struct UnifiedExecutor {
    /// Unique identifier
    id: Id,
    /// Configuration
    config: Config,
    /// LLM orchestrator for model interactions (centralized LLM communication)
    llm_orchestrator: LlmOrchestrator,
    /// Tool orchestrator for three-phase tool execution
    tool_orchestrator: ToolOrchestrator,
    /// Execution options
    options: ExecutionOptions,
    /// Input channel for blocking user input (None for batch mode)
    input_channel: Option<InputChannel>,
    /// Event manager for unified event handling and UI animations
    event_manager: EventManager,
    /// Session manager encapsulating all session-related state
    session_manager: SessionManager,
    /// Auto-compact manager for context window management
    auto_compact: AutoCompact,
    /// Skill registry for AI auto-invocation (Claude Code compatible)
    skill_registry: Arc<RwLock<SkillRegistry>>,
}

impl UnifiedExecutor {
    /// Set the input channel for interactive mode
    pub fn set_input_channel(&mut self, channel: InputChannel) {
        self.input_channel = Some(channel);
    }

    /// Set session recorder
    pub fn set_session_recorder(&mut self, recorder: Arc<Mutex<SessionRecorder>>) {
        self.session_manager.set_session_recorder(recorder);
    }

    /// Get current session ID
    pub fn current_session_id(&self) -> Option<&str> {
        self.session_manager.current_session_id()
    }

    /// Get the file tracker for external file tracking
    pub fn file_tracker_mut(&mut self) -> &mut crate::session::FileSnapshotTracker {
        self.session_manager.file_tracker_mut()
    }

    /// Get the session manager for external session management
    pub fn session_manager(&self) -> &SessionManager {
        &self.session_manager
    }

    /// Get the session manager mutably
    pub fn session_manager_mut(&mut self) -> &mut SessionManager {
        &mut self.session_manager
    }

    /// Register a tool with the executor
    pub fn register_tool(&mut self, tool: Arc<dyn crate::tools::base::Tool>) {
        self.tool_orchestrator.tool_executor_mut().register_tool(tool);
    }

    /// Register multiple tools with the executor
    pub fn register_tools(&mut self, tools: Vec<Arc<dyn crate::tools::base::Tool>>) {
        for tool in tools {
            self.tool_orchestrator.tool_executor_mut().register_tool(tool);
        }
    }

    /// Initialize sub-agent support
    ///
    /// This should be called after all tools are registered to enable
    /// the Task tool to execute sub-agents (Explore, Plan, etc.)
    pub fn init_subagent_support(&self) -> SageResult<()> {
        // Get all registered tools from the executor
        let tool_executor = self.tool_orchestrator.tool_executor();
        let tool_names = tool_executor.tool_names();
        let tools: Vec<Arc<dyn crate::tools::base::Tool>> = tool_names
            .iter()
            .filter_map(|name| tool_executor.get_tool(name).cloned())
            .collect();

        tracing::info!("Initializing sub-agent support with {} tools", tools.len());

        init_global_runner_from_config(&self.config, tools, self.options.working_directory.clone())
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

    /// Get the skill registry for managing skills
    pub fn skill_registry(&self) -> Arc<RwLock<SkillRegistry>> {
        Arc::clone(&self.skill_registry)
    }

    /// Set the hook registry for the executor
    ///
    /// This allows configuring hooks for PreToolUse, PostToolUse, and other events.
    /// Hooks can be used to:
    /// - Block tool execution based on custom logic
    /// - Log or audit tool calls
    /// - Modify tool behavior
    pub fn set_hook_registry(&mut self, registry: HookRegistry) {
        use crate::hooks::HookExecutor;
        self.tool_orchestrator.set_hook_executor(HookExecutor::new(registry));
    }

    /// Get a reference to the hook executor
    pub fn hook_executor(&self) -> &crate::hooks::HookExecutor {
        self.tool_orchestrator.hook_executor()
    }

    /// Get a reference to the tool orchestrator
    pub fn tool_orchestrator(&self) -> &ToolOrchestrator {
        &self.tool_orchestrator
    }

    /// Get a mutable reference to the tool orchestrator
    pub fn tool_orchestrator_mut(&mut self) -> &mut ToolOrchestrator {
        &mut self.tool_orchestrator
    }

    /// Discover skills from the file system
    ///
    /// This scans:
    /// - `.sage/skills/` - Project-specific skills
    /// - `~/.config/sage/skills/` - User-level skills
    ///
    /// Returns the number of skills discovered.
    pub async fn discover_skills(&self) -> SageResult<usize> {
        let mut registry = self.skill_registry.write().await;
        registry.discover().await
    }

    /// Graceful shutdown - cleanup resources and save state
    ///
    /// This method should be called when the executor is shutting down
    /// to ensure all resources are properly cleaned up.
    #[instrument(skip(self))]
    pub async fn shutdown(&mut self) -> SageResult<()> {
        tracing::info!("Initiating graceful shutdown of UnifiedExecutor");

        // Stop any animations
        self.event_manager.stop_animation().await;

        // Finalize session recording if present
        if let Some(recorder) = self.session_manager.session_recorder() {
            tracing::debug!("Finalizing session recording");
            let mut recorder_guard = recorder.lock().await;
            if let Err(e) = recorder_guard
                .record_session_end(false, Some("Shutdown".to_string()))
                .await
            {
                tracing::warn!("Failed to finalize session recording: {}", e);
            }
        }

        // Log session cleanup
        if let Some(session_id) = self.session_manager.current_session_id() {
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
