//! Basic run execution methods

use crate::client::{ExecutionResult, RunOptions, SageAgentSDK};
use sage_core::{
    agent::{ExecutionMode, ExecutionOptions, UnifiedExecutor},
    error::SageResult,
    input::{InputChannel, InputResponse},
    trajectory::recorder::TrajectoryRecorder,
    types::TaskMetadata,
};
use sage_tools::get_default_tools;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

impl SageAgentSDK {
    /// Run a task with default options.
    ///
    /// Executes the task in interactive mode with default settings.
    /// User input prompts will be handled via stdin.
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
    /// use sage_sdk::SageAgentSDK;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let sdk = SageAgentSDK::new()?;
    /// let result = sdk.run("Write a hello world program in Rust").await?;
    ///
    /// if result.is_success() {
    ///     println!("Task completed successfully");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn run(&self, task_description: &str) -> SageResult<ExecutionResult> {
        self.run_with_options(task_description, RunOptions::default())
            .await
    }

    /// Run a task with custom options.
    ///
    /// Provides fine-grained control over execution behavior including working
    /// directory, step limits, and trajectory recording.
    ///
    /// This method uses the unified execution loop internally, which properly
    /// blocks on user input when ask_user_question is called.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - LLM provider fails to respond
    /// - Tool execution fails critically
    /// - Configuration is invalid
    /// - Maximum steps exceeded
    /// - Working directory does not exist
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sage_sdk::{SageAgentSDK, RunOptions};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let sdk = SageAgentSDK::new()?;
    /// let options = RunOptions::new()
    ///     .with_max_steps(50)
    ///     .with_trajectory(true);
    ///
    /// let result = sdk.run_with_options("Refactor the code", options).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn run_with_options(
        &self,
        task_description: &str,
        options: RunOptions,
    ) -> SageResult<ExecutionResult> {
        // Create task metadata
        let working_dir = options
            .working_directory
            .or_else(|| self.config.working_directory.clone())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        let task = TaskMetadata::new(task_description, &working_dir.to_string_lossy());

        // Set up execution options for unified executor
        let max_steps = options.max_steps.or(self.config.max_steps);
        let exec_options = ExecutionOptions::default()
            .with_mode(ExecutionMode::interactive())
            .with_max_steps(max_steps);

        // Create unified executor
        let mut executor = UnifiedExecutor::with_options(self.config.clone(), exec_options)?;

        // Register default tools
        executor.register_tools(get_default_tools());

        // Initialize sub-agent support
        if let Err(e) = executor.init_subagent_support() {
            tracing::warn!("Failed to initialize sub-agent support: {}", e);
        }

        // Set up input channel for interactive mode
        let (input_channel, mut input_handle) = InputChannel::new(16);
        executor.set_input_channel(input_channel);

        // Spawn background task to handle user input from stdin
        let input_task = tokio::spawn(async move {
            use std::io::Write;
            while let Some(request) = input_handle.request_rx.recv().await {
                print!("> ");
                let _ = std::io::stdout().flush();

                let input_result = tokio::task::spawn_blocking(|| {
                    let mut input = String::new();
                    match std::io::stdin().read_line(&mut input) {
                        Ok(_) => Some(input),
                        Err(_) => None,
                    }
                })
                .await;

                match input_result {
                    Ok(Some(input)) => {
                        let content = input.trim().to_string();
                        let cancelled = content.to_lowercase() == "cancel"
                            || content.to_lowercase() == "quit"
                            || content.to_lowercase() == "exit";

                        let response = if cancelled {
                            InputResponse::cancelled(request.id)
                        } else {
                            InputResponse::text(request.id, content)
                        };

                        if input_handle.respond(response).await.is_err() {
                            break;
                        }
                    }
                    _ => {
                        let _ = input_handle
                            .respond(InputResponse::cancelled(request.id))
                            .await;
                        break;
                    }
                }
            }
        });

        // Set up trajectory recording if requested
        let trajectory_path = if options.enable_trajectory || self.trajectory_path.is_some() {
            let path = options
                .trajectory_path
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

        // Execute the task
        let outcome = executor.execute(task).await?;

        // Clean up input task
        input_task.abort();

        Ok(ExecutionResult::new(
            outcome,
            trajectory_path,
            self.config.clone(),
        ))
    }
}
