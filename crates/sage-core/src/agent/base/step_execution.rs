//! Single step execution logic

use crate::agent::{AgentState, AgentStep};
use crate::config::model::Config;
use crate::error::SageResult;
use crate::llm::client::LlmClient;
use crate::llm::messages::LlmMessage;
use crate::tools::executor::ToolExecutor;
use crate::tools::types::ToolSchema;
use crate::trajectory::recorder::TrajectoryRecorder;
use crate::ui::{AnimationManager, DisplayManager};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::instrument;

use super::llm_interaction::execute_llm_call;
use super::tool_execution::execute_tools;

/// Execute a single agent step
#[instrument(skip(llm_client, tool_executor, animation_manager, trajectory_recorder, config, messages, tools), fields(step_number = %step_number))]
pub(super) async fn execute_step(
    step_number: u32,
    messages: &[LlmMessage],
    tools: &[ToolSchema],
    llm_client: &mut LlmClient,
    tool_executor: &ToolExecutor,
    animation_manager: &mut AnimationManager,
    trajectory_recorder: &Option<Arc<Mutex<TrajectoryRecorder>>>,
    config: &Config,
) -> SageResult<AgentStep> {
    // Execute LLM call
    let mut step = execute_llm_call(
        step_number,
        messages,
        tools,
        llm_client,
        animation_manager,
        trajectory_recorder,
        config,
    )
    .await?;

    // Execute tools if any
    step = execute_tools(step, animation_manager, tool_executor).await?;

    // Check if task is completed
    if let Some(response) = &step.llm_response {
        if response.indicates_completion() {
            tracing::info!("step indicates task completion");
            step.state = AgentState::Completed;
            DisplayManager::print_separator("Task Completed", "green");
        }
    }

    step.complete();
    Ok(step)
}
