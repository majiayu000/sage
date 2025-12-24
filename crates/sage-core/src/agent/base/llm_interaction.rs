//! LLM interaction handling

use crate::agent::{AgentState, AgentStep};
use crate::config::model::Config;
use crate::error::{SageError, SageResult};
use crate::interrupt::global_interrupt_manager;
use crate::llm::client::LlmClient;
use crate::llm::messages::LlmMessage;
use crate::tools::types::ToolSchema;
use crate::trajectory::{SessionRecorder, TokenUsage};
use crate::ui::animation::AnimationState;
use crate::ui::{AnimationManager, DisplayManager};
use std::sync::Arc;
use tokio::select;
use tokio::sync::Mutex;

use super::utils::is_markdown_content;

/// Execute LLM call and record interaction
pub(super) async fn execute_llm_call(
    step_number: u32,
    messages: &[LlmMessage],
    tools: &[ToolSchema],
    llm_client: &mut LlmClient,
    animation_manager: &mut AnimationManager,
    session_recorder: &Option<Arc<Mutex<SessionRecorder>>>,
    config: &Config,
) -> SageResult<AgentStep> {
    // Print step separator
    DisplayManager::print_separator(&format!("Step {} - AI Thinking", step_number), "blue");

    let mut step = AgentStep::new(step_number, AgentState::Thinking);

    // Record LLM request before sending
    if let Some(recorder) = session_recorder {
        let input_messages: Vec<serde_json::Value> = messages
            .iter()
            .map(|msg| serde_json::to_value(msg).unwrap_or_default())
            .collect();
        let tools_available: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();
        let _ = recorder
            .lock()
            .await
            .record_llm_request(input_messages, Some(tools_available))
            .await;
    }

    // Get LLM response with timing and animation
    let start_time = std::time::Instant::now();

    // Start thinking animation
    animation_manager
        .start_animation(AnimationState::Thinking, "Thinking", "blue")
        .await;

    // Get cancellation token for interrupt handling
    let cancellation_token = global_interrupt_manager().lock().cancellation_token();

    // Execute LLM call with interrupt support
    let llm_response = select! {
        response = llm_client.chat(messages, Some(tools)) => {
            response?
        }
        _ = cancellation_token.cancelled() => {
            // Stop animation on interruption
            animation_manager.stop_animation().await;
            return Err(SageError::agent("Task interrupted during LLM call"));
        }
    };

    let api_duration = start_time.elapsed();

    // Stop animation and show timing
    animation_manager.stop_animation().await;
    DisplayManager::print_timing("AI Response", api_duration);

    step = step.with_llm_response(llm_response.clone());

    // Record LLM response
    if let Some(recorder) = session_recorder {
        let model = config.default_model_parameters()?.model.clone();
        let usage = llm_response.usage.as_ref().map(|u| TokenUsage {
            input_tokens: u.prompt_tokens as u64,
            output_tokens: u.completion_tokens as u64,
            cache_read_tokens: None,
            cache_write_tokens: None,
        });
        let tool_calls = if llm_response.tool_calls.is_empty() {
            None
        } else {
            Some(
                llm_response
                    .tool_calls
                    .iter()
                    .map(|tc| serde_json::to_value(tc).unwrap_or_default())
                    .collect(),
            )
        };
        let _ = recorder
            .lock()
            .await
            .record_llm_response(&llm_response.content, &model, usage, tool_calls)
            .await;
    }

    // Show AI response with markdown rendering
    if !llm_response.content.is_empty() {
        if is_markdown_content(&llm_response.content) {
            println!("\nAI Response:");
            DisplayManager::print_markdown(&llm_response.content);
        } else {
            println!("\n{}", llm_response.content.trim());
        }
    }

    Ok(step)
}
