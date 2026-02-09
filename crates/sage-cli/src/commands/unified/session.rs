//! Session management for the unified command

use crate::console::CliConsole;
use sage_core::agent::UnifiedExecutor;
use sage_core::config::Config;
use sage_core::error::{SageError, SageResult};
use sage_core::session::JsonlSessionStorage;
use sage_core::trajectory::SessionRecorder;
use sage_core::types::TaskMetadata;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use super::args::UnifiedArgs;
use super::input::handle_user_input;
use super::outcome::display_outcome;
use super::slash_commands::{SlashCommandAction, process_slash_command};
use sage_core::input::InputChannel;

/// Execute a single task (one-shot mode)
pub async fn execute_single_task(
    executor: &mut UnifiedExecutor,
    console: &CliConsole,
    working_dir: &std::path::Path,
    _jsonl_storage: &Arc<JsonlSessionStorage>,
    session_recorder: &Option<Arc<Mutex<SessionRecorder>>>,
    task_description: &str,
    config_file: &str,
) -> SageResult<()> {
    // Process slash commands if needed
    let action = process_slash_command(task_description, console, working_dir).await?;

    // Handle the command action
    let task_description = match action {
        SlashCommandAction::Prompt(desc) => desc,
        SlashCommandAction::Handled => return Ok(()),
        SlashCommandAction::HandledWithOutput(output) => {
            println!("{}", output);
            return Ok(());
        }
        SlashCommandAction::SetOutputMode(mode) => {
            executor.set_output_mode(mode);
            console.info("Output mode updated.");
            return Ok(());
        }
        SlashCommandAction::Resume { session_id } => {
            let resume_result: SageResult<String> = if let Some(id) = session_id {
                executor
                    .restore_session(&id)
                    .await
                    .map(|msgs| format!("Session {} restored ({} messages)", id, msgs.len()))
            } else {
                match executor.get_most_recent_session().await? {
                    Some(metadata) => {
                        let id = metadata.id;
                        let msgs = executor.restore_session(&id).await?;
                        Ok(format!("Session {} restored ({} messages)", id, msgs.len()))
                    }
                    None => Err(SageError::config(
                        "No previous sessions found. Start a new session first.",
                    )),
                }
            };

            match resume_result {
                Ok(msg) => console.success(&msg),
                Err(e) => console.error(&format!("Resume failed: {}", e)),
            }
            return Ok(());
        }
        SlashCommandAction::SwitchModel { model } => {
            match executor.switch_model(&model) {
                Ok(_) => console.success(&format!("Switched to model: {}", model)),
                Err(e) => console.error(&format!("Failed to switch model: {}", e)),
            }
            return Ok(());
        }
        SlashCommandAction::ModelSelect { models } => {
            // In non-interactive mode, just show the list
            let mut output = "Available models:\n".to_string();
            for model in &models {
                output.push_str(&format!("  - {}\n", model));
            }
            output.push_str("\nUse /model <name> to switch.");
            println!("{}", output);
            return Ok(());
        }
        SlashCommandAction::Doctor => {
            // Run the doctor command
            crate::commands::diagnostics::doctor(config_file).await?;
            return Ok(());
        }
        SlashCommandAction::Exit => {
            console.info("Exiting...");
            return Ok(());
        }
    };

    // Execute the task
    let task = TaskMetadata::new(&task_description, &working_dir.display().to_string());
    let start_time = std::time::Instant::now();
    let outcome = executor.execute(task).await?;
    let duration = start_time.elapsed();

    // Display results
    console.print_separator();
    let session_path = if let Some(recorder) = session_recorder {
        Some(recorder.lock().await.file_path().to_path_buf())
    } else {
        None
    };
    display_outcome(console, &outcome, duration, session_path.as_ref());

    Ok(())
}

/// Execute session resume (-c or -r flags)
pub async fn execute_session_resume(
    args: UnifiedArgs,
    mut executor: UnifiedExecutor,
    console: CliConsole,
    config: Config,
    working_dir: PathBuf,
    config_file: String,
) -> SageResult<()> {
    struct InputTaskGuard(Option<JoinHandle<()>>);

    impl Drop for InputTaskGuard {
        fn drop(&mut self) {
            if let Some(handle) = self.0.take() {
                handle.abort();
            }
        }
    }

    let mut input_task = InputTaskGuard(None);
    // Determine which session to resume
    let session_id = if let Some(id) = args.resume_session_id {
        id
    } else {
        match executor.get_most_recent_session().await? {
            Some(metadata) => {
                console.info(&format!(
                    "Resuming most recent session: {} ({})",
                    metadata.id,
                    metadata.resume_title()
                ));
                metadata.id
            }
            None => {
                return Err(SageError::config(
                    "No previous sessions found in this directory. Start a new session first.",
                ));
            }
        }
    };

    console.print_header("Session Resume");
    console.info(&format!("Resuming session: {}", session_id));

    // Restore the session
    let _restored_messages = executor.restore_session(&session_id).await?;
    console.success("Session restored successfully");

    // Set up session recording
    let session_recorder = if config.trajectory.is_enabled() {
        let recorder = sage_core::trajectory::init_session_recorder(&working_dir);
        if let Some(ref r) = recorder {
            executor.set_session_recorder(r.clone());
        }
        recorder
    } else {
        None
    };

    // Print session info
    console.info(&format!("Provider: {}", config.get_default_provider()));
    let max_steps_display = match executor.options().max_steps {
        Some(n) => n.to_string(),
        None => "unlimited".to_string(),
    };
    console.info(&format!("Max Steps: {}", max_steps_display));
    console.print_separator();

    // Read initial input BEFORE setting up InputChannel to avoid stdin competition
    console.info("Enter your next message to continue the conversation (press Enter to submit):");
    print!("> ");
    let _ = std::io::Write::flush(&mut std::io::stdout());

    // Use spawn_blocking to avoid blocking the async runtime
    let next_message = tokio::task::spawn_blocking(|| {
        let mut input = String::new();
        match std::io::stdin().read_line(&mut input) {
            Ok(0) => String::new(), // EOF
            Ok(_) => input.trim().to_string(),
            Err(_) => String::new(),
        }
    })
    .await
    .unwrap_or_default();

    if next_message.is_empty() {
        console.info("No input provided. Session ready for future continuation.");
        return Ok(());
    }

    // Now set up input channel for interactive mode (after initial input is read)
    let verbose = args.verbose;
    if !args.non_interactive {
        let (input_channel, input_handle) = InputChannel::new(16);
        executor.set_input_channel(input_channel);
        input_task.0 = Some(tokio::spawn(handle_user_input(input_handle, verbose)));
    }

    // Create task metadata with the new message
    // Process slash commands before executing resumed input
    let working_dir_path = std::path::Path::new(&working_dir);
    let action = process_slash_command(&next_message, &console, working_dir_path).await?;
    let task_description = match action {
        SlashCommandAction::Prompt(desc) => desc,
        SlashCommandAction::Handled => return Ok(()),
        SlashCommandAction::HandledWithOutput(output) => {
            println!("{}", output);
            return Ok(());
        }
        SlashCommandAction::SetOutputMode(mode) => {
            executor.set_output_mode(mode);
            console.info("Output mode updated.");
            return Ok(());
        }
        SlashCommandAction::Resume { session_id } => {
            let resume_result = if let Some(id) = session_id {
                executor
                    .restore_session(&id)
                    .await
                    .map(|_| format!("Session {} restored", id))
            } else {
                match executor.get_most_recent_session().await? {
                    Some(metadata) => {
                        let id = metadata.id;
                        executor.restore_session(&id).await?;
                        Ok(format!("Session {} restored", id))
                    }
                    None => Err(SageError::config(
                        "No previous sessions found. Start a new session first.",
                    )),
                }
            };

            match resume_result {
                Ok(msg) => console.success(&msg),
                Err(e) => console.error(&format!("Resume failed: {}", e)),
            }
            return Ok(());
        }
        SlashCommandAction::SwitchModel { model } => {
            match executor.switch_model(&model) {
                Ok(_) => console.success(&format!("Switched to model: {}", model)),
                Err(e) => console.error(&format!("Failed to switch model: {}", e)),
            }
            return Ok(());
        }
        SlashCommandAction::ModelSelect { models } => {
            let mut output = "Available models:\n".to_string();
            for model in &models {
                output.push_str(&format!("  - {}\n", model));
            }
            output.push_str("\nUse /model <name> to switch.");
            println!("{}", output);
            return Ok(());
        }
        SlashCommandAction::Doctor => {
            crate::commands::diagnostics::doctor(&config_file).await?;
            return Ok(());
        }
        SlashCommandAction::Exit => {
            console.info("Exiting...");
            return Ok(());
        }
    };

    let task = TaskMetadata::new(&task_description, &working_dir.display().to_string());

    // Execute the task
    let start_time = std::time::Instant::now();
    let outcome = executor.execute(task).await?;
    let duration = start_time.elapsed();

    // Display results
    console.print_separator();
    let session_path = if let Some(recorder) = &session_recorder {
        Some(recorder.lock().await.file_path().to_path_buf())
    } else {
        None
    };
    display_outcome(&console, &outcome, duration, session_path.as_ref());

    Ok(())
}
