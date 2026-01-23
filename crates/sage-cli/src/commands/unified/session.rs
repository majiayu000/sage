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
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::Mutex;

use super::args::UnifiedArgs;
use super::input::handle_user_input;
use super::outcome::display_outcome;
use super::slash_commands::{process_slash_command, SlashCommandAction};
use sage_core::input::InputChannel;

/// Execute a single task (one-shot mode)
pub async fn execute_single_task(
    executor: &mut UnifiedExecutor,
    console: &CliConsole,
    working_dir: &std::path::Path,
    _jsonl_storage: &Arc<JsonlSessionStorage>,
    session_recorder: &Option<Arc<Mutex<SessionRecorder>>>,
    task_description: &str,
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
            // Resume is handled at a higher level, not here
            console.warn(&format!(
                "Resume command should be handled at session level. Use `sage -c` or `sage -r {}`.",
                session_id.as_deref().unwrap_or("<id>")
            ));
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
) -> SageResult<()> {
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
        match SessionRecorder::new(&working_dir) {
            Ok(recorder) => {
                let recorder = Arc::new(Mutex::new(recorder));
                executor.set_session_recorder(recorder.clone());
                Some(recorder)
            }
            Err(e) => {
                console.warn(&format!("Failed to initialize session recorder: {}", e));
                None
            }
        }
    } else {
        None
    };

    // Set up input channel for interactive mode
    let verbose = args.verbose;
    if !args.non_interactive {
        let (input_channel, input_handle) = InputChannel::new(16);
        executor.set_input_channel(input_channel);
        tokio::spawn(handle_user_input(input_handle, verbose));
    }

    // Print session info
    console.info(&format!("Provider: {}", config.get_default_provider()));
    let max_steps_display = match executor.options().max_steps {
        Some(n) => n.to_string(),
        None => "unlimited".to_string(),
    };
    console.info(&format!("Max Steps: {}", max_steps_display));
    console.print_separator();

    // Prompt for next user input
    console.info("Enter your next message to continue the conversation (Ctrl+D to finish):");
    let mut input = String::new();
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    while reader.read_line(&mut input).await? > 0 {}
    let next_message = input.trim().to_string();

    if next_message.is_empty() {
        console.info("No input provided. Session ready for future continuation.");
        return Ok(());
    }

    // Create task metadata with the new message
    let task = TaskMetadata::new(&next_message, &working_dir.display().to_string());

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
