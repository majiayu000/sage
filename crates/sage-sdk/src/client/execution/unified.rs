//! Unified executor methods

use crate::client::{ExecutionResult, SageAgentSdk, UnifiedRunOptions};
use sage_core::{
    agent::{ExecutionMode, ExecutionOptions, UnifiedExecutor},
    error::SageResult,
    input::{InputChannel, InputChannelHandle},
    mcp::build_mcp_registry_from_config,
    types::TaskMetadata,
};
use sage_tools::get_default_tools;
use std::path::PathBuf;

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
        let exec_options = ExecutionOptions::default()
            .with_mode(mode)
            .with_max_steps(max_steps)
            .with_working_directory(&working_dir);

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

        // Session recording - always enabled, stored in ~/.sage/projects/{cwd}/
        if self.config.trajectory.is_enabled() {
            if let Some(recorder) = sage_core::trajectory::init_session_recorder(&working_dir) {
                executor.set_session_recorder(recorder);
            }
        }

        let config_used = self.config.clone();
        let config_for_mcp = self.config.clone();

        // Create the future that will execute the task
        let execution_future = async move {
            // Load MCP tools if MCP is enabled
            tracing::debug!("Checking MCP configuration: enabled={}", config_for_mcp.mcp.enabled);
            if config_for_mcp.mcp.enabled {
                tracing::info!("MCP is enabled, building MCP registry...");
                match build_mcp_registry_from_config(&config_for_mcp).await {
                    Ok(mcp_registry) => {
                        let mcp_tools = mcp_registry.as_tools().await;
                        tracing::info!("Loaded {} MCP tools from {} servers",
                            mcp_tools.len(),
                            mcp_registry.server_names().len());

                        if !mcp_tools.is_empty() {
                            executor.register_tools(mcp_tools);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to build MCP registry: {}", e);
                    }
                }
            } else {
                tracing::debug!("MCP is disabled in configuration");
            }

            let outcome = executor.execute(task).await?;
            Ok(ExecutionResult::new(
                outcome,
                None, // No longer returning trajectory path
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
