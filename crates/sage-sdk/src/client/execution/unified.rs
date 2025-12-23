//! Unified executor methods

use crate::client::{ExecutionResult, SageAgentSdk, UnifiedRunOptions};
use sage_core::{
    agent::{ExecutionMode, ExecutionOptions, UnifiedExecutor},
    error::SageResult,
    input::{InputChannel, InputChannelHandle},
    trajectory::recorder::TrajectoryRecorder,
    types::TaskMetadata,
};
use sage_tools::get_default_tools;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

impl SageAgentSdk {
    /// Execute a task using the unified execution loop (Claude Code style)
    ///
    /// This method uses a unified execution model where:
    /// - The execution loop never exits for user input
    /// - User questions block inline via InputChannel
    /// - Returns an InputChannelHandle for handling user prompts
    ///
    /// # Example
    /// ```ignore
    /// let sdk = SageAgentSdk::new()?;
    /// let (result_future, input_handle) = sdk.execute_unified("Create a web server", UnifiedRunOptions::default())?;
    ///
    /// // Handle user input in another task
    /// tokio::spawn(async move {
    ///     while let Some(request) = input_handle.request_rx.recv().await {
    ///         let response = InputResponse::text(request.id, "user input here");
    ///         input_handle.respond(response).await.unwrap();
    ///     }
    /// });
    ///
    /// let result = result_future.await;
    /// ```
    pub fn execute_unified(
        &self,
        task_description: &str,
        options: UnifiedRunOptions,
    ) -> SageResult<(
        impl std::future::Future<Output = SageResult<ExecutionResult>>,
        Option<InputChannelHandle>,
    )> {
        // Create task metadata
        let working_dir = options
            .working_directory
            .clone()
            .or_else(|| self.config.working_directory.clone())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        let task = TaskMetadata::new(task_description, &working_dir.to_string_lossy());

        // Set up execution options
        let mode = if options.non_interactive {
            ExecutionMode::non_interactive()
        } else {
            ExecutionMode::interactive()
        };

        let max_steps = options.max_steps.or(self.config.max_steps);
        let mut exec_options = ExecutionOptions::default()
            .with_mode(mode)
            .with_max_steps(max_steps);

        if let Some(dir) = &options.working_directory {
            exec_options = exec_options.with_working_directory(dir);
        }

        // Create the unified executor
        let mut executor = UnifiedExecutor::with_options(self.config.clone(), exec_options)?;

        // Register default tools
        executor.register_tools(get_default_tools());

        // Initialize sub-agent support
        if let Err(e) = executor.init_subagent_support() {
            tracing::warn!("Failed to initialize sub-agent support: {}", e);
        }

        // Set up input channel if interactive
        let input_handle = if !options.non_interactive {
            let (input_channel, handle) = InputChannel::new(16);
            executor.set_input_channel(input_channel);
            Some(handle)
        } else {
            None
        };

        // Set up trajectory recording if requested
        let trajectory_path = if options.enable_trajectory || self.trajectory_path.is_some() {
            let path = options
                .trajectory_path
                .clone()
                .or_else(|| self.trajectory_path.clone())
                .unwrap_or_else(|| {
                    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
                    self.config
                        .trajectory
                        .directory
                        .join(format!("sage_{}.json", timestamp))
                });

            let recorder = TrajectoryRecorder::new(&path)?;
            executor.set_trajectory_recorder(Arc::new(Mutex::new(recorder)));
            Some(path)
        } else {
            None
        };

        let config_used = self.config.clone();

        // Create the future that will execute the task
        let execution_future = async move {
            let outcome = executor.execute(task).await?;
            Ok(ExecutionResult::new(
                outcome,
                trajectory_path,
                config_used,
            ))
        };

        Ok((execution_future, input_handle))
    }

    /// Execute a task in non-interactive mode using the unified executor.
    ///
    /// This is a simpler API for cases where no user interaction is needed.
    /// The agent will automatically respond to user input prompts with default values.
    /// For interactive execution, use `execute_unified` instead.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - LLM provider fails to respond
    /// - Tool execution fails critically
    /// - Configuration is invalid
    /// - Maximum steps exceeded
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_sdk::{SageAgentSdk, UnifiedRunOptions};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let sdk = SageAgentSdk::new()?;
    /// let result = sdk.execute_non_interactive(
    ///     "Run tests",
    ///     UnifiedRunOptions::default()
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute_non_interactive(
        &self,
        task_description: &str,
        options: UnifiedRunOptions,
    ) -> SageResult<ExecutionResult> {
        let opts = UnifiedRunOptions {
            non_interactive: true,
            ..options
        };
        let (future, _) = self.execute_unified(task_description, opts)?;
        future.await
    }
}
