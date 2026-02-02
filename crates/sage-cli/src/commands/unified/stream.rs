//! Stream JSON output for the unified command

use sage_core::agent::{ExecutionOutcome, UnifiedExecutor};
use sage_core::config::Config;
use sage_core::error::{SageError, SageResult};
use sage_core::output::{CostInfo, OutputEvent, OutputFormat, OutputWriter};
use sage_core::trajectory::SessionRecorder;
use sage_core::types::TaskMetadata;
use std::io::stdout;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::args::UnifiedArgs;

/// Execute task with streaming JSON output (for SDK/programmatic use)
pub async fn execute_stream_json(
    args: UnifiedArgs,
    mut executor: UnifiedExecutor,
    config: Config,
    working_dir: PathBuf,
) -> SageResult<()> {
    // Create stream JSON writer
    let mut writer = OutputWriter::new(stdout(), OutputFormat::StreamJson);

    // Emit start event
    writer
        .write_event(&OutputEvent::system("Sage Agent starting"))
        .ok();

    // Get task description - required for stream mode
    let task_description = match args.task {
        Some(task) => {
            if let Ok(task_path) = std::path::Path::new(&task).canonicalize() {
                if task_path.is_file() {
                    writer
                        .write_event(&OutputEvent::system(&format!(
                            "Loading task from file: {}",
                            task_path.display()
                        )))
                        .ok();
                    tokio::fs::read_to_string(&task_path)
                        .await
                        .map_err(|e| SageError::config(format!("Failed to read task file: {e}")))?
                } else {
                    task
                }
            } else {
                task
            }
        }
        None => {
            writer
                .write_event(&OutputEvent::error("No task provided for stream mode"))
                .ok();
            return Err(SageError::config(
                "Stream JSON mode requires a task. Use: sage --stream-json \"your task\"",
            ));
        }
    };

    // Emit task received event
    writer
        .write_event(&OutputEvent::system(&format!(
            "Task: {}",
            &task_description[..task_description.len().min(100)]
        )))
        .ok();

    // Set up session recording
    let session_recorder = if config.trajectory.is_enabled() {
        match SessionRecorder::new(&working_dir) {
            Ok(recorder) => {
                let recorder = Arc::new(Mutex::new(recorder));
                executor.set_session_recorder(recorder.clone());
                Some(recorder)
            }
            Err(_) => None,
        }
    } else {
        None
    };

    // Create task metadata
    let task = TaskMetadata::new(&task_description, &working_dir.display().to_string());

    // Execute the task
    let start_time = std::time::Instant::now();
    let outcome = executor.execute(task).await;
    let duration = start_time.elapsed();

    // Get session ID if available
    let session_id = if let Some(recorder) = &session_recorder {
        Some(recorder.lock().await.session_id().to_string())
    } else {
        None
    };

    // Emit result based on outcome
    match outcome {
        Ok(ref execution_outcome) => {
            let execution = execution_outcome.execution();
            let mut cost = CostInfo::new(
                execution.total_usage.prompt_tokens as usize,
                execution.total_usage.completion_tokens as usize,
            );
            if let Some(cache_read) = execution.total_usage.cache_read_input_tokens {
                cost = cost.with_cache_read(cache_read as usize);
            }
            if let Some(cache_creation) = execution.total_usage.cache_creation_input_tokens {
                cost = cost.with_cache_creation(cache_creation as usize);
            }

            let result_content = match execution_outcome {
                ExecutionOutcome::Success(_) => execution
                    .final_result
                    .clone()
                    .unwrap_or_else(|| "Task completed successfully".to_string()),
                ExecutionOutcome::Failed { error, .. } => {
                    format!("Error: {}", error.message)
                }
                ExecutionOutcome::Interrupted { .. } => "Task interrupted by user".to_string(),
                ExecutionOutcome::MaxStepsReached { .. } => "Task reached maximum steps".to_string(),
                ExecutionOutcome::UserCancelled { .. } => "Task cancelled by user".to_string(),
                ExecutionOutcome::NeedsUserInput { last_response, .. } => {
                    format!("Waiting for input: {}", last_response)
                }
            };

            let result_event = match OutputEvent::result(&result_content) {
                OutputEvent::Result(mut e) => {
                    e.duration_ms = duration.as_millis() as u64;
                    e.cost = Some(cost);
                    if let Some(id) = session_id {
                        e.session_id = Some(id);
                    }
                    OutputEvent::Result(e)
                }
                _ => unreachable!(),
            };

            writer.write_event(&result_event).ok();
        }
        Err(ref e) => {
            writer
                .write_event(&OutputEvent::error(e.to_string()))
                .ok();
        }
    }

    outcome.map(|_| ())
}
