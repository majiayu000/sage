//! Trajectory recording implementation

use crate::agent::AgentStep;
use crate::error::{SageError, SageResult};
use crate::trajectory::storage::{FileStorage, TrajectoryStorage};
use crate::types::TaskMetadata;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::instrument;

/// Complete trajectory record - matches Python version format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryRecord {
    /// Unique identifier for this trajectory
    pub id: crate::types::Id,
    /// Task description
    pub task: String,
    /// Start time in ISO format
    pub start_time: String,
    /// End time in ISO format
    pub end_time: String,
    /// LLM provider used
    pub provider: String,
    /// Model name used
    pub model: String,
    /// Maximum steps allowed (None = unlimited)
    pub max_steps: Option<u32>,
    /// LLM interactions
    pub llm_interactions: Vec<LlmInteractionRecord>,
    /// Agent execution steps
    pub agent_steps: Vec<AgentStepRecord>,
    /// Whether task completed successfully
    pub success: bool,
    /// Final result or output
    pub final_result: Option<String>,
    /// Total execution time in seconds
    pub execution_time: f64,
}

/// LLM interaction record - matches Python version format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmInteractionRecord {
    /// Timestamp of the interaction
    pub timestamp: String,
    /// Provider used
    pub provider: String,
    /// Model used
    pub model: String,
    /// Input messages
    pub input_messages: Vec<serde_json::Value>,
    /// Response from LLM
    pub response: LlmResponseRecord,
    /// Tools available during interaction
    pub tools_available: Option<Vec<String>>,
}

/// Deprecated: Use `LlmInteractionRecord` instead
#[deprecated(since = "0.2.0", note = "Use `LlmInteractionRecord` instead")]
pub type LLMInteractionRecord = LlmInteractionRecord;

/// LLM response record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponseRecord {
    /// Response content
    pub content: String,
    /// Model that generated the response
    pub model: Option<String>,
    /// Finish reason
    pub finish_reason: Option<String>,
    /// Token usage
    pub usage: Option<TokenUsageRecord>,
    /// Tool calls made
    pub tool_calls: Option<Vec<serde_json::Value>>,
}

/// Deprecated: Use `LlmResponseRecord` instead
#[deprecated(since = "0.2.0", note = "Use `LlmResponseRecord` instead")]
pub type LLMResponseRecord = LlmResponseRecord;

/// Token usage record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageRecord {
    /// Input tokens
    pub input_tokens: u32,
    /// Output tokens
    pub output_tokens: u32,
    /// Cache creation input tokens (for some providers)
    pub cache_creation_input_tokens: Option<u32>,
    /// Cache read input tokens (for some providers)
    pub cache_read_input_tokens: Option<u32>,
    /// Reasoning tokens (for some providers)
    pub reasoning_tokens: Option<u32>,
}

/// Agent step record - matches Python version format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStepRecord {
    /// Step number
    pub step_number: u32,
    /// Timestamp
    pub timestamp: String,
    /// Agent state
    pub state: String,
    /// LLM messages sent in this step
    pub llm_messages: Option<Vec<serde_json::Value>>,
    /// LLM response received
    pub llm_response: Option<LlmResponseRecord>,
    /// Tool calls made
    pub tool_calls: Option<Vec<serde_json::Value>>,
    /// Tool results received
    pub tool_results: Option<Vec<serde_json::Value>>,
    /// Agent reflection
    pub reflection: Option<String>,
    /// Error message if any
    pub error: Option<String>,
}

/// Trajectory recorder for capturing agent execution
pub struct TrajectoryRecorder {
    storage: Arc<dyn TrajectoryStorage>,
    current_record: Arc<Mutex<Option<TrajectoryRecord>>>,
    auto_save: bool,
    save_interval_steps: usize,
    steps_since_save: usize,
    start_time: Option<DateTime<Utc>>,
}

impl TrajectoryRecorder {
    /// Create a new trajectory recorder with file storage
    pub fn new<P: AsRef<Path>>(path: P) -> SageResult<Self> {
        let storage = Arc::new(FileStorage::new(path)?);
        Ok(Self {
            storage,
            current_record: Arc::new(Mutex::new(None)),
            auto_save: true,
            save_interval_steps: 5,
            steps_since_save: 0,
            start_time: None,
        })
    }

    /// Create a trajectory recorder with custom storage
    pub fn with_storage(storage: Arc<dyn TrajectoryStorage>) -> Self {
        Self {
            storage,
            current_record: Arc::new(Mutex::new(None)),
            auto_save: true,
            save_interval_steps: 5,
            steps_since_save: 0,
            start_time: None,
        }
    }

    /// Start recording a new trajectory
    #[instrument(skip(self, task), fields(provider = %provider, model = %model, max_steps = ?max_steps))]
    pub async fn start_recording(
        &mut self,
        task: TaskMetadata,
        provider: String,
        model: String,
        max_steps: Option<u32>,
    ) -> SageResult<()> {
        let start_time = Utc::now();
        self.start_time = Some(start_time);

        let record = TrajectoryRecord {
            id: uuid::Uuid::new_v4(),
            task: task.description.clone(),
            start_time: start_time.to_rfc3339(),
            end_time: String::new(), // Will be filled when recording ends
            provider,
            model,
            max_steps,
            llm_interactions: Vec::new(),
            agent_steps: Vec::new(),
            success: false,
            final_result: None,
            execution_time: 0.0,
        };

        let mut current = self.current_record.lock().await;
        *current = Some(record);

        Ok(())
    }

    /// Record an agent step
    #[instrument(skip(self, step), fields(step_number = step.step_number))]
    pub async fn record_step(&mut self, step: AgentStep) -> SageResult<()> {
        let mut current = self.current_record.lock().await;

        if let Some(record) = current.as_mut() {
            // Convert LLM response if present
            let llm_response = step.llm_response.as_ref().map(|resp| LlmResponseRecord {
                content: resp.content.clone(),
                model: resp.model.clone(),
                finish_reason: resp.finish_reason.clone(),
                usage: resp.usage.as_ref().map(|u| TokenUsageRecord {
                    input_tokens: u.prompt_tokens,
                    output_tokens: u.completion_tokens,
                    cache_creation_input_tokens: u.cache_creation_input_tokens,
                    cache_read_input_tokens: u.cache_read_input_tokens,
                    reasoning_tokens: None,
                }),
                tool_calls: if resp.tool_calls.is_empty() {
                    None
                } else {
                    Some(
                        resp.tool_calls
                            .iter()
                            .map(|call| serde_json::to_value(call).unwrap_or_default())
                            .collect(),
                    )
                },
            });

            // Convert AgentStep to AgentStepRecord
            let step_record = AgentStepRecord {
                step_number: step.step_number,
                timestamp: Utc::now().to_rfc3339(),
                state: format!("{:?}", step.state),
                llm_messages: step.llm_messages.clone(),
                llm_response,
                tool_calls: if step.tool_calls.is_empty() {
                    None
                } else {
                    Some(
                        step.tool_calls
                            .iter()
                            .map(|call| serde_json::to_value(call).unwrap_or_default())
                            .collect(),
                    )
                },
                tool_results: if step.tool_results.is_empty() {
                    None
                } else {
                    Some(
                        step.tool_results
                            .iter()
                            .map(|result| serde_json::to_value(result).unwrap_or_default())
                            .collect(),
                    )
                },
                reflection: step.reflection.clone(),
                error: step.error.clone(),
            };

            record.agent_steps.push(step_record);
            self.steps_since_save += 1;

            // Auto-save if enabled and interval reached
            if self.auto_save && self.steps_since_save >= self.save_interval_steps {
                self.save_current(record).await?;
                self.steps_since_save = 0;
            }
        } else {
            return Err(SageError::agent("No active recording session"));
        }

        Ok(())
    }

    /// Finalize the recording
    #[instrument(skip(self, final_result), fields(success = success))]
    pub async fn finalize_recording(
        &mut self,
        success: bool,
        final_result: Option<String>,
    ) -> SageResult<()> {
        let mut current = self.current_record.lock().await;

        if let Some(record) = current.as_mut() {
            let end_time = Utc::now();
            record.end_time = end_time.to_rfc3339();
            record.success = success;
            record.final_result = final_result;

            // Calculate execution time
            if let Some(start_time) = self.start_time {
                record.execution_time = (end_time - start_time).num_milliseconds() as f64 / 1000.0;
            }

            // Final save
            self.save_current(record).await?;

            // Clear current record
            *current = None;
        }

        Ok(())
    }

    /// Record an LLM interaction
    pub async fn record_llm_interaction(
        &mut self,
        provider: String,
        model: String,
        input_messages: Vec<serde_json::Value>,
        response: LlmResponseRecord,
        tools_available: Option<Vec<String>>,
    ) -> SageResult<()> {
        let mut current = self.current_record.lock().await;

        if let Some(record) = current.as_mut() {
            let interaction = LlmInteractionRecord {
                timestamp: Utc::now().to_rfc3339(),
                provider: provider.clone(),
                model: model.clone(),
                input_messages: input_messages.clone(),
                response: response.clone(),
                tools_available: tools_available.clone(),
            };

            record.llm_interactions.push(interaction);

            // Also update the latest agent step with LLM messages and response
            if let Some(latest_step) = record.agent_steps.last_mut() {
                latest_step.llm_messages = Some(input_messages);
                latest_step.llm_response = Some(response);
            }
        }

        Ok(())
    }

    /// Save the current record
    async fn save_current(&self, record: &TrajectoryRecord) -> SageResult<()> {
        self.storage.save(record).await
    }

    /// Get the current trajectory record
    pub async fn get_current_record(&self) -> Option<TrajectoryRecord> {
        self.current_record.lock().await.clone()
    }

    /// Set auto-save settings
    pub fn set_auto_save(&mut self, enabled: bool, interval_steps: usize) {
        self.auto_save = enabled;
        self.save_interval_steps = interval_steps;
    }

    /// Get the trajectory file path (if using file storage)
    pub fn get_trajectory_path(&self) -> Option<PathBuf> {
        // Try to downcast to FileStorage
        self.storage
            .as_any()
            .downcast_ref::<FileStorage>()
            .map(|file_storage| file_storage.path().to_path_buf())
    }

    /// Load a trajectory from storage
    pub async fn load_trajectory(&self, id: String) -> SageResult<Option<TrajectoryRecord>> {
        let uuid = uuid::Uuid::parse_str(&id)
            .map_err(|e| SageError::config(format!("Invalid trajectory ID '{}': {}", id, e)))?;
        self.storage.load(uuid).await
    }

    /// List all trajectories
    pub async fn list_trajectories(&self) -> SageResult<Vec<String>> {
        let ids = self.storage.list().await?;
        Ok(ids.iter().map(|id| id.to_string()).collect())
    }

    /// Delete a trajectory
    pub async fn delete_trajectory(&self, id: String) -> SageResult<()> {
        let uuid = uuid::Uuid::parse_str(&id)
            .map_err(|e| SageError::config(format!("Invalid trajectory ID '{}': {}", id, e)))?;
        self.storage.delete(uuid).await
    }

    /// Search trajectories by criteria
    pub async fn search_trajectories(
        &self,
        criteria: SearchCriteria,
    ) -> SageResult<Vec<TrajectoryRecord>> {
        let all_ids = self.storage.list().await?;
        let mut results = Vec::new();

        for id in all_ids {
            if let Some(record) = self.storage.load(id).await? {
                if criteria.matches(&record) {
                    results.push(record);
                }
            }
        }

        Ok(results)
    }

    /// Get trajectory statistics
    pub async fn get_statistics(&self) -> SageResult<TrajectoryStatistics> {
        let ids = self.storage.list().await?;
        let mut stats = TrajectoryStatistics::default();

        for id in ids {
            if let Some(record) = self.storage.load(id).await? {
                stats.total_trajectories += 1;
                if record.success {
                    stats.successful_trajectories += 1;
                } else {
                    stats.failed_trajectories += 1;
                }
                stats.total_steps += record.agent_steps.len();

                // Sum token usage from LLM interactions
                for interaction in &record.llm_interactions {
                    if let Some(usage) = &interaction.response.usage {
                        stats.total_tokens += (usage.input_tokens + usage.output_tokens) as usize;
                    }
                }

                // Add execution time
                stats.total_execution_time +=
                    chrono::Duration::milliseconds((record.execution_time * 1000.0) as i64);
            }
        }

        Ok(stats)
    }
}

/// Search criteria for trajectories
#[derive(Debug, Clone)]
pub struct SearchCriteria {
    /// Filter by task description (substring match)
    pub task_description: Option<String>,
    /// Filter by success status
    pub success: Option<bool>,
    /// Filter by date range
    pub date_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    /// Filter by minimum steps
    pub min_steps: Option<usize>,
    /// Filter by maximum steps
    pub max_steps: Option<usize>,
    /// Filter by provider used
    pub provider: Option<String>,
}

impl SearchCriteria {
    /// Check if a trajectory record matches the criteria
    pub fn matches(&self, record: &TrajectoryRecord) -> bool {
        if let Some(desc) = &self.task_description {
            if !record.task.contains(desc) {
                return false;
            }
        }

        if let Some(success) = self.success {
            if record.success != success {
                return false;
            }
        }

        // TODO: Fix date range filtering after trajectory refactor
        if self.date_range.is_some() {
            // Skip date filtering for now
        }

        if let Some(min_steps) = self.min_steps {
            if record.agent_steps.len() < min_steps {
                return false;
            }
        }

        if let Some(max_steps) = self.max_steps {
            if record.agent_steps.len() > max_steps {
                return false;
            }
        }

        true
    }
}

/// Statistics about trajectories
#[derive(Debug, Clone, Default)]
pub struct TrajectoryStatistics {
    /// Total number of trajectories
    pub total_trajectories: usize,
    /// Number of successful trajectories
    pub successful_trajectories: usize,
    /// Number of failed trajectories
    pub failed_trajectories: usize,
    /// Total number of steps across all trajectories
    pub total_steps: usize,
    /// Total tokens used across all trajectories
    pub total_tokens: usize,
    /// Total execution time across all trajectories
    pub total_execution_time: chrono::Duration,
}
