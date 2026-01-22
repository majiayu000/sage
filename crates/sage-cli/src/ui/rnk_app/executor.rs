//! Executor logic for rnk app

use super::state::{SharedState, UiCommand};
use rnk::prelude::*;
use sage_core::agent::{ExecutionMode, ExecutionOptions, UnifiedExecutor};
use sage_core::config::load_config;
use sage_core::error::SageResult;
use sage_core::input::InputChannel;
use sage_core::output::OutputMode;
use sage_core::types::TaskMetadata;
use sage_core::ui::bridge::state::ExecutionPhase;
use sage_core::ui::bridge::{emit_event, AgentEvent};
use sage_tools::get_default_tools;
use tokio::sync::mpsc;

/// Create executor with default configuration
pub async fn create_executor() -> SageResult<UnifiedExecutor> {
    let config = load_config()?;
    let working_dir = std::env::current_dir().unwrap_or_default();
    let mode = ExecutionMode::interactive();
    let options = ExecutionOptions::default()
        .with_mode(mode)
        .with_working_directory(&working_dir);

    let mut executor = UnifiedExecutor::with_options(config, options)?;
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
) {
    // Create executor
    let mut executor = match create_executor().await {
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
                {
                    let mut s = state.write();
                    s.is_busy = true;
                    s.status_text = "Thinking...".to_string();
                }
                rnk::request_render();

                emit_event(AgentEvent::UserInputReceived { input: task.clone() });
                emit_event(AgentEvent::ThinkingStarted);

                // Execute task
                let working_dir = std::env::current_dir()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let task_meta = TaskMetadata::new(&task, &working_dir);

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
pub fn background_loop(
    state: SharedState,
    adapter: sage_core::ui::bridge::EventAdapter,
) {
    use super::components::{format_message, render_header};

    loop {
        std::thread::sleep(std::time::Duration::from_millis(80));

        // Check if should quit
        {
            let s = state.read();
            if s.should_quit {
                break;
            }
        }

        // Check for new messages and print them
        {
            let app_state = adapter.get_state();
            let messages = app_state.display_messages();
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

            // Print header once (after session info is available)
            if !ui_state.header_printed {
                // Wait for session info or print with defaults after a short delay
                let has_session_info = ui_state.session.model != "unknown";
                if has_session_info || ui_state.printed_count > 0 {
                    drop(ui_state);
                    let s = state.read();
                    rnk::println(render_header(&s.session));
                    rnk::println(""); // Empty line
                    drop(s);
                    let mut s = state.write();
                    s.header_printed = true;
                    ui_state = s;
                }
            }

            // Update busy state from adapter - Error state is not busy
            ui_state.is_busy = !matches!(app_state.phase, ExecutionPhase::Idle | ExecutionPhase::Error { .. });
            if ui_state.is_busy && ui_state.status_text.is_empty() {
                ui_state.status_text = app_state.status_text();
            }

            // Check for error state and display error message
            if let ExecutionPhase::Error { ref message } = app_state.phase {
                if !ui_state.error_displayed {
                    ui_state.error_displayed = true;
                    drop(ui_state);
                    rnk::println(
                        Text::new(format!("Error: {}", message))
                            .color(Color::Red)
                            .bold()
                            .into_element(),
                    );
                    rnk::println(""); // Empty line
                    ui_state = state.write();
                }
            } else {
                // Reset error_displayed flag when not in error state
                ui_state.error_displayed = false;
            }

            // Print new messages
            if new_count > ui_state.printed_count {
                for msg in messages.iter().skip(ui_state.printed_count) {
                    drop(ui_state);
                    rnk::println(format_message(msg));
                    rnk::println(""); // Empty line
                    ui_state = state.write();
                }
                ui_state.printed_count = new_count;
            }
        }

        // Request render to update spinner animation
        rnk::request_render();
    }
}
