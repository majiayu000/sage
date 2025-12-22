//! Trajectory replay implementation
//!
//! Provides functionality to load and replay recorded agent trajectories.
//! Useful for debugging, testing, and analyzing agent behavior.

use crate::error::{SageError, SageResult};
use crate::trajectory::recorder::{AgentStepRecord, TrajectoryRecord};
use crate::trajectory::storage::TrajectoryStorage;
use crate::tools::types::ToolResult;
use std::path::Path;
use std::sync::Arc;
use tokio::fs;
use tracing::{debug, info, instrument, warn};

/// Replay mode determines how the trajectory is replayed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayMode {
    /// Just display the recorded steps without re-execution
    DryRun,
    /// Re-execute tool calls and compare with recorded results
    LiveReplay,
    /// Step through one step at a time
    Interactive,
}

/// Result of replaying a single step
#[derive(Debug, Clone)]
pub struct StepReplayResult {
    /// Step number
    pub step_number: u32,
    /// Whether the step was replayed successfully
    pub success: bool,
    /// The recorded tool calls
    pub recorded_tool_calls: Vec<serde_json::Value>,
    /// The recorded tool results
    pub recorded_tool_results: Vec<serde_json::Value>,
    /// The replayed tool results (if live replay)
    pub replayed_tool_results: Option<Vec<ToolResult>>,
    /// Whether the replayed results match the recorded results
    pub results_match: Option<bool>,
    /// Any differences detected
    pub differences: Vec<String>,
}

/// Summary of a trajectory replay
#[derive(Debug, Clone)]
pub struct ReplaySummary {
    /// Trajectory ID
    pub trajectory_id: String,
    /// Original task description
    pub task: String,
    /// Total steps in trajectory
    pub total_steps: usize,
    /// Steps successfully replayed
    pub steps_replayed: usize,
    /// Steps that matched recorded results
    pub steps_matched: usize,
    /// Total tool calls made
    pub total_tool_calls: usize,
    /// Original execution time
    pub original_execution_time: f64,
    /// Replay execution time
    pub replay_execution_time: Option<f64>,
    /// Overall success
    pub success: bool,
    /// Errors encountered
    pub errors: Vec<String>,
}

/// Trajectory replayer for loading and replaying agent trajectories
pub struct TrajectoryReplayer {
    storage: Option<Arc<dyn TrajectoryStorage>>,
}

impl TrajectoryReplayer {
    /// Create a new trajectory replayer
    pub fn new() -> Self {
        Self { storage: None }
    }

    /// Create a trajectory replayer with storage backend
    pub fn with_storage(storage: Arc<dyn TrajectoryStorage>) -> Self {
        Self {
            storage: Some(storage),
        }
    }

    /// Load a trajectory from a file path
    #[instrument(skip(self), fields(path = %path.as_ref().display()))]
    pub async fn load_from_file<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> SageResult<TrajectoryRecord> {
        let path = path.as_ref();
        debug!("Loading trajectory from file: {:?}", path);

        if !path.exists() {
            return Err(SageError::config(format!(
                "Trajectory file not found: {:?}",
                path
            )));
        }

        let content = fs::read_to_string(path).await.map_err(|e| {
            SageError::config(format!("Failed to read trajectory file {:?}: {}", path, e))
        })?;

        let record: TrajectoryRecord = serde_json::from_str(&content).map_err(|e| {
            SageError::config(format!("Failed to parse trajectory JSON: {}", e))
        })?;

        info!(
            "Loaded trajectory: {} steps, task: {}",
            record.agent_steps.len(),
            record.task
        );

        Ok(record)
    }

    /// Load a trajectory by ID from storage
    #[instrument(skip(self))]
    pub async fn load_by_id(&self, id: uuid::Uuid) -> SageResult<Option<TrajectoryRecord>> {
        let storage = self.storage.as_ref().ok_or_else(|| {
            SageError::config("No storage backend configured for replayer")
        })?;

        storage.load(id).await
    }

    /// List all available trajectories in a directory
    #[instrument(skip(self), fields(dir = %dir.as_ref().display()))]
    pub async fn list_trajectories<P: AsRef<Path>>(
        &self,
        dir: P,
    ) -> SageResult<Vec<TrajectoryInfo>> {
        let dir = dir.as_ref();
        debug!("Listing trajectories in: {:?}", dir);

        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut trajectories = Vec::new();
        let mut entries = fs::read_dir(dir).await.map_err(|e| {
            SageError::config(format!("Failed to read directory {:?}: {}", dir, e))
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            SageError::config(format!("Failed to read directory entry: {}", e))
        })? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                match self.load_from_file(&path).await {
                    Ok(record) => {
                        trajectories.push(TrajectoryInfo {
                            path: path.clone(),
                            id: record.id.to_string(),
                            task: record.task.clone(),
                            start_time: record.start_time.clone(),
                            end_time: record.end_time.clone(),
                            steps: record.agent_steps.len(),
                            success: record.success,
                            execution_time: record.execution_time,
                        });
                    }
                    Err(e) => {
                        warn!("Skipping invalid trajectory file {:?}: {}", path, e);
                    }
                }
            }
        }

        // Sort by start time (newest first)
        trajectories.sort_by(|a, b| b.start_time.cmp(&a.start_time));

        Ok(trajectories)
    }

    /// Get a summary of a trajectory without full replay
    pub fn summarize(&self, record: &TrajectoryRecord) -> ReplaySummary {
        let total_tool_calls: usize = record
            .agent_steps
            .iter()
            .map(|step| {
                step.tool_calls
                    .as_ref()
                    .map(|calls| calls.len())
                    .unwrap_or(0)
            })
            .sum();

        ReplaySummary {
            trajectory_id: record.id.to_string(),
            task: record.task.clone(),
            total_steps: record.agent_steps.len(),
            steps_replayed: 0,
            steps_matched: 0,
            total_tool_calls,
            original_execution_time: record.execution_time,
            replay_execution_time: None,
            success: record.success,
            errors: Vec::new(),
        }
    }

    /// Analyze a trajectory step by step (dry run mode)
    #[instrument(skip(self, record))]
    pub fn analyze_steps(&self, record: &TrajectoryRecord) -> Vec<StepAnalysis> {
        record
            .agent_steps
            .iter()
            .map(|step| self.analyze_step(step))
            .collect()
    }

    /// Analyze a single step
    fn analyze_step(&self, step: &AgentStepRecord) -> StepAnalysis {
        let tool_calls = step.tool_calls.clone().unwrap_or_default();
        let tool_results = step.tool_results.clone().unwrap_or_default();

        // Extract tool names from tool calls
        let tool_names: Vec<String> = tool_calls
            .iter()
            .filter_map(|call| {
                call.get("name")
                    .and_then(|n| n.as_str())
                    .map(|s| s.to_string())
            })
            .collect();

        // Check for errors in results
        let has_errors = tool_results.iter().any(|result| {
            result
                .get("success")
                .and_then(|s| s.as_bool())
                .map(|s| !s)
                .unwrap_or(false)
        });

        StepAnalysis {
            step_number: step.step_number,
            timestamp: step.timestamp.clone(),
            state: step.state.clone(),
            tool_names,
            tool_call_count: tool_calls.len(),
            tool_result_count: tool_results.len(),
            has_llm_response: step.llm_response.is_some(),
            has_errors,
            error_message: step.error.clone(),
            reflection: step.reflection.clone(),
        }
    }

    /// Get the LLM response content for a step
    pub fn get_step_llm_response(&self, step: &AgentStepRecord) -> Option<String> {
        step.llm_response.as_ref().map(|r| r.content.clone())
    }

    /// Get tool calls for a step as structured data
    pub fn get_step_tool_calls(&self, step: &AgentStepRecord) -> Vec<ToolCallInfo> {
        step.tool_calls
            .as_ref()
            .map(|calls| {
                calls
                    .iter()
                    .filter_map(|call| {
                        let name = call.get("name")?.as_str()?.to_string();
                        let id = call
                            .get("id")
                            .and_then(|i| i.as_str())
                            .unwrap_or("unknown")
                            .to_string();
                        let arguments = call.get("arguments").cloned();

                        Some(ToolCallInfo {
                            id,
                            name,
                            arguments,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get tool results for a step
    pub fn get_step_tool_results(&self, step: &AgentStepRecord) -> Vec<ToolResultInfo> {
        step.tool_results
            .as_ref()
            .map(|results| {
                results
                    .iter()
                    .filter_map(|result| {
                        let call_id = result
                            .get("call_id")
                            .and_then(|i| i.as_str())
                            .unwrap_or("unknown")
                            .to_string();
                        let tool_name = result
                            .get("tool_name")
                            .and_then(|n| n.as_str())
                            .unwrap_or("unknown")
                            .to_string();
                        let success = result
                            .get("success")
                            .and_then(|s| s.as_bool())
                            .unwrap_or(false);
                        let output = result
                            .get("output")
                            .and_then(|o| o.as_str())
                            .map(|s| s.to_string());

                        Some(ToolResultInfo {
                            call_id,
                            tool_name,
                            success,
                            output,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Calculate token usage statistics from the trajectory
    pub fn calculate_token_usage(&self, record: &TrajectoryRecord) -> TokenUsageStats {
        let mut total_input = 0u32;
        let mut total_output = 0u32;
        let mut total_cache_creation = 0u32;
        let mut total_cache_read = 0u32;

        for interaction in &record.llm_interactions {
            if let Some(usage) = &interaction.response.usage {
                total_input += usage.input_tokens;
                total_output += usage.output_tokens;
                total_cache_creation += usage.cache_creation_input_tokens.unwrap_or(0);
                total_cache_read += usage.cache_read_input_tokens.unwrap_or(0);
            }
        }

        TokenUsageStats {
            total_input_tokens: total_input,
            total_output_tokens: total_output,
            total_tokens: total_input + total_output,
            cache_creation_tokens: total_cache_creation,
            cache_read_tokens: total_cache_read,
            interactions_count: record.llm_interactions.len(),
        }
    }
}

impl Default for TrajectoryReplayer {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about a trajectory file
#[derive(Debug, Clone)]
pub struct TrajectoryInfo {
    /// File path
    pub path: std::path::PathBuf,
    /// Trajectory ID
    pub id: String,
    /// Task description
    pub task: String,
    /// Start time
    pub start_time: String,
    /// End time
    pub end_time: String,
    /// Number of steps
    pub steps: usize,
    /// Whether successful
    pub success: bool,
    /// Execution time in seconds
    pub execution_time: f64,
}

/// Analysis of a single step
#[derive(Debug, Clone)]
pub struct StepAnalysis {
    /// Step number
    pub step_number: u32,
    /// Timestamp
    pub timestamp: String,
    /// Agent state
    pub state: String,
    /// Tool names used
    pub tool_names: Vec<String>,
    /// Number of tool calls
    pub tool_call_count: usize,
    /// Number of tool results
    pub tool_result_count: usize,
    /// Whether step has LLM response
    pub has_llm_response: bool,
    /// Whether any tool results were errors
    pub has_errors: bool,
    /// Error message if any
    pub error_message: Option<String>,
    /// Agent reflection if any
    pub reflection: Option<String>,
}

/// Information about a tool call
#[derive(Debug, Clone)]
pub struct ToolCallInfo {
    /// Call ID
    pub id: String,
    /// Tool name
    pub name: String,
    /// Arguments as JSON
    pub arguments: Option<serde_json::Value>,
}

/// Information about a tool result
#[derive(Debug, Clone)]
pub struct ToolResultInfo {
    /// Call ID this result is for
    pub call_id: String,
    /// Tool name
    pub tool_name: String,
    /// Whether successful
    pub success: bool,
    /// Output content
    pub output: Option<String>,
}

/// Token usage statistics
#[derive(Debug, Clone, Default)]
pub struct TokenUsageStats {
    /// Total input tokens
    pub total_input_tokens: u32,
    /// Total output tokens
    pub total_output_tokens: u32,
    /// Total tokens (input + output)
    pub total_tokens: u32,
    /// Cache creation tokens
    pub cache_creation_tokens: u32,
    /// Cache read tokens
    pub cache_read_tokens: u32,
    /// Number of LLM interactions
    pub interactions_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replayer_creation() {
        let replayer = TrajectoryReplayer::new();
        assert!(replayer.storage.is_none());
    }

    #[test]
    fn test_token_usage_stats_default() {
        let stats = TokenUsageStats::default();
        assert_eq!(stats.total_tokens, 0);
        assert_eq!(stats.interactions_count, 0);
    }
}
