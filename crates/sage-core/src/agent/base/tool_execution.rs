//! Tool execution handling

use crate::agent::{AgentState, AgentStep};
use crate::error::{SageError, SageResult};
use crate::interrupt::global_interrupt_manager;
use crate::tools::executor::ToolExecutor;
use crate::trajectory::SessionRecorder;
use crate::ui::animation::AnimationState;
use crate::ui::{AnimationManager, DisplayManager};
use std::sync::Arc;
use tokio::select;
use tokio::sync::Mutex;

use super::tool_display::display_tool_actions;

/// Execute tools and update step
pub(super) async fn execute_tools(
    mut step: AgentStep,
    animation_manager: &mut AnimationManager,
    tool_executor: &ToolExecutor,
    session_recorder: &Option<Arc<Mutex<SessionRecorder>>>,
) -> SageResult<AgentStep> {
    let llm_response = step.llm_response.as_ref().unwrap();

    if llm_response.tool_calls.is_empty() {
        return Ok(step);
    }

    tracing::info!(
        tool_count = llm_response.tool_calls.len(),
        "executing tools"
    );
    step.state = AgentState::ToolExecution;

    // Print tool execution separator
    DisplayManager::print_separator("Tool Execution", "cyan");

    // Show tool actions
    display_tool_actions(&llm_response.tool_calls);

    // Execute tools with timing and animation
    let tool_start_time = std::time::Instant::now();

    // Start tool execution animation
    animation_manager
        .start_animation(AnimationState::ExecutingTools, "Executing tools", "cyan")
        .await;

    // Get cancellation token for interrupt handling
    let cancellation_token = global_interrupt_manager().lock().cancellation_token();

    // Record tool calls before execution
    if let Some(recorder) = session_recorder {
        for tool_call in &llm_response.tool_calls {
            let input = serde_json::to_value(&tool_call.arguments).unwrap_or_default();
            let _ = recorder
                .lock()
                .await
                .record_tool_call(&tool_call.name, input)
                .await;
        }
    }

    // Execute tools with interrupt support
    let tool_results = select! {
        results = tool_executor.execute_tools(&llm_response.tool_calls) => {
            results
        }
        _ = cancellation_token.cancelled() => {
            // Stop animation on interruption
            animation_manager.stop_animation().await;
            return Err(SageError::agent("Task interrupted during tool execution"));
        }
    };

    let tool_duration = tool_start_time.elapsed();

    // Stop animation and show timing if significant
    animation_manager.stop_animation().await;
    if tool_duration.as_millis() > 1000 {
        DisplayManager::print_timing("Tools", tool_duration);
    }

    // Record tool results and show output
    for result in &tool_results {
        // Record tool result
        if let Some(recorder) = session_recorder {
            let _ = recorder
                .lock()
                .await
                .record_tool_result(
                    &result.tool_name,
                    result.success,
                    result.output.clone(),
                    result.error.clone(),
                    result.execution_time_ms.unwrap_or(0),
                )
                .await;
        }

        // Show tool results briefly
        if !result.success {
            println!(
                "Error: {}",
                result.error.as_deref().unwrap_or("Unknown error")
            );
        } else if let Some(output) = &result.output {
            // Only show output for certain tools or if it's short
            if result.tool_name == "task_done" || output.len() < 100 {
                println!("{}", output.trim());
            }
        }
    }

    step = step.with_tool_results(tool_results);
    Ok(step)
}
