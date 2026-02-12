//! Command processing loop

use super::super::state::{SharedState, UiCommand};
use super::creation::create_executor;
use crate::commands::unified::slash_commands::{SlashCommandAction, process_slash_command};
use crate::console::CliConsole;
use rnk::prelude::*;
use sage_core::input::InputChannel;
use sage_core::interrupt::{
    InterruptReason, interrupt_current_task, reset_global_interrupt_manager,
};
use sage_core::types::TaskMetadata;
use sage_core::ui::bridge::AgentEvent;
use sage_core::ui::traits::UiContext;
use tokio::sync::mpsc;

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
                        handle_resume(&state, &mut executor, session_id).await;
                        continue;
                    }
                    Ok(SlashCommandAction::SwitchModel { model }) => {
                        handle_switch_model(&state, &mut executor, &model);
                        continue;
                    }
                    Ok(SlashCommandAction::ModelSelect { models }) => {
                        handle_model_select(&state, models);
                        continue;
                    }
                    Ok(SlashCommandAction::Doctor) => {
                        handle_doctor(&state, &config_file).await;
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

async fn handle_resume(
    state: &SharedState,
    executor: &mut sage_core::agent::UnifiedExecutor,
    session_id: Option<String>,
) {
    {
        let mut s = state.write();
        s.is_busy = true;
        s.status_text = "Resuming session...".to_string();
    }
    rnk::request_render();

    let resume_result = if let Some(id) = session_id {
        executor
            .restore_session(&id)
            .await
            .map(|msgs| format!("Session {} restored ({} messages)", id, msgs.len()))
    } else {
        match executor.get_most_recent_session().await {
            Ok(Some(metadata)) => {
                let id = metadata.id;
                match executor.restore_session(&id).await {
                    Ok(msgs) => Ok(format!("Session {} restored ({} messages)", id, msgs.len())),
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
}

fn handle_switch_model(
    state: &SharedState,
    executor: &mut sage_core::agent::UnifiedExecutor,
    model: &str,
) {
    match executor.switch_model(model) {
        Ok(_) => {
            {
                let mut s = state.write();
                s.session.model = model.to_string();
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
}

fn handle_model_select(state: &SharedState, models: Vec<String>) {
    {
        let mut s = state.write();
        s.model_select_mode = true;
        s.available_models = models;
        s.model_select_index = 0;
    }
    rnk::request_render();
}

async fn handle_doctor(state: &SharedState, config_file: &str) {
    {
        let mut s = state.write();
        s.is_busy = true;
        s.status_text = "Running diagnostics...".to_string();
    }
    rnk::request_render();

    let result = crate::commands::diagnostics::doctor(config_file).await;

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
}
