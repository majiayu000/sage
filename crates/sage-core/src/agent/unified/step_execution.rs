//! Single step execution logic

use crate::agent::{AgentState, AgentStep};
use crate::error::{SageError, SageResult};
use crate::interrupt::global_interrupt_manager;
use crate::llm::messages::LlmMessage;
use crate::tools::types::{ToolCall, ToolSchema};
use crate::ui::DisplayManager;
use colored::Colorize;
use tracing::instrument;

use super::event_manager::ExecutionEvent;
use super::permission_handler;
use super::tool_display;
use super::tool_orchestrator::ToolExecutionContext;
use super::UnifiedExecutor;

impl UnifiedExecutor {
    /// Execute a single step in the loop
    #[instrument(skip(self, messages, tool_schemas, task_scope), fields(step_number = %step_number))]
    pub(super) async fn execute_step(
        &mut self,
        step_number: u32,
        messages: &[LlmMessage],
        tool_schemas: &[ToolSchema],
        task_scope: &crate::interrupt::TaskScope,
    ) -> SageResult<(AgentStep, Vec<LlmMessage>)> {
        let mut step = AgentStep::new(step_number, AgentState::Thinking);

        // Check and auto-compact context if needed
        let mut working_messages = messages.to_vec();
        let compact_result = self
            .auto_compact
            .check_and_compact(&mut working_messages)
            .await?;

        if compact_result.was_compacted {
            tracing::info!(
                "Auto-compacted: {} -> {} msgs, saved {} tokens",
                compact_result.messages_before,
                compact_result.messages_after,
                compact_result.tokens_saved()
            );
        }

        // Emit events
        self.event_manager
            .emit(ExecutionEvent::StepStarted { step_number })
            .await;
        self.event_manager
            .emit(ExecutionEvent::ThinkingStarted { step_number })
            .await;

        // Record and execute LLM call
        self.session_manager
            .record_llm_request(&working_messages, tool_schemas)
            .await;

        let cancellation_token = global_interrupt_manager().lock().cancellation_token();
        let llm_response = match self
            .llm_orchestrator
            .stream_chat(&working_messages, Some(tool_schemas), cancellation_token)
            .await
        {
            Ok(response) => response,
            Err(e) => {
                self.event_manager.emit(ExecutionEvent::ThinkingStopped).await;
                return Err(e);
            }
        };

        self.event_manager.emit(ExecutionEvent::ThinkingStopped).await;

        // Record response
        let model = self.llm_orchestrator.model_name();
        self.session_manager
            .record_llm_response(&llm_response, model)
            .await;

        // Build step with messages and response
        let messages_json: Vec<serde_json::Value> = working_messages
            .iter()
            .map(|m| serde_json::to_value(m).unwrap_or_default())
            .collect();
        step = step
            .with_llm_messages(messages_json)
            .with_llm_response(llm_response.clone());

        let mut new_messages = working_messages;

        // Display assistant response
        if !llm_response.content.is_empty() {
            println!();
            println!("  {} {}", "󰚩".bright_cyan(), "AI Response".bright_white().bold());
            println!();
            for line in DisplayManager::render_markdown_lines(&llm_response.content) {
                println!("  {}", line);
            }
        }

        // Add assistant message
        if !llm_response.tool_calls.is_empty() || !llm_response.content.is_empty() {
            let mut assistant_msg = LlmMessage::assistant(&llm_response.content);
            if !llm_response.tool_calls.is_empty() {
                assistant_msg.tool_calls = Some(llm_response.tool_calls.clone());
            }
            new_messages.push(assistant_msg);
        }

        // Handle tool calls
        if !llm_response.tool_calls.is_empty() {
            self.handle_tool_calls(&mut step, &mut new_messages, &llm_response.tool_calls, task_scope)
                .await?;
        }

        // Check for completion
        let is_natural_end = matches!(
            llm_response.finish_reason.as_deref(),
            Some("end_turn") | Some("stop") | Some("STOP")
        );

        if is_natural_end && llm_response.tool_calls.is_empty() {
            tracing::info!(finish_reason = ?llm_response.finish_reason, "task completed");
            step.state = AgentState::Completed;
        }

        Ok((step, new_messages))
    }

    /// Handle tool call execution
    async fn handle_tool_calls(
        &mut self,
        step: &mut AgentStep,
        new_messages: &mut Vec<LlmMessage>,
        tool_calls: &[ToolCall],
        task_scope: &crate::interrupt::TaskScope,
    ) -> SageResult<()> {
        tracing::info!(tool_count = tool_calls.len(), "executing tools");

        println!();
        println!(
            "  {} {} ({})",
            "".bright_magenta(),
            "Executing tools".bright_white().bold(),
            tool_calls.len().to_string().dimmed()
        );

        let context = self.build_execution_context();

        for tool_call in tool_calls {
            if task_scope.is_cancelled() {
                self.event_manager.stop_animation().await;
                return Err(SageError::agent("Task interrupted during tool execution"));
            }

            let tool_result = self
                .execute_single_tool(tool_call, &context, task_scope)
                .await?;

            step.tool_results.push(tool_result.clone());
            new_messages.push(LlmMessage::tool(
                tool_result.output.clone().unwrap_or_default(),
                tool_call.id.clone(),
                Some(tool_call.name.clone()),
            ));
        }

        step.state = AgentState::ToolExecution;
        Ok(())
    }

    /// Build execution context for tool orchestrator
    fn build_execution_context(&self) -> ToolExecutionContext {
        let session_id = self
            .session_manager
            .current_session_id()
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.id.to_string());
        let working_dir = self
            .options
            .working_directory
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        ToolExecutionContext::new(session_id, working_dir)
    }

    /// Execute a single tool with the three-phase model
    async fn execute_single_tool(
        &mut self,
        tool_call: &ToolCall,
        context: &ToolExecutionContext,
        task_scope: &crate::interrupt::TaskScope,
    ) -> SageResult<crate::tools::types::ToolResult> {
        // Display and track
        tool_display::display_tool_start(&mut self.event_manager, tool_call).await;
        self.track_file_for_undo(tool_call).await;
        self.session_manager
            .record_tool_call(
                &tool_call.name,
                &serde_json::to_value(&tool_call.arguments).unwrap_or_default(),
            )
            .await;

        let tool_start_time = std::time::Instant::now();
        let cancel_token = task_scope.token().clone();

        // Phase 1: Pre-execution
        let pre_result = self
            .tool_orchestrator
            .pre_execution_phase(tool_call, context, cancel_token.clone())
            .await?;

        let tool_result = if let Some(reason) = pre_result.block_reason() {
            crate::tools::types::ToolResult::error(
                &tool_call.id,
                &tool_call.name,
                format!("Tool blocked by hook: {}", reason),
            )
        } else {
            // Phase 2: Execution
            self.execute_tool_phase(tool_call, cancel_token.clone()).await?
        };

        // Phase 3: Post-execution
        self.tool_orchestrator
            .post_execution_phase(tool_call, &tool_result, context, cancel_token)
            .await?;

        // Record and display result
        let duration_ms = tool_start_time.elapsed().as_millis() as u64;
        self.session_manager
            .record_tool_result(
                &tool_call.name,
                tool_result.success,
                tool_result.output.clone(),
                tool_result.error.clone(),
                duration_ms,
            )
            .await;
        tool_display::display_tool_result(&mut self.event_manager, &tool_result, duration_ms).await;

        Ok(tool_result)
    }

    /// Execute the tool (phase 2)
    async fn execute_tool_phase(
        &mut self,
        tool_call: &ToolCall,
        cancel_token: tokio_util::sync::CancellationToken,
    ) -> SageResult<crate::tools::types::ToolResult> {
        let requires_interaction = self
            .tool_orchestrator
            .requires_user_interaction(&tool_call.name);

        if requires_interaction && tool_call.name == "ask_user_question" {
            self.handle_ask_user_question(tool_call).await
        } else if requires_interaction {
            Ok(self.tool_orchestrator.execution_phase(tool_call, cancel_token).await)
        } else {
            Ok(permission_handler::execute_with_permission_check(
                &self.tool_orchestrator,
                &self.event_manager,
                tool_call,
                cancel_token,
            )
            .await)
        }
    }

    /// Track file for undo capability
    async fn track_file_for_undo(&mut self, tool_call: &ToolCall) {
        if crate::tools::names::is_file_modifying_tool(&tool_call.name) {
            if let Some(file_path) = tool_call
                .arguments
                .get("file_path")
                .or_else(|| tool_call.arguments.get("path"))
                .and_then(|v| v.as_str())
            {
                if let Err(e) = self.session_manager.track_file(file_path).await {
                    tracing::warn!(error = %e, file_path = %file_path, "Failed to track file");
                }
            }
        }
    }
}
