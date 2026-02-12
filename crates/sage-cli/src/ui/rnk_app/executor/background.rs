//! Background UI rendering loop

use super::super::components::{format_message, format_tool_start, render_error};
use super::super::state::SharedState;
use super::super::theme::current_theme;
use rnk::prelude::*;
use sage_core::ui::bridge::state::ExecutionPhase;
use tokio::time::{Duration, sleep};
use unicode_width::UnicodeWidthStr;

/// Background thread logic for printing messages and updating UI
pub async fn background_loop(state: SharedState, adapter: sage_core::ui::bridge::EventAdapter) {
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
                            sage_core::ui::bridge::state::UiMessageContent::ToolCall { .. }
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
