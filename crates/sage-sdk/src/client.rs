//! SDK client implementation

use sage_core::{
    agent::{Agent, AgentExecution, base::BaseAgent},
    config::{loader::load_config_with_overrides, model::Config},
    error::SageResult,
    tools::executor::ToolExecutorBuilder,
    trajectory::recorder::TrajectoryRecorder,
    types::TaskMetadata,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

// Import and re-export outcome types
pub use sage_core::agent::{ExecutionError, ExecutionErrorKind, ExecutionOutcome};
use sage_tools::get_default_tools;

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
            let mut params = sage_core::config::model::ModelParameters::default();
            params.model = model.to_string();
            if let Some(key) = api_key {
                params.api_key = Some(key.to_string());
            }
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

    /// Set maximum steps
    pub fn with_max_steps(mut self, max_steps: u32) -> Self {
        self.config.max_steps = max_steps;
        self
    }

    /// Run a task
    pub async fn run(&self, task_description: &str) -> SageResult<ExecutionResult> {
        self.run_with_options(task_description, RunOptions::default())
            .await
    }

    /// Run a task with options
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
        agent.set_max_steps(options.max_steps.unwrap_or(self.config.max_steps));

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
            agent.set_trajectory_recorder(Arc::new(Mutex::new(recorder)));
            Some(path)
        } else {
            None
        };

        // Execute the task - now returns ExecutionOutcome
        let outcome = agent.execute_task(task).await?;

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

    /// Continue an existing execution with a new user message
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
