//! Evaluation executor for running tasks through the agent
//!
//! Integrates with UnifiedExecutor to run evaluation tasks and collect metrics.

use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::Utc;
use sage_core::agent::{ExecutionMode, ExecutionOptions, ExecutionOutcome, UnifiedExecutor};
use sage_core::config::load_config_from_file;
use sage_core::output::OutputMode;
use sage_core::types::TaskMetadata;
use sage_tools::get_default_tools_with_working_dir;
use tokio::time::timeout;

use super::{EvalConfig, TestHarness};
use crate::metrics::{EvalMetrics, MetricsAggregator, TaskResult, TaskStatus};
use crate::tasks::{EvalTask, TaskCategory, TaskLoader};

/// Callback for progress updates during evaluation
pub type ProgressCallback = Box<dyn Fn(EvalProgress) + Send + Sync>;

/// Progress update during evaluation
#[derive(Debug, Clone)]
pub struct EvalProgress {
    /// Current task index (0-based)
    pub current: usize,
    /// Total number of tasks
    pub total: usize,
    /// Current task ID
    pub task_id: String,
    /// Current task name
    pub task_name: String,
    /// Current attempt number
    pub attempt: u32,
    /// Status message
    pub message: String,
}

/// Executor for running evaluation tasks
pub struct EvalExecutor {
    /// Configuration
    config: EvalConfig,

    /// Task loader
    loader: TaskLoader,

    /// Progress callback
    progress_callback: Option<ProgressCallback>,

    /// Model name (extracted from config)
    model: String,

    /// Provider name
    provider: String,
}

impl EvalExecutor {
    /// Create a new evaluation executor
    pub fn new(config: EvalConfig) -> Result<Self> {
        let loader = TaskLoader::builtin();

        // Extract model/provider from sage config
        let (model, provider) = Self::extract_model_info(&config)?;

        Ok(Self {
            config,
            loader,
            progress_callback: None,
            model,
            provider,
        })
    }

    /// Create with a custom task loader
    pub fn with_loader(config: EvalConfig, loader: TaskLoader) -> Result<Self> {
        let (model, provider) = Self::extract_model_info(&config)?;

        Ok(Self {
            config,
            loader,
            progress_callback: None,
            model,
            provider,
        })
    }

    /// Set progress callback
    pub fn set_progress_callback(&mut self, callback: ProgressCallback) {
        self.progress_callback = Some(callback);
    }

    /// Extract model info from sage config
    fn extract_model_info(config: &EvalConfig) -> Result<(String, String)> {
        // Try to load sage config to get model info
        let config_path = &config.config_file;
        if config_path.exists() {
            let content = std::fs::read_to_string(config_path)
                .context("Failed to read sage config")?;
            let sage_config: serde_json::Value = serde_json::from_str(&content)
                .context("Failed to parse sage config")?;

            let provider = sage_config
                .get("default_provider")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            // Get model from the provider's config
            let model = sage_config
                .get("model_providers")
                .and_then(|providers| providers.get(&provider))
                .and_then(|p| p.get("model"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            Ok((model, provider))
        } else {
            // Try loading from default locations
            if let Some(home) = dirs::home_dir() {
                let global_config = home.join(".sage").join("config.json");
                if global_config.exists() {
                    let content = std::fs::read_to_string(&global_config)
                        .context("Failed to read global sage config")?;
                    let sage_config: serde_json::Value = serde_json::from_str(&content)
                        .context("Failed to parse global sage config")?;

                    let provider = sage_config
                        .get("default_provider")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();

                    let model = sage_config
                        .get("model_providers")
                        .and_then(|providers| providers.get(&provider))
                        .and_then(|p| p.get("model"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();

                    return Ok((model, provider));
                }
            }
            Ok(("unknown".to_string(), "unknown".to_string()))
        }
    }

    /// Run all evaluation tasks
    pub async fn run_all(&self) -> Result<EvalMetrics> {
        let tasks = self.load_tasks()?;
        self.run_tasks(tasks).await
    }

    /// Run tasks for specific categories
    pub async fn run_categories(&self, categories: &[TaskCategory]) -> Result<EvalMetrics> {
        let tasks = self.loader.load_categories(categories)?;
        self.run_tasks(tasks).await
    }

    /// Run a single task by ID
    pub async fn run_task(&self, task_id: &str) -> Result<Option<TaskResult>> {
        let task = self.loader.load_by_id(task_id)?;
        match task {
            Some(task) => {
                let results = self.run_tasks(vec![task]).await?;
                Ok(results.task_results.into_iter().next())
            }
            None => Ok(None),
        }
    }

    /// Load tasks based on configuration filters
    fn load_tasks(&self) -> Result<Vec<EvalTask>> {
        let mut tasks = if !self.config.task_ids.is_empty() {
            // Load specific tasks
            let mut selected = Vec::new();
            for id in &self.config.task_ids {
                if let Some(task) = self.loader.load_by_id(id)? {
                    selected.push(task);
                }
            }
            selected
        } else if !self.config.categories.is_empty() {
            // Load by category
            let categories: Vec<TaskCategory> = self
                .config
                .categories
                .iter()
                .filter_map(|c| match c.as_str() {
                    "code_generation" => Some(TaskCategory::CodeGeneration),
                    "code_editing" => Some(TaskCategory::CodeEditing),
                    "bug_fixing" => Some(TaskCategory::BugFixing),
                    "refactoring" => Some(TaskCategory::Refactoring),
                    "multi_file" => Some(TaskCategory::MultiFile),
                    _ => None,
                })
                .collect();
            self.loader.load_categories(&categories)?
        } else {
            // Load all
            self.loader.load_all()?
        };

        // Filter by tags if specified
        if !self.config.tags.is_empty() {
            tasks.retain(|t| t.tags.iter().any(|tag| self.config.tags.contains(tag)));
        }

        Ok(tasks)
    }

    /// Run a set of tasks and collect metrics
    async fn run_tasks(&self, tasks: Vec<EvalTask>) -> Result<EvalMetrics> {
        let total_tasks = tasks.len();
        let mut all_results = Vec::new();
        let start_time = std::time::Instant::now();

        for (index, task) in tasks.into_iter().enumerate() {
            // Run multiple attempts if configured
            for attempt in 1..=self.config.attempts {
                self.emit_progress(EvalProgress {
                    current: index,
                    total: total_tasks,
                    task_id: task.id.clone(),
                    task_name: task.name.clone(),
                    attempt,
                    message: format!("Running attempt {}/{}", attempt, self.config.attempts),
                });

                let result = self.run_single_task(&task, attempt).await;

                match result {
                    Ok(task_result) => {
                        let passed = task_result.passed();
                        all_results.push(task_result);

                        // If passed, no need for more attempts
                        if passed {
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            task_id = %task.id,
                            attempt = attempt,
                            error = %e,
                            "Task execution failed"
                        );

                        // Create error result
                        let mut error_result = TaskResult::new(
                            &task.id,
                            &task.name,
                            task.category,
                            task.difficulty,
                            TaskStatus::Error,
                        );
                        error_result.attempt = attempt;
                        error_result.error_message = Some(e.to_string());
                        all_results.push(error_result);

                        if !self.config.continue_on_failure {
                            break;
                        }
                    }
                }
            }

            if !self.config.continue_on_failure {
                // Check if last result was an error
                if let Some(last) = all_results.last() {
                    if matches!(last.status, TaskStatus::Error) {
                        break;
                    }
                }
            }
        }

        let total_time = start_time.elapsed().as_secs_f64();

        // Aggregate metrics
        let aggregator = MetricsAggregator::new(&self.model, &self.provider);
        let metrics = aggregator.aggregate(all_results, total_time);

        // Save results if configured
        if self.config.save_results {
            self.save_results(&metrics).await?;
        }

        Ok(metrics)
    }

    /// Run a single task
    async fn run_single_task(&self, task: &EvalTask, attempt: u32) -> Result<TaskResult> {
        let timeout_secs = self.config.effective_timeout(Some(task.timeout_secs));

        // Create test harness
        let mut harness = TestHarness::new(task.clone(), timeout_secs).await?;
        harness.start_attempt(attempt);

        // Get sandbox path for agent execution
        let sandbox_root = harness.sandbox_root().to_path_buf();

        // Execute the task using the agent
        let execution_result = timeout(
            harness.timeout(),
            self.execute_task_in_sandbox(task, &sandbox_root, &mut harness),
        )
        .await;

        let result = match execution_result {
            Ok(Ok(())) => {
                // Task completed, verify result
                harness.complete_attempt().await
            }
            Ok(Err(e)) => {
                // Task failed with error
                harness.complete_with_error(e.to_string())
            }
            Err(_) => {
                // Task timed out
                harness.complete_with_timeout()
            }
        };

        // Cleanup sandbox
        if self.config.cleanup_sandbox {
            harness.cleanup().await?;
        }

        Ok(result)
    }

    /// Execute a task in the sandbox environment using UnifiedExecutor
    async fn execute_task_in_sandbox(
        &self,
        task: &EvalTask,
        sandbox_root: &PathBuf,
        harness: &mut TestHarness,
    ) -> Result<()> {
        tracing::info!(
            task_id = %task.id,
            sandbox = %sandbox_root.display(),
            "Executing task in sandbox with UnifiedExecutor"
        );

        // Load sage configuration
        let config = if self.config.config_file.exists() {
            load_config_from_file(self.config.config_file.to_str().unwrap_or("sage_config.json"))
                .context("Failed to load sage config")?
        } else {
            sage_core::config::load_config().context("Failed to load default config")?
        };

        // Set up execution options - non-interactive mode for evaluation
        let options = ExecutionOptions::default()
            .with_mode(ExecutionMode::non_interactive())
            .with_step_limit(self.config.max_steps)
            .with_working_directory(sandbox_root);

        // Create the unified executor
        let mut executor = UnifiedExecutor::with_options(config, options)
            .map_err(|e| anyhow::anyhow!("Failed to create executor: {}", e))?;

        // Set silent output mode for evaluation
        executor.set_output_mode(OutputMode::Silent);

        // Register default tools with sandbox working directory
        let tools = get_default_tools_with_working_dir(sandbox_root);
        executor.register_tools(tools);

        // Initialize sub-agent support
        if let Err(e) = executor.init_subagent_support() {
            tracing::warn!("Failed to initialize sub-agent support: {}", e);
        }

        // Execute the task
        let task_meta = TaskMetadata::new(task.description.clone(), sandbox_root.to_string_lossy().to_string());
        let outcome = executor
            .execute(task_meta)
            .await
            .map_err(|e| anyhow::anyhow!("Task execution failed: {}", e))?;

        // Extract metrics from execution outcome
        let execution = match &outcome {
            ExecutionOutcome::Success(execution)
            | ExecutionOutcome::MaxStepsReached { execution }
            | ExecutionOutcome::Interrupted { execution }
            | ExecutionOutcome::UserCancelled { execution, .. }
            | ExecutionOutcome::NeedsUserInput { execution, .. } => execution,
            ExecutionOutcome::Failed { execution, error } => {
                tracing::warn!("Task execution failed: {}", error);
                execution
            }
        };

        // Record metrics from each step
        for step in &execution.steps {
            let input_tokens = step
                .llm_response
                .as_ref()
                .and_then(|r| r.usage.as_ref())
                .map(|u| u.prompt_tokens as u64)
                .unwrap_or(0);
            let output_tokens = step
                .llm_response
                .as_ref()
                .and_then(|r| r.usage.as_ref())
                .map(|u| u.completion_tokens as u64)
                .unwrap_or(0);

            harness.record_turn(input_tokens, output_tokens);

            // Record tool usage
            for tool_call in &step.tool_calls {
                harness.record_tool_use(&tool_call.name);
            }
        }

        // Shutdown executor gracefully
        if let Err(e) = executor.shutdown().await {
            tracing::warn!("Failed to shutdown executor: {}", e);
        }

        Ok(())
    }

    /// Save evaluation results to file
    async fn save_results(&self, metrics: &EvalMetrics) -> Result<()> {
        let output_dir = self
            .config
            .output_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from("."));

        tokio::fs::create_dir_all(&output_dir).await?;

        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("eval_results_{}.json", timestamp);
        let output_path = output_dir.join(filename);

        let json = serde_json::to_string_pretty(metrics)?;
        tokio::fs::write(&output_path, json).await?;

        tracing::info!("Saved evaluation results to {:?}", output_path);
        Ok(())
    }

    /// Emit progress update
    fn emit_progress(&self, progress: EvalProgress) {
        if let Some(callback) = &self.progress_callback {
            callback(progress);
        }
    }

    /// List available tasks
    pub fn list_tasks(&self) -> Result<Vec<(String, String, TaskCategory)>> {
        let tasks = self.load_tasks()?;
        Ok(tasks
            .into_iter()
            .map(|t| (t.id, t.name, t.category))
            .collect())
    }

    /// Get task count by category
    pub fn task_counts(&self) -> Result<std::collections::HashMap<TaskCategory, usize>> {
        self.loader.count_by_category()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_config_default() {
        let config = EvalConfig::default();
        assert_eq!(config.attempts, 1);
        assert_eq!(config.max_steps, 50);
    }

    #[tokio::test]
    async fn test_executor_creation() {
        let config = EvalConfig::default();
        let executor = EvalExecutor::new(config);
        // Should succeed even without a valid sage config
        assert!(executor.is_ok());
    }
}
