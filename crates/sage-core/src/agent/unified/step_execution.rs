//! Single step execution logic

use crate::agent::{AgentState, AgentStep};
use crate::error::{SageError, SageResult};
use crate::interrupt::global_interrupt_manager;
use crate::llm::messages::LlmMessage;
use crate::tools::types::{ToolCall, ToolSchema};
use crate::trajectory::TokenUsage;
use crate::ui::DisplayManager;
use crate::ui::animation::AnimationState;
use crate::ui::prompt::{PermissionChoice, PermissionDialogConfig, show_permission_dialog};
use colored::Colorize;
use tracing::instrument;

use super::event_manager::ExecutionEvent;
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

        // Check and auto-compact context if needed before LLM call
        let mut working_messages = messages.to_vec();
        let compact_result = self
            .auto_compact
            .check_and_compact(&mut working_messages)
            .await?;

        if compact_result.was_compacted {
            tracing::info!(
                "Auto-compacted context: {} -> {} messages, saved {} tokens",
                compact_result.messages_before,
                compact_result.messages_after,
                compact_result.tokens_saved()
            );
        }

        // Emit step started event
        self.event_manager
            .emit(ExecutionEvent::StepStarted { step_number })
            .await;

        // Emit thinking started event (handles animation internally)
        self.event_manager
            .emit(ExecutionEvent::ThinkingStarted { step_number })
            .await;

        // Record LLM request before sending
        if let Some(recorder) = self.session_manager.session_recorder() {
            let input_messages: Vec<serde_json::Value> = working_messages
                .iter()
                .map(|msg| serde_json::to_value(msg).unwrap_or_default())
                .collect();
            let tools_available: Vec<String> =
                tool_schemas.iter().map(|t| t.name.clone()).collect();
            if let Err(e) = recorder
                .lock()
                .await
                .record_llm_request(input_messages, Some(tools_available))
                .await
            {
                tracing::warn!(error = %e, "Failed to record LLM request (non-fatal)");
            }
        }

        // Get cancellation token for interrupt handling
        let cancellation_token = global_interrupt_manager().lock().cancellation_token();

        // Execute LLM call with interrupt support using the LLM orchestrator
        // Streaming keeps the connection alive, avoiding timeout issues with slow models
        // The orchestrator handles cancellation internally for faster response
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

        // Emit thinking stopped event
        self.event_manager.emit(ExecutionEvent::ThinkingStopped).await;

        // Record LLM response
        if let Some(recorder) = self.session_manager.session_recorder() {
            let model = self.llm_orchestrator.model_name().to_string();
            let usage = llm_response.usage.as_ref().map(|u| TokenUsage {
                input_tokens: u.prompt_tokens as u64,
                output_tokens: u.completion_tokens as u64,
                cache_read_tokens: u.cache_read_input_tokens.map(|v| v as u64),
                cache_write_tokens: u.cache_creation_input_tokens.map(|v| v as u64),
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
            if let Err(e) = recorder
                .lock()
                .await
                .record_llm_response(&llm_response.content, &model, usage, tool_calls)
                .await
            {
                tracing::warn!(error = %e, "Failed to record LLM response (non-fatal)");
            }
        }

        // Convert messages to JSON for recording
        let messages_json: Vec<serde_json::Value> = working_messages
            .iter()
            .map(|m| serde_json::to_value(m).unwrap_or_default())
            .collect();

        // Add input messages and LLM response to step
        step = step
            .with_llm_messages(messages_json)
            .with_llm_response(llm_response.clone());

        // Process response - use working_messages which may have been auto-compacted
        let mut new_messages = working_messages;

        // Display assistant response with proper formatting
        if !llm_response.content.is_empty() {
            println!();
            println!(
                "  {} {}",
                "󰚩".bright_cyan(),
                "AI Response".bright_white().bold()
            );
            println!();
            // Print markdown content with 2-space indent
            for line in DisplayManager::render_markdown_lines(&llm_response.content) {
                println!("  {}", line);
            }
        }

        // Add assistant message with tool_calls if present
        // CRITICAL: The assistant message MUST include tool_calls for the subsequent
        // tool messages to reference via tool_call_id. OpenRouter/Anthropic API requires
        // each tool_result to have a corresponding tool_use in the previous message.
        if !llm_response.tool_calls.is_empty() || !llm_response.content.is_empty() {
            let mut assistant_msg = LlmMessage::assistant(&llm_response.content);
            if !llm_response.tool_calls.is_empty() {
                assistant_msg.tool_calls = Some(llm_response.tool_calls.clone());
            }
            new_messages.push(assistant_msg);
        }

        // Handle tool calls
        if !llm_response.tool_calls.is_empty() {
            self.handle_tool_calls(
                &mut step,
                &mut new_messages,
                &llm_response.tool_calls,
                task_scope,
            )
            .await?;
        }

        // Check for completion indicator in response
        // Support multiple LLM providers with different finish_reason values:
        // - Anthropic: "end_turn"
        // - OpenAI/GLM/others: "stop"
        // - Google: "STOP"
        let is_natural_end = match llm_response.finish_reason.as_deref() {
            Some("end_turn") | Some("stop") | Some("STOP") => true,
            _ => false,
        };

        if is_natural_end && llm_response.tool_calls.is_empty() {
            tracing::info!(
                finish_reason = ?llm_response.finish_reason,
                "step indicates task completion (natural end)"
            );
            step.state = AgentState::Completed;
        }

        Ok((step, new_messages))
    }

    /// Handle tool call execution using three-phase model
    async fn handle_tool_calls(
        &mut self,
        step: &mut AgentStep,
        new_messages: &mut Vec<LlmMessage>,
        tool_calls: &[ToolCall],
        task_scope: &crate::interrupt::TaskScope,
    ) -> SageResult<()> {
        tracing::info!(tool_count = tool_calls.len(), "executing tools");

        // Display tool execution header
        println!();
        println!(
            "  {} {} ({})",
            "".bright_magenta(),
            "Executing tools".bright_white().bold(),
            tool_calls.len().to_string().dimmed()
        );

        // Build execution context for tool orchestrator
        let context = self.build_execution_context();

        for tool_call in tool_calls {
            // Check for interrupt before each tool
            if task_scope.is_cancelled() {
                self.event_manager.stop_animation().await;
                return Err(SageError::agent("Task interrupted during tool execution"));
            }

            let tool_result = self
                .execute_single_tool(tool_call, step.step_number, &context)
                .await?;

            step.tool_results.push(tool_result.clone());

            // Add tool result to messages
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
        step_number: u32,
        context: &ToolExecutionContext,
    ) -> SageResult<crate::tools::types::ToolResult> {
        // Display and animation setup
        self.display_tool_start(tool_call, step_number).await;

        // Track files for undo capability
        self.track_file_for_undo(tool_call).await;

        // Record tool call
        self.record_tool_call(tool_call).await;

        let tool_start_time = std::time::Instant::now();

        // === Phase 1: Pre-execution (hooks) ===
        let pre_result = self
            .tool_orchestrator
            .pre_execution_phase(tool_call, context)
            .await?;

        let tool_result = if let Some(reason) = pre_result.block_reason() {
            // Tool was blocked by hook
            crate::tools::types::ToolResult::error(
                &tool_call.id,
                &tool_call.name,
                format!("Tool execution blocked by hook: {}", reason),
            )
        } else {
            // === Phase 2: Execution ===
            self.execute_tool_phase(tool_call).await?
        };

        // === Phase 3: Post-execution (hooks) ===
        self.tool_orchestrator
            .post_execution_phase(tool_call, &tool_result, context)
            .await?;

        // Record and display result
        self.record_tool_result(tool_call, &tool_result, tool_start_time)
            .await;
        self.display_tool_result(&tool_result, tool_start_time).await;

        Ok(tool_result)
    }

    /// Execute the tool (phase 2), handling user interaction and permission checks
    async fn execute_tool_phase(
        &mut self,
        tool_call: &ToolCall,
    ) -> SageResult<crate::tools::types::ToolResult> {
        let requires_interaction = self
            .tool_orchestrator
            .requires_user_interaction(&tool_call.name);

        if requires_interaction && tool_call.name == "ask_user_question" {
            self.handle_ask_user_question(tool_call).await
        } else if requires_interaction {
            Ok(self.tool_orchestrator.execution_phase(tool_call).await)
        } else {
            Ok(self.execute_tool_with_permission_check(tool_call).await)
        }
    }

    /// Display tool start information and animation
    async fn display_tool_start(&mut self, tool_call: &ToolCall, _step_number: u32) {
        let tool_icon = tool_display::get_tool_icon(&tool_call.name);
        let params_preview = tool_display::format_tool_params(&tool_call.arguments);

        println!();
        println!(
            "  {} {} {}",
            tool_icon.bright_magenta(),
            tool_call.name.bright_magenta().bold(),
            params_preview.dimmed()
        );

        // Emit tool execution started event
        self.event_manager
            .emit(ExecutionEvent::ToolExecutionStarted {
                tool_name: tool_call.name.clone(),
                tool_id: tool_call.id.clone(),
            })
            .await;
    }

    /// Track file for undo capability before modification
    async fn track_file_for_undo(&mut self, tool_call: &ToolCall) {
        if crate::tools::names::is_file_modifying_tool(&tool_call.name) {
            if let Some(file_path) = tool_call
                .arguments
                .get("file_path")
                .or_else(|| tool_call.arguments.get("path"))
                .and_then(|v| v.as_str())
            {
                if let Err(e) = self.session_manager.track_file(file_path).await {
                    tracing::warn!(
                        error = %e,
                        file_path = %file_path,
                        "Failed to track file for undo (non-fatal)"
                    );
                }
            }
        }
    }

    /// Record tool call before execution
    async fn record_tool_call(&self, tool_call: &ToolCall) {
        if let Some(recorder) = self.session_manager.session_recorder() {
            let tool_input = serde_json::to_value(&tool_call.arguments).unwrap_or_default();
            if let Err(e) = recorder
                .lock()
                .await
                .record_tool_call(&tool_call.name, tool_input)
                .await
            {
                tracing::warn!(
                    error = %e,
                    tool_name = %tool_call.name,
                    "Failed to record tool call (non-fatal)"
                );
            }
        }
    }

    /// Record tool result after execution
    async fn record_tool_result(
        &self,
        tool_call: &ToolCall,
        tool_result: &crate::tools::types::ToolResult,
        start_time: std::time::Instant,
    ) {
        if let Some(recorder) = self.session_manager.session_recorder() {
            let execution_time_ms = start_time.elapsed().as_millis() as u64;
            if let Err(e) = recorder
                .lock()
                .await
                .record_tool_result(
                    &tool_call.name,
                    tool_result.success,
                    tool_result.output.clone(),
                    tool_result.error.clone(),
                    execution_time_ms,
                )
                .await
            {
                tracing::warn!(
                    error = %e,
                    tool_name = %tool_call.name,
                    "Failed to record tool result (non-fatal)"
                );
            }
        }
    }

    /// Display tool result
    async fn display_tool_result(
        &mut self,
        tool_result: &crate::tools::types::ToolResult,
        start_time: std::time::Instant,
    ) {
        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Emit tool execution completed event
        self.event_manager
            .emit(ExecutionEvent::ToolExecutionCompleted {
                tool_name: tool_result.tool_name.clone(),
                tool_id: tool_result.call_id.clone(),
                success: tool_result.success,
                duration_ms,
            })
            .await;

        let status_icon = if tool_result.success {
            "✓".green()
        } else {
            "✗".red()
        };

        print!("    {} ", status_icon);
        if tool_result.success {
            println!("{} ({}ms)", "done".green(), duration_ms);
        } else {
            println!("{} ({}ms)", "failed".red(), duration_ms);
            if let Some(ref err) = tool_result.error {
                let first_line = err.lines().next().unwrap_or(err);
                let truncated = crate::utils::truncate_with_ellipsis(first_line, 60);
                println!("      {}", truncated.dimmed());
            }
        }
    }

    /// Execute a tool with permission check for dangerous operations
    ///
    /// If the tool returns ConfirmationRequired error, this will:
    /// 1. Stop the animation
    /// 2. Show a permission dialog to the user
    /// 3. If user confirms, re-execute with user_confirmed=true
    /// 4. If user denies, return a rejection message
    async fn execute_tool_with_permission_check(
        &mut self,
        tool_call: &ToolCall,
    ) -> crate::tools::types::ToolResult {
        // First attempt - may fail with ConfirmationRequired
        let result = self.tool_orchestrator.execution_phase(tool_call).await;

        // Check if the result indicates confirmation is required
        if !result.success {
            if let Some(ref error_msg) = result.error {
                if error_msg.contains("DESTRUCTIVE COMMAND BLOCKED")
                    || error_msg.contains("Confirmation required")
                {
                    return self.handle_permission_dialog(tool_call).await;
                }
            }
        }

        result
    }

    /// Handle permission dialog for destructive operations
    async fn handle_permission_dialog(
        &mut self,
        tool_call: &ToolCall,
    ) -> crate::tools::types::ToolResult {
        // Stop animation to show dialog
        self.event_manager.stop_animation().await;

        let command = tool_call
            .arguments
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown command");

        let config = PermissionDialogConfig::new(
            &tool_call.name,
            command,
            "This is a destructive operation that may delete files or make irreversible changes.",
        );

        let choice = show_permission_dialog(&config);

        // Restart animation
        self.event_manager
            .start_animation(AnimationState::ExecutingTools, "Executing tools", "green")
            .await;

        match choice {
            PermissionChoice::YesOnce | PermissionChoice::YesAlways => {
                let mut confirmed_call = tool_call.clone();
                confirmed_call
                    .arguments
                    .insert("user_confirmed".to_string(), serde_json::Value::Bool(true));

                tracing::info!(
                    tool = %tool_call.name,
                    command = %command,
                    "user confirmed destructive operation"
                );

                self.tool_orchestrator.execution_phase(&confirmed_call).await
            }
            PermissionChoice::NoOnce | PermissionChoice::NoAlways => {
                tracing::info!(
                    tool = %tool_call.name,
                    command = %command,
                    "user rejected destructive operation"
                );

                crate::tools::types::ToolResult::error(
                    &tool_call.id,
                    &tool_call.name,
                    format!(
                        "Operation cancelled by user. The user rejected the command: {}",
                        command
                    ),
                )
            }
            PermissionChoice::Cancelled => {
                tracing::info!(
                    tool = %tool_call.name,
                    "user cancelled permission dialog"
                );

                crate::tools::types::ToolResult::error(
                    &tool_call.id,
                    &tool_call.name,
                    "Operation cancelled by user (Ctrl+C or empty input).",
                )
            }
        }
    }
}
