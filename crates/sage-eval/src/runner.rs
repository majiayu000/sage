//! Eval runners and harness orchestration.

use crate::metrics::evaluate_tool_metrics;
use crate::report::{EvalReport, TaskReport};
use crate::task::{EvalSuite, EvalTask, OfflineTrace};
use crate::trace::write_jsonl;
use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use sage_core::trajectory::{SessionEntry, SessionReplayer};
use sage_sdk::{SageAgentSdk, UnifiedRunOptions};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvalRunnerKind {
    Offline,
    Sdk,
}

#[derive(Debug, Clone)]
pub struct EvalRunOptions {
    pub output_dir: PathBuf,
}

impl EvalRunOptions {
    pub fn new(output_dir: impl Into<PathBuf>) -> Self {
        Self {
            output_dir: output_dir.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TaskRunOutput {
    pub final_output: String,
    pub trajectory_path: PathBuf,
    pub workspace_path: PathBuf,
    pub entries: Vec<SessionEntry>,
}

#[async_trait]
pub trait EvalRunner {
    async fn run_task(&self, task: &EvalTask, run_dir: &Path) -> Result<TaskRunOutput>;
}

#[derive(Debug, Clone, Default)]
pub struct OfflineRunner;

#[async_trait]
impl EvalRunner for OfflineRunner {
    async fn run_task(&self, task: &EvalTask, run_dir: &Path) -> Result<TaskRunOutput> {
        let offline = task
            .offline
            .as_ref()
            .with_context(|| format!("task '{}' has no offline trace", task.id))?;
        let workspace_path = prepare_workspace(task, run_dir).await?;
        let entries = offline_entries(task, &workspace_path, offline);
        let trajectory_path = run_dir.join("trajectory.jsonl");
        write_jsonl(&trajectory_path, &entries).await?;

        Ok(TaskRunOutput {
            final_output: offline.final_output.clone(),
            trajectory_path,
            workspace_path,
            entries,
        })
    }
}

pub struct SdkAgentRunner {
    sdk: SageAgentSdk,
    max_steps: Option<u32>,
}

impl SdkAgentRunner {
    pub fn new(sdk: SageAgentSdk) -> Self {
        Self {
            sdk,
            max_steps: Some(12),
        }
    }

    pub fn with_max_steps(mut self, max_steps: u32) -> Self {
        self.max_steps = Some(max_steps);
        self
    }
}

#[async_trait]
impl EvalRunner for SdkAgentRunner {
    async fn run_task(&self, task: &EvalTask, run_dir: &Path) -> Result<TaskRunOutput> {
        let workspace_path = prepare_workspace(task, run_dir).await?;
        let mut options = UnifiedRunOptions::new()
            .with_working_directory(&workspace_path)
            .with_non_interactive(true);
        if let Some(max_steps) = self.max_steps {
            options = options.with_max_steps(max_steps);
        }

        let result = self
            .sdk
            .execute_non_interactive(&task.prompt, options)
            .await
            .with_context(|| format!("SDK eval task '{}' failed to run", task.id))?;
        let session = SessionReplayer::list_sessions(&workspace_path)
            .await?
            .into_iter()
            .next()
            .with_context(|| format!("no trajectory produced for task '{}'", task.id))?;
        let entries = SessionReplayer::load(&session.file_path).await?;

        Ok(TaskRunOutput {
            final_output: result.final_result().unwrap_or_default().to_string(),
            trajectory_path: session.file_path,
            workspace_path,
            entries,
        })
    }
}

pub async fn run_suite<R>(
    suite: &EvalSuite,
    runner: &R,
    options: &EvalRunOptions,
) -> Result<EvalReport>
where
    R: EvalRunner + Sync,
{
    tokio::fs::create_dir_all(&options.output_dir).await?;
    let mut reports = Vec::new();
    for task in &suite.tasks {
        let run_dir = options.output_dir.join(&task.id);
        if tokio::fs::try_exists(&run_dir).await? {
            tokio::fs::remove_dir_all(&run_dir).await?;
        }
        tokio::fs::create_dir_all(&run_dir).await?;

        let output = runner.run_task(task, &run_dir).await?;
        let assertion_results = evaluate_assertions(task, &output).await?;
        let passed = !assertion_results.is_empty() && assertion_results.iter().all(|value| *value);
        let tool_metrics = evaluate_tool_metrics(task, &output.entries);
        reports.push(TaskReport {
            task_id: task.id.clone(),
            passed,
            assertion_results,
            trajectory_path: output.trajectory_path,
            tool_metrics,
        });
    }

    Ok(EvalReport::new(suite.name.clone(), reports))
}

async fn prepare_workspace(task: &EvalTask, run_dir: &Path) -> Result<PathBuf> {
    let workspace_path = run_dir.join("workspace");
    tokio::fs::create_dir_all(&workspace_path).await?;
    for file in &task.workspace_files {
        file.write_to(&workspace_path).await?;
    }
    Ok(workspace_path)
}

async fn evaluate_assertions(task: &EvalTask, output: &TaskRunOutput) -> Result<Vec<bool>> {
    if task.assertions.is_empty() {
        bail!("task '{}' has no assertions", task.id);
    }
    let mut results = Vec::with_capacity(task.assertions.len());
    for assertion in &task.assertions {
        results.push(
            assertion
                .evaluate(&output.final_output, &output.workspace_path)
                .await?,
        );
    }
    Ok(results)
}

fn offline_entries(
    task: &EvalTask,
    workspace_path: &Path,
    offline: &OfflineTrace,
) -> Vec<SessionEntry> {
    let mut entries = Vec::new();
    let mut next_uuid = 1_u128;
    let session_id = deterministic_uuid(next_uuid);
    next_uuid += 1;

    entries.push(SessionEntry::SessionStart {
        session_id,
        task: task.prompt.clone(),
        provider: "offline".to_string(),
        model: "offline-deterministic".to_string(),
        cwd: workspace_path.to_string_lossy().to_string(),
        git_branch: None,
        timestamp: timestamp(0),
    });
    entries.push(SessionEntry::User {
        uuid: deterministic_uuid(next_uuid),
        parent_uuid: Some(session_id),
        content: serde_json::json!({"content": task.prompt}),
        timestamp: timestamp(1),
    });
    next_uuid += 1;

    for intent in &offline.tool_intents {
        entries.push(SessionEntry::ToolIntent {
            uuid: deterministic_uuid(next_uuid),
            parent_uuid: Some(session_id),
            tool_category: intent.tool_category.clone(),
            tool_name: intent.tool_name.clone(),
            reason: intent.reason.clone(),
            timestamp: timestamp(next_uuid),
        });
        next_uuid += 1;
    }

    for call in &offline.tool_calls {
        entries.push(SessionEntry::ToolCall {
            uuid: deterministic_uuid(next_uuid),
            parent_uuid: Some(session_id),
            tool_name: call.tool_name.clone(),
            tool_input: call.tool_input.clone(),
            timestamp: timestamp(next_uuid),
        });
        next_uuid += 1;
        entries.push(SessionEntry::ToolResult {
            uuid: deterministic_uuid(next_uuid),
            parent_uuid: Some(session_id),
            tool_name: call.tool_name.clone(),
            success: call.success,
            output: call.output.clone(),
            error: (!call.success).then(|| "offline tool call failed".to_string()),
            execution_time_ms: 0,
            timestamp: timestamp(next_uuid),
        });
        next_uuid += 1;
    }

    entries.push(SessionEntry::LlmResponse {
        uuid: deterministic_uuid(next_uuid),
        parent_uuid: Some(session_id),
        content: offline.final_output.clone(),
        model: "offline-deterministic".to_string(),
        usage: None,
        tool_calls: None,
        timestamp: timestamp(next_uuid),
    });
    next_uuid += 1;
    entries.push(SessionEntry::SessionEnd {
        uuid: deterministic_uuid(next_uuid),
        parent_uuid: Some(session_id),
        success: true,
        final_result: Some(offline.final_output.clone()),
        total_steps: 1,
        execution_time_secs: 0.0,
        timestamp: timestamp(next_uuid),
    });

    entries
}

fn deterministic_uuid(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn timestamp(offset: u128) -> String {
    format!("1970-01-01T00:00:{:02}Z", offset.min(59))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{Assertion, ToolCallSpec, ToolIntentSpec, WorkspaceFile};

    fn suite() -> EvalSuite {
        EvalSuite {
            name: "offline-test".to_string(),
            tasks: vec![EvalTask {
                id: "read-marker".to_string(),
                prompt: "Read marker.txt".to_string(),
                required_tool_categories: vec!["file_read".to_string()],
                expected_tool_names: vec!["Read".to_string()],
                workspace_files: vec![WorkspaceFile {
                    path: PathBuf::from("marker.txt"),
                    content: "marker-value".to_string(),
                }],
                assertions: vec![Assertion::OutputContains {
                    value: "marker-value".to_string(),
                }],
                offline: Some(OfflineTrace {
                    final_output: "marker-value".to_string(),
                    tool_intents: vec![ToolIntentSpec {
                        tool_category: "file_read".to_string(),
                        tool_name: Some("Read".to_string()),
                        reason: "Need marker".to_string(),
                    }],
                    tool_calls: vec![ToolCallSpec {
                        tool_name: "Read".to_string(),
                        tool_input: serde_json::json!({"file_path": "marker.txt"}),
                        success: true,
                        output: Some("marker-value".to_string()),
                    }],
                }),
            }],
        }
    }

    #[tokio::test]
    async fn offline_runner_is_deterministic_for_pass_at_1_and_metrics() {
        let temp = tempfile::tempdir().unwrap();
        let suite = suite();
        let first = run_suite(
            &suite,
            &OfflineRunner,
            &EvalRunOptions::new(temp.path().join("first")),
        )
        .await
        .unwrap();
        let second = run_suite(
            &suite,
            &OfflineRunner,
            &EvalRunOptions::new(temp.path().join("second")),
        )
        .await
        .unwrap();

        assert_eq!(first.pass_at_1, 1.0);
        assert_eq!(first.pass_at_1, second.pass_at_1);
        assert_eq!(first.tool_metrics, second.tool_metrics);
        assert_eq!(first.tool_metrics.recognition_accuracy, Some(1.0));
        assert_eq!(first.tool_metrics.execution_accuracy, Some(1.0));
    }
}
