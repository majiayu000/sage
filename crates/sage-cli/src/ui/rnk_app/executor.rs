//! Executor logic for rnk app

use super::state::{SharedState, UiCommand};
use super::theme::current_theme;
use crate::commands::unified::slash_commands::{SlashCommandAction, process_slash_command};
use crate::console::CliConsole;
use rnk::prelude::*;
use sage_core::agent::UnifiedExecutor;
use sage_core::agent::{ExecutionMode, ExecutionOptions};
use sage_core::error::SageResult;
use sage_core::input::InputChannel;
use sage_core::interrupt::{
    InterruptReason, interrupt_current_task, reset_global_interrupt_manager,
};
use sage_core::output::OutputMode;
use sage_core::types::TaskMetadata;
use sage_core::ui::bridge::AgentEvent;
use sage_core::ui::bridge::state::ExecutionPhase;
use sage_core::ui::traits::UiContext;
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};
use unicode_width::UnicodeWidthStr;

/// Create executor with unified configuration path
pub async fn create_executor(
    ui_context: Option<UiContext>,
    config_file: &str,
    working_dir: Option<std::path::PathBuf>,
    max_steps: Option<u32>,
) -> SageResult<UnifiedExecutor> {
    let mut config = if std::path::Path::new(config_file).exists() {
        sage_core::config::load_config_from_file(config_file)?
    } else {
        sage_core::config::load_config()?
    };

    // If the default provider has no key, pick the first provider that does.
    if let Some(params) = config.model_providers.get(&config.default_provider) {
        if params
            .get_api_key_info_for_provider(&config.default_provider)
            .key
            .is_none()
        {
            if let Some((provider, _)) = config.model_providers.iter().find(|(provider, params)| {
                params.get_api_key_info_for_provider(provider).key.is_some()
                    || provider.as_str() == "ollama"
            }) {
                config.default_provider = provider.clone();
            }
        }
    }

    let resolved_working_dir = working_dir
        .or_else(|| config.working_directory.clone())
        .unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
        });

    let mut options = ExecutionOptions::default()
        .with_mode(ExecutionMode::interactive())
        .with_working_directory(&resolved_working_dir);

    if let Some(steps) = max_steps {
        options = options.with_step_limit(steps);
    }

    let mut executor = UnifiedExecutor::with_options(config.clone(), options)?;

    if let Some(ctx) = ui_context {
        executor.set_ui_context(ctx);
    }

    executor.set_output_mode(OutputMode::Rnk);

    // Register default tools
    let mut all_tools = sage_tools::get_default_tools();

    // Load MCP tools if MCP is enabled
    if config.mcp.enabled {
        match crate::commands::unified::build_mcp_registry_from_config(&config).await {
            Ok(mcp_registry) => {
                let mcp_tools = mcp_registry.as_tools().await;
                if !mcp_tools.is_empty() {
                    all_tools.extend(mcp_tools);
                }
            }
            Err(e) => {
                tracing::error!("Failed to build MCP registry: {}", e);
            }
        }
    }

    executor.register_tools(all_tools);
    let _ = executor.init_subagent_support();

    // Set up JSONL storage for session management
    let jsonl_storage = sage_core::session::JsonlSessionStorage::default_path()?;
    executor.set_jsonl_storage(std::sync::Arc::new(jsonl_storage));

    // Enable JSONL session recording
    let _ = executor.enable_session_recording().await;

    Ok(executor)
}

/// Executor loop in background - processes commands and runs tasks
pub async fn executor_loop(
    state: SharedState,
    mut rx: mpsc::Receiver<UiCommand>,
    input_channel: InputChannel,
    ui_context: UiContext,
    config_file: String,
    working_dir: Option<std::path::PathBuf>,
    max_steps: Option<u32>,
) {
    // Clone ui_context for event emission, pass original to executor
    let event_ctx = ui_context.clone();

    // Create executor with UI context
    let mut executor = match create_executor(
        Some(ui_context),
        &config_file,
        working_dir.clone(),
        max_steps,
    )
    .await
    {
        Ok(e) => e,
        Err(e) => {
            rnk::println(
                Text::new(format!("Failed to create executor: {}", e))
                    .color(Color::Red)
                    .into_element(),
            );
            state.write().should_quit = true;
            rnk::request_render();
            return;
        }
    };
    executor.set_input_channel(input_channel);

    // Process commands
    while let Some(cmd) = rx.recv().await {
        match cmd {
            UiCommand::Submit(task) => {
                let working_dir = executor
                    .options()
                    .working_directory
                    .clone()
                    .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
                let console = CliConsole::new(false);

                // Process slash commands first
                let prompt = match process_slash_command(&task, &console, &working_dir).await {
                    Ok(SlashCommandAction::Prompt(p)) => p,
                    Ok(SlashCommandAction::Handled) => {
                        // Command was handled locally, no LLM needed
                        rnk::request_render();
                        continue;
                    }
                    Ok(SlashCommandAction::HandledWithOutput(output)) => {
                        // Command was handled locally with output to display
                        // Print each line separately to avoid rnk layout issues
                        for line in output.lines() {
                            rnk::println(Text::new(line).color(Color::White).into_element());
                        }
                        rnk::request_render();
                        continue;
                    }
                    Ok(SlashCommandAction::SetOutputMode(mode)) => {
                        executor.set_output_mode(mode);
                        rnk::println(
                            Text::new(format!("Output mode set to {:?}", mode))
                                .color(Color::Cyan)
                                .dim()
                                .into_element(),
                        );
                        rnk::request_render();
                        continue;
                    }
                    Ok(SlashCommandAction::Resume { session_id }) => {
                        // Handle resume command
                        {
                            let mut s = state.write();
                            s.is_busy = true;
                            s.status_text = "Resuming session...".to_string();
                        }
                        rnk::request_render();

                        let resume_result = if let Some(id) = session_id {
                            executor.restore_session(&id).await.map(|msgs| {
                                format!("Session {} restored ({} messages)", id, msgs.len())
                            })
                        } else {
                            match executor.get_most_recent_session().await {
                                Ok(Some(metadata)) => {
                                    let id = metadata.id;
                                    match executor.restore_session(&id).await {
                                        Ok(msgs) => Ok(format!(
                                            "Session {} restored ({} messages)",
                                            id,
                                            msgs.len()
                                        )),
                                        Err(e) => Err(e),
                                    }
                                }
                                Ok(None) => Err(sage_core::error::SageError::config(
                                    "No previous sessions found. Start a new session first.",
                                )),
                                Err(e) => Err(e),
                            }
                        };

                        {
                            let mut s = state.write();
                            s.is_busy = false;
                            s.status_text.clear();
                        }

                        match resume_result {
                            Ok(msg) => {
                                rnk::println(
                                    Text::new(format!("✓ {}", msg))
                                        .color(Color::Green)
                                        .into_element(),
                                );
                            }
                            Err(e) => {
                                rnk::println(
                                    Text::new(format!("✗ Resume failed: {}", e))
                                        .color(Color::Red)
                                        .into_element(),
                                );
                            }
                        }
                        rnk::request_render();
                        continue;
                    }
                    Ok(SlashCommandAction::SwitchModel { model }) => {
                        // Try to switch model dynamically
                        match executor.switch_model(&model) {
                            Ok(_) => {
                                // Update UI state with new model
                                {
                                    let mut s = state.write();
                                    s.session.model = model.clone();
                                }
                                rnk::println(
                                    Text::new(format!("✓ Switched to model: {}", model))
                                        .color(Color::Green)
                                        .into_element(),
                                );
                            }
                            Err(e) => {
                                rnk::println(
                                    Text::new(format!("✗ Failed to switch model: {}", e))
                                        .color(Color::Red)
                                        .into_element(),
                                );
                            }
                        }
                        rnk::request_render();
                        continue;
                    }
                    Ok(SlashCommandAction::ModelSelect { models }) => {
                        // Enter model selection mode
                        {
                            let mut s = state.write();
                            s.model_select_mode = true;
                            s.available_models = models;
                            s.model_select_index = 0;
                        }
                        rnk::request_render();
                        continue;
                    }
                    Ok(SlashCommandAction::Doctor) => {
                        // Run diagnostics
                        {
                            let mut s = state.write();
                            s.is_busy = true;
                            s.status_text = "Running diagnostics...".to_string();
                        }
                        rnk::request_render();

                        // Run doctor command
                        let result = crate::commands::diagnostics::doctor(&config_file).await;

                        {
                            let mut s = state.write();
                            s.is_busy = false;
                            s.status_text.clear();
                        }

                        if let Err(e) = result {
                            rnk::println(
                                Text::new(format!("Diagnostics failed: {}", e))
                                    .color(Color::Red)
                                    .into_element(),
                            );
                        }
                        rnk::request_render();
                        continue;
                    }
                    Ok(SlashCommandAction::Exit) => {
                        state.write().should_quit = true;
                        rnk::request_render();
                        break;
                    }
                    Err(e) => {
                        rnk::println(
                            Text::new(format!("Command error: {}", e))
                                .color(Color::Red)
                                .into_element(),
                        );
                        rnk::request_render();
                        continue;
                    }
                };

                {
                    let mut s = state.write();
                    s.is_busy = true;
                    s.status_text = "Thinking...".to_string();
                }
                rnk::request_render();

                // Reset interrupt manager for new task
                reset_global_interrupt_manager();

                event_ctx.emit(AgentEvent::UserInputReceived {
                    input: prompt.clone(),
                });
                event_ctx.emit(AgentEvent::ThinkingStarted);

                // Execute task
                let task_meta = TaskMetadata::new(&prompt, &working_dir.display().to_string());

                match executor.execute(task_meta).await {
                    Ok(_) => {}
                    Err(e) => {
                        event_ctx.emit(AgentEvent::error("execution", e.to_string()));
                    }
                }

                {
                    let mut s = state.write();
                    s.is_busy = false;
                    s.status_text.clear();
                }
                rnk::request_render();
            }
            UiCommand::Cancel => {
                // Actually cancel the running task through interrupt manager
                interrupt_current_task(InterruptReason::UserInterrupt);

                event_ctx.emit(AgentEvent::ThinkingStopped);
                rnk::println(
                    Text::new("⦻ Cancelled")
                        .color(Color::Yellow)
                        .dim()
                        .into_element(),
                );
                {
                    let mut s = state.write();
                    s.is_busy = false;
                    s.status_text.clear();
                }
                rnk::request_render();
            }
            UiCommand::Quit => {
                state.write().should_quit = true;
                rnk::request_render();
                break;
            }
        }
    }
}

/// Background thread logic for printing messages and updating UI
pub async fn background_loop(state: SharedState, adapter: sage_core::ui::bridge::EventAdapter) {
    use super::components::{format_message, format_tool_start, render_error};

    let theme = current_theme();

    // Print header banner with border (Claude Code style)
    let version = env!("CARGO_PKG_VERSION");
    let (model, provider, working_dir) = {
        let ui_state = state.read();
        (
            ui_state.session.model.clone(),
            ui_state.session.provider.clone(),
            ui_state.session.working_dir.clone(),
        )
    };

    // Calculate box width based on content
    let title_line = format!("  ◆ Sage v{}", version);
    let model_line = format!("    {} · {}", model, provider);
    let dir_line = format!("    {}", working_dir);
    let content_width = [&title_line, &model_line, &dir_line]
        .iter()
        .map(|s| s.width())
        .max()
        .unwrap_or(40);
    let box_width = content_width + 4; // padding

    let top_border = format!("╭{}╮", "─".repeat(box_width));
    let bottom_border = format!("╰{}╯", "─".repeat(box_width));

    // Helper to pad line to box width
    let pad_line = |s: &str| -> String {
        let w = s.width();
        let padding = box_width.saturating_sub(w);
        format!("│{}{}│", s, " ".repeat(padding))
    };

    rnk::println(Text::new("").into_element());
    rnk::println(
        Text::new(&top_border)
            .color(theme.border_subtle)
            .into_element(),
    );
    rnk::println(
        Text::new(pad_line(&title_line))
            .color(theme.border_subtle)
            .into_element(),
    );
    rnk::println(
        Text::new(pad_line(&model_line))
            .color(theme.border_subtle)
            .into_element(),
    );
    rnk::println(
        Text::new(pad_line(&dir_line))
            .color(theme.border_subtle)
            .into_element(),
    );
    rnk::println(
        Text::new(&bottom_border)
            .color(theme.border_subtle)
            .into_element(),
    );
    // Spacing before bottom UI
    rnk::println(Text::new("").into_element());
    rnk::println(Text::new("").into_element());

    loop {
        sleep(Duration::from_millis(80)).await;

        // Check if should quit
        if state.read().should_quit {
            break;
        }

        // Collect data under lock, then process I/O outside lock
        let pending_work = {
            let app_state = adapter.get_state();
            // Use completed messages only (not streaming/temporary messages)
            // This avoids truncation issues where partial messages get printed
            let messages = &app_state.messages;
            let new_count = messages.len();

            let mut ui_state = state.write();

            // Update session info from adapter if changed
            if app_state.session.model != "unknown" && ui_state.session.model == "unknown" {
                ui_state.session.model = app_state.session.model.clone();
                ui_state.session.provider = app_state.session.provider.clone();
                if let Some(ref sid) = app_state.session.session_id {
                    ui_state.session.session_id = Some(sid.clone());
                }
            }

            // Header printing removed - now done in run_rnk_app() before rnk starts

            // Update busy state from adapter - Error state is not busy
            ui_state.is_busy = !matches!(
                app_state.phase,
                ExecutionPhase::Idle | ExecutionPhase::Error { .. }
            );
            if ui_state.is_busy {
                ui_state.status_text = app_state.status_text();
                // Increment animation frame for spinner
                ui_state.animation_frame = ui_state.animation_frame.wrapping_add(1);
            } else {
                ui_state.status_text.clear();
            }

            // Check for tool execution start - cache tool info to print after messages
            if let Some(ref tool_exec) = app_state.tool_execution {
                let tool_key = format!("{}:{}", tool_exec.tool_name, tool_exec.description);
                if ui_state.current_tool_printed.as_ref() != Some(&tool_key) {
                    // New tool detected, cache it
                    ui_state.pending_tool =
                        Some((tool_exec.tool_name.clone(), tool_exec.description.clone()));
                    ui_state.current_tool_printed = Some(tool_key);
                }
            } else {
                // Tool finished, clear the tracking
                ui_state.current_tool_printed = None;
            }

            // Collect error work
            let error_work = if let ExecutionPhase::Error { ref message } = app_state.phase {
                if !ui_state.error_displayed {
                    ui_state.error_displayed = true;
                    Some(render_error(message, theme))
                } else {
                    None
                }
            } else {
                ui_state.error_displayed = false;
                None
            };

            // Collect new messages - format them while holding lock
            // Skip ToolCall messages - they are printed via pending_tool mechanism
            let (new_messages, pending_tool_to_print) = if new_count > ui_state.printed_count {
                let msgs: Vec<_> = messages
                    .iter()
                    .skip(ui_state.printed_count)
                    .filter(|msg| {
                        !matches!(
                            msg.content,
                            sage_core::ui::bridge::state::MessageContent::ToolCall { .. }
                        )
                    })
                    .map(|msg| format_message(msg, theme))
                    .collect();
                ui_state.printed_count = new_count;
                // Only take pending tool if there are new text messages
                // This ensures text messages are printed before tool calls
                let pending = if !msgs.is_empty() {
                    ui_state.pending_tool.take()
                } else {
                    None
                };
                (msgs, pending)
            } else {
                (Vec::new(), None)
            };

            (error_work, new_messages, pending_tool_to_print)
        }; // Lock released here

        // Process all I/O outside the lock
        let (error_work, new_messages, pending_tool_to_print) = pending_work;

        // Print new messages first (Assistant response comes before tool call)
        for msg_element in new_messages {
            rnk::println(msg_element);
            rnk::println(""); // Empty line
        }

        // Print pending tool after messages
        if let Some((tool_name, description)) = pending_tool_to_print {
            rnk::println(format_tool_start(&tool_name, &description, theme));
        }

        if let Some(error) = error_work {
            rnk::println(error);
            rnk::println(""); // Empty line
        }

        // Request render to update spinner animation
        rnk::request_render();
    }
}
