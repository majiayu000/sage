//! Basic run execution methods

use crate::client::{ExecutionResult, RunOptions, SageAgentSdk};
use sage_core::{
    agent::{ExecutionMode, ExecutionOptions, UnifiedExecutor},
    error::SageResult,
    input::{InputChannel, InputResponse},
    types::TaskMetadata,
};
use sage_tools::get_default_tools;
use std::path::PathBuf;

impl SageAgentSdk {
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
    /// use sage_sdk::SageAgentSdk;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let sdk = SageAgentSdk::new()?;
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
    /// directory, step limits, and session recording.
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
    /// use sage_sdk::{SageAgentSdk, RunOptions};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let sdk = SageAgentSdk::new()?;
    /// let options = RunOptions::new()
    ///     .with_max_steps(50);
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
            .with_max_steps(max_steps)
            .with_working_directory(&working_dir);

        // Create unified executor
        let mut executor = UnifiedExecutor::with_options(self.config.clone(), exec_options)?;

        // Register default tools
        let mut all_tools = get_default_tools();

        // Load MCP tools if MCP is enabled
        if self.config.mcp.enabled {
            tracing::info!("MCP is enabled, building MCP registry...");
            match sage_core::mcp::build_mcp_registry_from_config(&self.config).await {
                Ok(mcp_registry) => {
                    let mcp_tools = mcp_registry.as_tools().await;
                    tracing::info!(
                        "Loaded {} MCP tools from {} servers",
                        mcp_tools.len(),
                        mcp_registry.server_names().len()
                    );

                    if !mcp_tools.is_empty() {
                        all_tools.extend(mcp_tools);
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to build MCP registry: {}", e);
                }
            }
        }

        executor.register_tools(all_tools);

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

        // Session recording - always enabled, stored in ~/.sage/projects/{cwd}/
        if self.config.trajectory.is_enabled() {
            if let Some(recorder) = sage_core::trajectory::init_session_recorder(&working_dir) {
                executor.set_session_recorder(recorder);
            }
        }

        // Execute the task
        let outcome = executor.execute(task).await?;

        // Clean up input task
        input_task.abort();

        Ok(ExecutionResult::new(
            outcome,
            self.config.clone(),
        ))
    }
}
