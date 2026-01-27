//! Executor logic for rnk app

use super::state::{SharedState, UiCommand};
use super::theme::current_theme;
use crate::commands::unified::slash_commands::{process_slash_command, SlashCommandAction};
use crate::console::CliConsole;
use rnk::prelude::*;
use sage_core::agent::{ExecutionMode, ExecutionOptions, UnifiedExecutor};
use sage_core::config::load_config;
use sage_core::error::SageResult;
use sage_core::input::InputChannel;
use sage_core::interrupt::{interrupt_current_task, reset_global_interrupt_manager, InterruptReason};
use sage_core::output::OutputMode;
use sage_core::types::TaskMetadata;
use sage_core::ui::bridge::state::ExecutionPhase;
use sage_core::ui::traits::UiContext;
#[allow(deprecated)]
use sage_core::ui::bridge::{emit_event, AgentEvent};
use sage_tools::get_default_tools;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

/// Handle resume command
async fn handle_resume(
    executor: &mut UnifiedExecutor,
    session_id: Option<&str>,
) -> SageResult<String> {
    let session_id = match session_id {
        Some(id) => id.to_string(),
        None => {
            // Get most recent session
            match executor.get_most_recent_session().await? {
                Some(metadata) => metadata.id,
                None => {
                    return Err(sage_core::error::SageError::config(
                        "No previous sessions found. Start a new session first.",
                    ));
                }
            }
        }
    };

    // Restore the session
    let restored_messages = executor.restore_session(&session_id).await?;
    Ok(format!(
        "Session {} restored ({} messages)",
        session_id, restored_messages.len()
    ))
}

/// Create executor with default configuration
pub async fn create_executor(ui_context: Option<UiContext>) -> SageResult<UnifiedExecutor> {
    let config = load_config()?;
    let working_dir = std::env::current_dir().unwrap_or_default();
    let mode = ExecutionMode::interactive();
    let options = ExecutionOptions::default()
        .with_mode(mode)
        .with_working_directory(&working_dir);

    let mut executor = UnifiedExecutor::with_options(config, options)?;

    // Set UI context for event handling
    if let Some(ctx) = ui_context {
        executor.set_ui_context(ctx);
    }

    executor.set_output_mode(OutputMode::Rnk);
    executor.register_tools(get_default_tools());
    let _ = executor.init_subagent_support();
    Ok(executor)
}

/// Executor loop in background - processes commands and runs tasks
pub async fn executor_loop(
    state: SharedState,
    mut rx: mpsc::Receiver<UiCommand>,
    input_channel: InputChannel,
    ui_context: UiContext,
) {
    // Create executor with UI context
    let mut executor = match create_executor(Some(ui_context)).await {
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
                let working_dir = std::env::current_dir().unwrap_or_default();
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
                            rnk::println(
                                Text::new(line).color(Color::White).into_element(),
                            );
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

                        let result = handle_resume(&mut executor, session_id.as_deref()).await;

                        {
                            let mut s = state.write();
                            s.is_busy = false;
                            s.status_text.clear();
                        }

                        match result {
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
                    Ok(SlashCommandAction::Doctor) => {
                        // Run diagnostics
                        {
                            let mut s = state.write();
                            s.is_busy = true;
                            s.status_text = "Running diagnostics...".to_string();
                        }
                        rnk::request_render();

                        // Run doctor command
                        let result = crate::commands::diagnostics::doctor("sage_config.json").await;

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

                emit_event(AgentEvent::UserInputReceived { input: prompt.clone() });
                emit_event(AgentEvent::ThinkingStarted);

                // Execute task
                let working_dir_str = working_dir.to_string_lossy().to_string();
                let task_meta = TaskMetadata::new(&prompt, &working_dir_str);

                match executor.execute(task_meta).await {
                    Ok(_) => {}
                    Err(e) => {
                        emit_event(AgentEvent::error("execution", e.to_string()));
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

                emit_event(AgentEvent::ThinkingStopped);
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
pub async fn background_loop(
    state: SharedState,
    adapter: sage_core::ui::bridge::EventAdapter,
) {
    use super::components::{format_message, format_tool_start, render_error};
    use rnk::prelude::*;

    let theme = current_theme();

    // Print compact header using rnk::println
    let version = env!("CARGO_PKG_VERSION");
    let (model, provider, working_dir) = {
        let ui_state = state.read();
        (
            ui_state.session.model.clone(),
            ui_state.session.provider.clone(),
            ui_state.session.working_dir.clone(),
        )
    };

    // Get terminal width for full-width separator
    let term_width = crossterm::terminal::size().map(|(w, _)| w as usize).unwrap_or(80);

    // Compact header - dark colors for light background
    rnk::println(Text::new("").into_element());
    rnk::println(
        Text::new(format!("sage v{} · {} · {}", version, model, provider))
            .color(theme.accent_assistant)
            .bold()
            .into_element(),
    );
    rnk::println(
        Text::new(working_dir)
            .color(theme.text_muted)
            .into_element(),
    );
    rnk::println(
        Text::new("─".repeat(term_width))
            .color(theme.separator)
            .into_element(),
    );
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
            ui_state.is_busy =
                !matches!(app_state.phase, ExecutionPhase::Idle | ExecutionPhase::Error { .. });
            if ui_state.is_busy {
                ui_state.status_text = app_state.status_text();
                // Increment animation frame for spinner
                ui_state.animation_frame = ui_state.animation_frame.wrapping_add(1);
            } else {
                ui_state.status_text.clear();
            }

            // Check for tool execution start - print tool info when a new tool starts
            let tool_start_work = if let Some(ref tool_exec) = app_state.tool_execution {
                let tool_key = format!("{}:{}", tool_exec.tool_name, tool_exec.description);
                if ui_state.current_tool_printed.as_ref() != Some(&tool_key) {
                    ui_state.current_tool_printed = Some(tool_key);
                    Some(format_tool_start(
                        &tool_exec.tool_name,
                        &tool_exec.description,
                        theme,
                    ))
                } else {
                    None
                }
            } else {
                // Tool finished, clear the tracking
                ui_state.current_tool_printed = None;
                None
            };

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
            let new_messages: Vec<_> = if new_count > ui_state.printed_count {
                let msgs: Vec<_> = messages
                    .iter()
                    .skip(ui_state.printed_count)
                    .map(|msg| format_message(msg, theme))
                    .collect();
                ui_state.printed_count = new_count;
                msgs
            } else {
                Vec::new()
            };

            (tool_start_work, error_work, new_messages)
        }; // Lock released here

        // Process all I/O outside the lock
        let (tool_start_work, error_work, new_messages) = pending_work;

        // Print tool start info
        if let Some(tool_element) = tool_start_work {
            rnk::println(tool_element);
        }

        if let Some(error) = error_work {
            rnk::println(error);
            rnk::println(""); // Empty line
        }

        for msg_element in new_messages {
            rnk::println(msg_element);
            rnk::println(""); // Empty line
        }

        // Request render to update spinner animation
        rnk::request_render();
    }
}
