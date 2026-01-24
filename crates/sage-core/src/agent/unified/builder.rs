//! Builder pattern for creating unified executors

use crate::agent::{ExecutionMode, ExecutionOptions};
use crate::config::model::Config;
use crate::error::SageResult;
use crate::input::InputChannel;
use crate::trajectory::SessionRecorder;
use crate::ui::traits::UiContext;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::UnifiedExecutor;

/// Builder for creating unified executors with fluent API
pub struct UnifiedExecutorBuilder {
    config: Config,
    options: ExecutionOptions,
    input_channel: Option<InputChannel>,
    session_recorder: Option<Arc<Mutex<SessionRecorder>>>,
    ui_context: Option<UiContext>,
}

impl UnifiedExecutorBuilder {
    /// Create a new builder with configuration
    pub fn new(config: Config) -> Self {
        Self {
            config,
            options: ExecutionOptions::default(),
            input_channel: None,
            session_recorder: None,
            ui_context: None,
        }
    }

    /// Set execution options
    pub fn with_options(mut self, options: ExecutionOptions) -> Self {
        self.options = options;
        self
    }

    /// Set execution mode
    pub fn with_mode(mut self, mode: ExecutionMode) -> Self {
        self.options.mode = mode;
        self
    }

    /// Set input channel for interactive mode
    pub fn with_input_channel(mut self, channel: InputChannel) -> Self {
        self.input_channel = Some(channel);
        self
    }

    /// Set session recorder
    pub fn with_session_recorder(mut self, recorder: Arc<Mutex<SessionRecorder>>) -> Self {
        self.session_recorder = Some(recorder);
        self
    }

    /// Set UI context for event handling
    ///
    /// This enables dependency injection for UI updates instead of using
    /// global state.
    pub fn with_ui_context(mut self, ui_context: UiContext) -> Self {
        self.ui_context = Some(ui_context);
        self
    }

    /// Set max steps (None = unlimited)
    pub fn with_max_steps(mut self, max_steps: Option<u32>) -> Self {
        self.options.max_steps = max_steps;
        self
    }

    /// Set a specific step limit
    pub fn with_step_limit(mut self, limit: u32) -> Self {
        self.options.max_steps = Some(limit);
        self
    }

    /// Set working directory
    pub fn with_working_directory(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.options.working_directory = Some(path.into());
        self
    }

    /// Build the unified executor
    pub fn build(self) -> SageResult<UnifiedExecutor> {
        let mut executor = UnifiedExecutor::with_options(self.config, self.options)?;

        if let Some(channel) = self.input_channel {
            executor.set_input_channel(channel);
        }

        if let Some(recorder) = self.session_recorder {
            executor.set_session_recorder(recorder);
        }

        if let Some(ui_context) = self.ui_context {
            executor.set_ui_context(ui_context);
        }

        Ok(executor)
    }
}
