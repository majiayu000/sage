//! SDK client implementation

use sage_core::{
    agent::{
        Agent, AgentExecution, ExecutionMode, ExecutionOptions, UnifiedExecutor, base::BaseAgent,
    },
    config::{loader::load_config_with_overrides, model::Config},
    error::SageResult,
    input::{InputChannel, InputChannelHandle, InputResponse},
    tools::executor::ToolExecutorBuilder,
    trajectory::recorder::TrajectoryRecorder,
    types::TaskMetadata,
};
use sage_tools::get_default_tools;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

// Import and re-export outcome types
pub use sage_core::agent::{ExecutionError, ExecutionErrorKind, ExecutionOutcome};
pub use sage_core::input::InputRequest;

/// SDK client for Sage Agent
pub struct SageAgentSDK {
    config: Config,
    trajectory_path: Option<PathBuf>,
}

impl SageAgentSDK {
    /// Create a new SDK instance with default configuration
    pub fn new() -> SageResult<Self> {
        let config = load_config_with_overrides(None, HashMap::new())?;
        Ok(Self {
            config,
            trajectory_path: None,
        })
    }

    /// Create SDK instance with custom configuration
    pub fn with_config(config: Config) -> Self {
        Self {
            config,
            trajectory_path: None,
        }
    }

    /// Create SDK instance with configuration file
    pub fn with_config_file<P: AsRef<std::path::Path>>(config_file: P) -> SageResult<Self> {
        let config_path = config_file.as_ref();
        tracing::info!("Loading SDK config from: {}", config_path.display());

        let config = load_config_with_overrides(
            Some(config_file.as_ref().to_str().unwrap()),
            HashMap::new(),
        )?;

        tracing::info!(
            "SDK config loaded - provider: {}, model: {}",
            config.get_default_provider(),
            config
                .default_model_parameters()
                .map(|p| p.model.clone())
                .unwrap_or_else(|_| "unknown".to_string())
        );

        Ok(Self {
            config,
            trajectory_path: None,
        })
    }

    /// Set trajectory recording path
    pub fn with_trajectory_path<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.trajectory_path = Some(path.into());
        self
    }

    /// Set provider and model
    pub fn with_provider_and_model(
        mut self,
        provider: &str,
        model: &str,
        api_key: Option<&str>,
    ) -> SageResult<Self> {
        // Update configuration
        if let Some(params) = self.config.model_providers.get_mut(provider) {
            params.model = model.to_string();
            if let Some(key) = api_key {
                params.api_key = Some(key.to_string());
            }
        } else {
            let params = sage_core::config::model::ModelParameters {
                model: model.to_string(),
                api_key: api_key.map(|k| k.to_string()),
                ..Default::default()
            };
            self.config
                .model_providers
                .insert(provider.to_string(), params);
        }

        self.config.default_provider = provider.to_string();
        Ok(self)
    }

    /// Set working directory
    pub fn with_working_directory<P: Into<PathBuf>>(mut self, working_dir: P) -> Self {
        self.config.working_directory = Some(working_dir.into());
        self
    }

    /// Set maximum steps (None = unlimited)
    pub fn with_max_steps(mut self, max_steps: Option<u32>) -> Self {
        self.config.max_steps = max_steps;
        self
    }

    /// Set a specific step limit
    pub fn with_step_limit(mut self, limit: u32) -> Self {
        self.config.max_steps = Some(limit);
        self
    }

    /// Run a task
    pub async fn run(&self, task_description: &str) -> SageResult<ExecutionResult> {
        self.run_with_options(task_description, RunOptions::default())
            .await
    }

    /// Run a task with options
    ///
    /// This method now uses the unified execution loop internally, which properly
    /// blocks on user input when ask_user_question is called.
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
        // Use options.max_steps if provided, otherwise fall back to config.max_steps
        let max_steps = options.max_steps.or(self.config.max_steps);
        let exec_options = ExecutionOptions::default()
            .with_mode(ExecutionMode::interactive())
            .with_max_steps(max_steps);

        // Create unified executor
        let mut executor = UnifiedExecutor::with_options(self.config.clone(), exec_options)?;

        // Register default tools - CRITICAL: without tools the AI cannot do anything!
        executor.register_tools(get_default_tools());

        // Initialize sub-agent support so Task tool can execute Explore/Plan agents
        if let Err(e) = executor.init_subagent_support() {
            tracing::warn!("Failed to initialize sub-agent support: {}", e);
        }

        // Set up input channel for interactive mode - handles ask_user_question
        let (input_channel, mut input_handle) = InputChannel::new(16);
        executor.set_input_channel(input_channel);

        // Spawn background task to handle user input from stdin
        let input_task = tokio::spawn(async move {
            use std::io::Write;
            while let Some(request) = input_handle.request_rx.recv().await {
                // The question is already printed by UnifiedExecutor
                // Just show a prompt and read input
                print!("> ");
                let _ = std::io::stdout().flush();

                // Use blocking task for stdin
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

        // Execute the task using unified executor
        let outcome = executor.execute(task).await?;

        // Clean up input task
        input_task.abort();

        Ok(ExecutionResult {
            outcome,
            trajectory_path,
            config_used: self.config.clone(),
        })
    }

    /// Get the current configuration
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Validate the current configuration
    pub fn validate_config(&self) -> SageResult<()> {
        self.config.validate()
    }

    /// Get the current SDK API version
    ///
    /// Returns the semantic version of the SDK's public API.
    ///
    /// # Example
    ///
    /// ```
    /// use sage_sdk::SageAgentSDK;
    ///
    /// let sdk = SageAgentSDK::new().unwrap();
    /// let version = sdk.api_version();
    /// println!("SDK API Version: {}", version);
    /// ```
    pub fn api_version(&self) -> crate::version::Version {
        crate::version::API_VERSION
    }

    /// Get version information string
    ///
    /// Returns a formatted string with SDK version details.
    ///
    /// # Example
    ///
    /// ```
    /// use sage_sdk::SageAgentSDK;
    ///
    /// let sdk = SageAgentSDK::new().unwrap();
    /// println!("{}", sdk.version_info());
    /// ```
    pub fn version_info(&self) -> String {
        crate::version::version_info()
    }

    /// Check if a client version is compatible with this SDK
    ///
    /// Returns `true` if the specified client version can safely use this SDK.
    ///
    /// # Example
    ///
    /// ```
    /// use sage_sdk::{SageAgentSDK, version::Version};
    ///
    /// let sdk = SageAgentSDK::new().unwrap();
    /// let client_version = Version::new(0, 1, 0);
    /// assert!(sdk.is_compatible_with(&client_version));
    /// ```
    pub fn is_compatible_with(&self, client_version: &crate::version::Version) -> bool {
        crate::version::is_compatible(client_version)
    }

    /// Continue an existing execution with a new user message
    ///
    /// **Deprecated**: This method uses the old exit-resume pattern.
    /// Consider using `execute_unified` with an InputChannel instead.
    #[deprecated(
        since = "0.2.0",
        note = "Use execute_unified with InputChannel for better user interaction handling"
    )]
    pub async fn continue_execution(
        &self,
        execution: &mut AgentExecution,
        user_message: &str,
    ) -> SageResult<()> {
        // Create agent
        let mut agent = BaseAgent::new(self.config.clone())?;

        // Set up tool executor with default tools
        let tool_executor = ToolExecutorBuilder::new()
            .with_tools(get_default_tools())
            .with_max_execution_time(std::time::Duration::from_secs(
                self.config.tools.max_execution_time,
            ))
            .with_parallel_execution(self.config.tools.allow_parallel_execution)
            .build();

        agent.set_tool_executor(tool_executor);

        // Set up trajectory recording if enabled
        if let Some(trajectory_path) = &self.trajectory_path {
            let recorder = Arc::new(Mutex::new(TrajectoryRecorder::new(
                trajectory_path.clone(),
            )?));
            agent.set_trajectory_recorder(recorder);
        }

        // Continue the execution
        agent.continue_execution(execution, user_message).await
    }

    /// Execute a task using the unified execution loop (Claude Code style)
    ///
    /// This method uses a unified execution model where:
    /// - The execution loop never exits for user input
    /// - User questions block inline via InputChannel
    /// - Returns an InputChannelHandle for handling user prompts
    ///
    /// # Example
    /// ```ignore
    /// let sdk = SageAgentSDK::new()?;
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

        // Register default tools - CRITICAL: without tools the AI cannot do anything!
        executor.register_tools(get_default_tools());

        // Initialize sub-agent support so Task tool can execute Explore/Plan agents
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
            Ok(ExecutionResult {
                outcome,
                trajectory_path,
                config_used,
            })
        };

        Ok((execution_future, input_handle))
    }

    /// Execute a task in non-interactive mode using the unified executor
    ///
    /// This is a simpler API for cases where no user interaction is needed.
    /// For interactive execution, use `execute_unified` instead.
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

impl Default for SageAgentSDK {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            config: Config::default(),
            trajectory_path: None,
        })
    }
}

/// Options for running tasks
#[derive(Debug, Clone, Default)]
pub struct RunOptions {
    /// Working directory for the task
    pub working_directory: Option<PathBuf>,
    /// Maximum number of steps
    pub max_steps: Option<u32>,
    /// Enable trajectory recording
    pub enable_trajectory: bool,
    /// Custom trajectory file path
    pub trajectory_path: Option<PathBuf>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl RunOptions {
    /// Create new run options
    pub fn new() -> Self {
        Self::default()
    }

    /// Set working directory
    pub fn with_working_directory<P: Into<PathBuf>>(mut self, working_dir: P) -> Self {
        self.working_directory = Some(working_dir.into());
        self
    }

    /// Set maximum steps
    pub fn with_max_steps(mut self, max_steps: u32) -> Self {
        self.max_steps = Some(max_steps);
        self
    }

    /// Enable trajectory recording
    pub fn with_trajectory(mut self, enabled: bool) -> Self {
        self.enable_trajectory = enabled;
        self
    }

    /// Set trajectory file path
    pub fn with_trajectory_path<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.trajectory_path = Some(path.into());
        self.enable_trajectory = true;
        self
    }

    /// Add metadata
    pub fn with_metadata<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<serde_json::Value>,
    {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Options for running tasks with the unified executor
#[derive(Debug, Clone, Default)]
pub struct UnifiedRunOptions {
    /// Working directory for the task
    pub working_directory: Option<PathBuf>,
    /// Maximum number of steps
    pub max_steps: Option<u32>,
    /// Enable trajectory recording
    pub enable_trajectory: bool,
    /// Custom trajectory file path
    pub trajectory_path: Option<PathBuf>,
    /// Non-interactive mode (auto-respond to user questions)
    pub non_interactive: bool,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl UnifiedRunOptions {
    /// Create new unified run options
    pub fn new() -> Self {
        Self::default()
    }

    /// Set working directory
    pub fn with_working_directory<P: Into<PathBuf>>(mut self, working_dir: P) -> Self {
        self.working_directory = Some(working_dir.into());
        self
    }

    /// Set maximum steps
    pub fn with_max_steps(mut self, max_steps: u32) -> Self {
        self.max_steps = Some(max_steps);
        self
    }

    /// Enable trajectory recording
    pub fn with_trajectory(mut self, enabled: bool) -> Self {
        self.enable_trajectory = enabled;
        self
    }

    /// Set trajectory file path
    pub fn with_trajectory_path<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.trajectory_path = Some(path.into());
        self.enable_trajectory = true;
        self
    }

    /// Set non-interactive mode
    pub fn with_non_interactive(mut self, non_interactive: bool) -> Self {
        self.non_interactive = non_interactive;
        self
    }

    /// Add metadata
    pub fn with_metadata<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<serde_json::Value>,
    {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Result of task execution
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// The execution outcome (success, failure, interrupted, or max steps)
    pub outcome: ExecutionOutcome,
    /// Path to trajectory file (if recorded)
    pub trajectory_path: Option<PathBuf>,
    /// Configuration used for execution
    pub config_used: Config,
}

impl ExecutionResult {
    /// Check if the execution was successful
    pub fn is_success(&self) -> bool {
        self.outcome.is_success()
    }

    /// Check if the execution failed
    pub fn is_failed(&self) -> bool {
        self.outcome.is_failed()
    }

    /// Check if the execution was interrupted
    pub fn is_interrupted(&self) -> bool {
        self.outcome.is_interrupted()
    }

    /// Get the execution outcome
    pub fn outcome(&self) -> &ExecutionOutcome {
        &self.outcome
    }

    /// Get the underlying execution (regardless of outcome)
    pub fn execution(&self) -> &AgentExecution {
        self.outcome.execution()
    }

    /// Get the error if the execution failed
    pub fn error(&self) -> Option<&ExecutionError> {
        self.outcome.error()
    }

    /// Get the final result message
    pub fn final_result(&self) -> Option<&str> {
        self.outcome.execution().final_result.as_deref()
    }

    /// Get execution statistics
    pub fn statistics(&self) -> sage_core::agent::execution::ExecutionStatistics {
        self.outcome.execution().statistics()
    }

    /// Get a summary of the execution
    pub fn summary(&self) -> String {
        self.outcome.execution().summary()
    }

    /// Get all tool calls made during execution
    pub fn tool_calls(&self) -> Vec<&sage_core::tools::types::ToolCall> {
        self.outcome.execution().all_tool_calls()
    }

    /// Get all tool results from execution
    pub fn tool_results(&self) -> Vec<&sage_core::tools::types::ToolResult> {
        self.outcome.execution().all_tool_results()
    }

    /// Get steps that had errors
    pub fn error_steps(&self) -> Vec<&sage_core::agent::AgentStep> {
        self.outcome.execution().error_steps()
    }

    /// Get the trajectory file path if available
    pub fn trajectory_path(&self) -> Option<&PathBuf> {
        self.trajectory_path.as_ref()
    }

    /// Get a user-friendly status message
    pub fn status_message(&self) -> &'static str {
        self.outcome.status_message()
    }

    /// Get the status icon for CLI display
    pub fn status_icon(&self) -> &'static str {
        self.outcome.status_icon()
    }
}
