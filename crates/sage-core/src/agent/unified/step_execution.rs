//! Single step execution logic

use crate::agent::{AgentState, AgentStep};
use crate::error::{SageError, SageResult};
use crate::hooks::{HookEvent, HookInput};
use crate::interrupt::global_interrupt_manager;
use crate::llm::messages::LlmMessage;
use crate::llm::streaming::{StreamingLlmClient, stream_utils};
use crate::tools::types::{ToolCall, ToolSchema};
use crate::trajectory::TokenUsage;
use crate::ui::DisplayManager;
use crate::ui::animation::{AnimationContext, AnimationState};
use crate::ui::prompt::{PermissionChoice, PermissionDialogConfig, show_permission_dialog};
use colored::Colorize;
use tokio::select;
use tokio_util::sync::CancellationToken;
use tracing::instrument;

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

        // Update animation manager with current step info
        self.animation_manager.set_step(step_number);

        // Start thinking animation with context
        let context = AnimationContext::new().with_step(step_number);
        self.animation_manager
            .start_with_context(AnimationState::Thinking, "Thinking", "blue", context)
            .await;

        // Record LLM request before sending
        if let Some(recorder) = &self.session_recorder {
            let input_messages: Vec<serde_json::Value> = working_messages
                .iter()
                .map(|msg| serde_json::to_value(msg).unwrap_or_default())
                .collect();
            let tools_available: Vec<String> =
                tool_schemas.iter().map(|t| t.name.clone()).collect();
            let _ = recorder
                .lock()
                .await
                .record_llm_request(input_messages, Some(tools_available))
                .await;
        }

        // Get cancellation token for interrupt handling
        let cancellation_token = global_interrupt_manager().lock().cancellation_token();

        // Execute LLM call with interrupt support using streaming API
        // Streaming keeps the connection alive, avoiding timeout issues with slow models
        // Uses collect_stream_with_cancel for faster cancellation response between chunks
        let llm_response = select! {
            response = async {
                // Use streaming API and collect into complete response with cancel support
                let stream = self.llm_client.chat_stream(&working_messages, Some(tool_schemas)).await?;
                stream_utils::collect_stream_with_cancel(stream, &cancellation_token).await
            } => {
                response?
            }
            _ = cancellation_token.cancelled() => {
                self.animation_manager.stop_animation().await;
                return Err(SageError::agent("Task interrupted during LLM call"));
            }
        };

        // Stop animation
        self.animation_manager.stop_animation().await;

        // Record LLM response
        if let Some(recorder) = &self.session_recorder {
            let model = self
                .config
                .default_model_parameters()
                .map(|p| p.model.clone())
                .unwrap_or_default();
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
            let _ = recorder
                .lock()
                .await
                .record_llm_response(&llm_response.content, &model, usage, tool_calls)
                .await;
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

    /// Handle tool call execution
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

        for tool_call in tool_calls {
            // Build activity description for animation detail
            let activity_desc =
                Self::build_activity_description(&tool_call.name, &tool_call.arguments);

            // Display tool call info with parameters
            let tool_icon = Self::get_tool_icon(&tool_call.name);
            let params_preview = Self::format_tool_params(&tool_call.arguments);
            println!();
            println!(
                "  {} {} {}",
                tool_icon.bright_magenta(),
                tool_call.name.bright_magenta().bold(),
                params_preview.dimmed()
            );

            // Start animation for this specific tool with detail context
            let context = AnimationContext::new()
                .with_step(step.step_number)
                .with_detail(&activity_desc);
            self.animation_manager
                .start_with_context(
                    AnimationState::ExecutingTools,
                    &format!("Running {}", tool_call.name),
                    "green",
                    context,
                )
                .await;
            // Check for interrupt before each tool
            if task_scope.is_cancelled() {
                self.animation_manager.stop_animation().await;
                return Err(SageError::agent("Task interrupted during tool execution"));
            }

            // Track files before file-modifying tools execute (for undo capability)
            if crate::tools::names::is_file_modifying_tool(&tool_call.name) {
                if let Some(file_path) = tool_call
                    .arguments
                    .get("file_path")
                    .or_else(|| tool_call.arguments.get("path"))
                    .and_then(|v| v.as_str())
                {
                    let _ = self.file_tracker.track_file(file_path).await;
                }
            }

            // Record tool call before execution
            if let Some(recorder) = &self.session_recorder {
                let tool_input = serde_json::to_value(&tool_call.arguments).unwrap_or_default();
                let _ = recorder
                    .lock()
                    .await
                    .record_tool_call(&tool_call.name, tool_input)
                    .await;
            }

            // === PreToolUse Hook ===
            // Execute PreToolUse hooks before tool execution
            let session_id = self
                .current_session_id
                .clone()
                .unwrap_or_else(|| self.id.to_string());
            let working_dir = self
                .options
                .working_directory
                .clone()
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

            let pre_hook_input = HookInput::new(HookEvent::PreToolUse, &session_id)
                .with_cwd(working_dir.clone())
                .with_tool_name(&tool_call.name)
                .with_tool_input(serde_json::to_value(&tool_call.arguments).unwrap_or_default());

            let cancel_token = CancellationToken::new();
            let pre_hook_results = self
                .hook_executor
                .execute(
                    HookEvent::PreToolUse,
                    &tool_call.name,
                    pre_hook_input,
                    cancel_token.clone(),
                )
                .await
                .unwrap_or_default();

            // Check if any hook blocked the tool execution
            let mut hook_blocked = false;
            let mut block_reason = String::new();
            for result in &pre_hook_results {
                if !result.should_continue() {
                    hook_blocked = true;
                    block_reason = result.message().unwrap_or("Blocked by hook").to_string();
                    tracing::warn!(
                        tool = %tool_call.name,
                        reason = %block_reason,
                        "PreToolUse hook blocked tool execution"
                    );
                    break;
                }
            }

            let tool_start_time = std::time::Instant::now();

            // Execute tool (or skip if blocked by hook)
            let tool_result = if hook_blocked {
                // Tool was blocked by PreToolUse hook
                crate::tools::types::ToolResult::error(
                    &tool_call.id,
                    &tool_call.name,
                    format!("Tool execution blocked by hook: {}", block_reason),
                )
            } else {
                // Check if this tool requires user interaction (blocking input)
                let requires_interaction = self
                    .tool_executor
                    .get_tool(&tool_call.name)
                    .map(|t| t.requires_user_interaction())
                    .unwrap_or(false);

                // Handle tools that require user interaction with blocking input
                if requires_interaction && tool_call.name == "ask_user_question" {
                    // Use specialized handler for ask_user_question
                    self.handle_ask_user_question(tool_call).await?
                } else if requires_interaction {
                    // Generic handling for other interactive tools
                    // For now, just execute normally - can be extended later
                    self.tool_executor.execute_tool(tool_call).await
                } else {
                    // Normal tool execution - may require permission confirmation
                    self.execute_tool_with_permission_check(tool_call).await
                }
            };

            // === PostToolUse Hook ===
            // Execute PostToolUse hooks after tool execution
            let post_event = if tool_result.success {
                HookEvent::PostToolUse
            } else {
                HookEvent::PostToolUseFailure
            };

            let post_hook_input = HookInput::new(post_event, &session_id)
                .with_cwd(working_dir)
                .with_tool_name(&tool_call.name)
                .with_tool_input(serde_json::to_value(&tool_call.arguments).unwrap_or_default())
                .with_tool_result(serde_json::to_value(&tool_result).unwrap_or_default());

            let _ = self
                .hook_executor
                .execute(post_event, &tool_call.name, post_hook_input, cancel_token)
                .await;

            // Record tool result after execution
            if let Some(recorder) = &self.session_recorder {
                let execution_time_ms = tool_start_time.elapsed().as_millis() as u64;
                let _ = recorder
                    .lock()
                    .await
                    .record_tool_result(
                        &tool_call.name,
                        tool_result.success,
                        tool_result.output.clone(),
                        tool_result.error.clone(),
                        execution_time_ms,
                    )
                    .await;
            }

            // Stop animation and display result
            self.animation_manager.stop_animation().await;

            // Display tool result
            let status_icon = if tool_result.success {
                "✓".green()
            } else {
                "✗".red()
            };
            let duration_ms = tool_start_time.elapsed().as_millis();
            print!("    {} ", status_icon);
            if tool_result.success {
                println!("{} ({}ms)", "done".green(), duration_ms);
            } else {
                println!("{} ({}ms)", "failed".red(), duration_ms);
                if let Some(ref err) = tool_result.error {
                    // Show first line of error (UTF-8 safe truncation)
                    let first_line = err.lines().next().unwrap_or(err);
                    let truncated = crate::utils::truncate_with_ellipsis(first_line, 60);
                    println!("      {}", truncated.dimmed());
                }
            }

            step.tool_results.push(tool_result.clone());

            // Add tool result to messages using LlmMessage::tool
            let tool_name = Some(tool_call.name.clone());
            new_messages.push(LlmMessage::tool(
                tool_result.output.clone().unwrap_or_default(),
                tool_call.id.clone(),
                tool_name,
            ));
        }

        step.state = AgentState::ToolExecution;

        Ok(())
    }

    /// Get icon for specific tool type
    fn get_tool_icon(tool_name: &str) -> &'static str {
        match tool_name.to_lowercase().as_str() {
            "bash" | "shell" | "execute" => "",
            "read" | "cat" => "",
            "write" | "edit" => "",
            "grep" | "search" => "",
            "glob" | "find" => "",
            "lsp" | "code" => "",
            "web_fetch" | "web_search" => "󰖟",
            "task" | "todo_write" => "",
            _ => "",
        }
    }

    /// Format tool parameters for display
    fn format_tool_params(
        arguments: &std::collections::HashMap<String, serde_json::Value>,
    ) -> String {
        // Extract key parameters to show
        let mut parts = Vec::new();

        // Show file_path or path if present
        if let Some(path) = arguments.get("file_path").or(arguments.get("path")) {
            if let Some(s) = path.as_str() {
                let display = if s.len() > 40 {
                    format!("...{}", &s[s.len().saturating_sub(37)..])
                } else {
                    s.to_string()
                };
                parts.push(display);
            }
        }

        // Show command if present (for bash) - UTF-8 safe
        if let Some(cmd) = arguments.get("command") {
            if let Some(s) = cmd.as_str() {
                let display = crate::utils::truncate_with_ellipsis(s, 50);
                parts.push(display);
            }
        }

        // Show pattern if present (for grep/glob)
        if let Some(pattern) = arguments.get("pattern") {
            if let Some(s) = pattern.as_str() {
                parts.push(format!("pattern={}", s));
            }
        }

        // Show query if present (for search) - UTF-8 safe
        if let Some(query) = arguments.get("query") {
            if let Some(s) = query.as_str() {
                let display = crate::utils::truncate_with_ellipsis(s, 30);
                parts.push(format!("query=\"{}\"", display));
            }
        }

        if parts.is_empty() {
            String::new()
        } else {
            parts.join(" ")
        }
    }

    /// Build activity description for progress tracking
    fn build_activity_description(
        tool_name: &str,
        arguments: &std::collections::HashMap<String, serde_json::Value>,
    ) -> String {
        let verb = match tool_name.to_lowercase().as_str() {
            "read" => "reading",
            "write" => "writing",
            "edit" => "editing",
            "bash" => "running",
            "glob" => "searching",
            "grep" => "searching",
            "web_fetch" => "fetching",
            "web_search" => "searching web",
            "task" => "running subagent",
            "lsp" => "analyzing",
            _ => "executing",
        };

        // Extract key info
        if let Some(path) = arguments.get("file_path").or(arguments.get("path")) {
            if let Some(s) = path.as_str() {
                // Get just the filename
                let filename = std::path::Path::new(s)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(s);
                return format!("{} {}", verb, filename);
            }
        }

        if let Some(cmd) = arguments.get("command") {
            if let Some(s) = cmd.as_str() {
                let short = crate::utils::truncate_str(s, 30);
                return format!("{} '{}'", verb, short);
            }
        }

        if let Some(pattern) = arguments.get("pattern") {
            if let Some(s) = pattern.as_str() {
                return format!("{} for '{}'", verb, s);
            }
        }

        // Task tool: show description or prompt preview
        if tool_name.to_lowercase() == "task" {
            if let Some(desc) = arguments.get("description") {
                if let Some(s) = desc.as_str() {
                    return format!("{}: {}", verb, crate::utils::truncate_str(s, 40));
                }
            }
            if let Some(prompt) = arguments.get("prompt") {
                if let Some(s) = prompt.as_str() {
                    let preview = crate::utils::truncate_str(s, 40);
                    return format!("{}: {}", verb, preview);
                }
            }
        }

        format!("{} {}", verb, tool_name)
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
        let result = self.tool_executor.execute_tool(tool_call).await;

        // Check if the result indicates confirmation is required
        // The error message will be in the output field for failed results
        if !result.success {
            if let Some(ref error_msg) = result.error {
                if error_msg.contains("DESTRUCTIVE COMMAND BLOCKED")
                    || error_msg.contains("Confirmation required")
                {
                    // Stop animation to show dialog
                    self.animation_manager.stop_animation().await;

                    // Extract command from tool call
                    let command = tool_call
                        .arguments
                        .get("command")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown command");

                    // Show permission dialog
                    let config = PermissionDialogConfig::new(
                        &tool_call.name,
                        command,
                        "This is a destructive operation that may delete files or make irreversible changes.",
                    );

                    let choice = show_permission_dialog(&config);

                    // Restart animation
                    self.animation_manager
                        .start_animation(AnimationState::ExecutingTools, "Executing tools", "green")
                        .await;

                    match choice {
                        PermissionChoice::YesOnce | PermissionChoice::YesAlways => {
                            // User confirmed - re-execute with user_confirmed=true
                            let mut confirmed_call = tool_call.clone();
                            confirmed_call.arguments.insert(
                                "user_confirmed".to_string(),
                                serde_json::Value::Bool(true),
                            );

                            tracing::info!(
                                tool = %tool_call.name,
                                command = %command,
                                "user confirmed destructive operation"
                            );

                            return self.tool_executor.execute_tool(&confirmed_call).await;
                        }
                        PermissionChoice::NoOnce | PermissionChoice::NoAlways => {
                            tracing::info!(
                                tool = %tool_call.name,
                                command = %command,
                                "user rejected destructive operation"
                            );

                            return crate::tools::types::ToolResult::error(
                                &tool_call.id,
                                &tool_call.name,
                                format!(
                                    "Operation cancelled by user. The user rejected the command: {}",
                                    command
                                ),
                            );
                        }
                        PermissionChoice::Cancelled => {
                            tracing::info!(
                                tool = %tool_call.name,
                                "user cancelled permission dialog"
                            );

                            return crate::tools::types::ToolResult::error(
                                &tool_call.id,
                                &tool_call.name,
                                "Operation cancelled by user (Ctrl+C or empty input).",
                            );
                        }
                    }
                }
            }
        }

        result
    }
}
