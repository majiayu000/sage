//! Interactive REPL loop for the unified command

use crate::console::CliConsole;
use crate::ui::NerdConsole;
use sage_core::agent::UnifiedExecutor;
use sage_core::config::Config;
use sage_core::error::SageResult;
use sage_core::session::JsonlSessionStorage;
use sage_core::types::TaskMetadata;
use std::io::Write;
use std::sync::Arc;

use super::input::read_input_raw;
use super::session::resume_session_inline;
use super::slash_commands::{process_slash_command, SlashCommandAction};
use super::utils::{get_git_branch, show_recent_activity_nerd};

/// Interactive REPL loop (Claude Code style) with Nerd Font UI
pub async fn execute_interactive_loop(
    executor: &mut UnifiedExecutor,
    console: &CliConsole,
    config: &Config,
    working_dir: &std::path::Path,
    jsonl_storage: &Arc<JsonlSessionStorage>,
) -> SageResult<()> {
    let nerd = NerdConsole::new();

    let model = config
        .model_providers
        .get(&config.default_provider)
        .map(|p| p.model.as_str())
        .unwrap_or("unknown");

    let git_branch = get_git_branch(working_dir);
    let dir_display = working_dir.display().to_string();

    nerd.print_header(model, git_branch.as_deref(), &dir_display);
    show_recent_activity_nerd(&nerd, jsonl_storage).await;
    nerd.info("Type your message, or /help for commands. Press Ctrl+C to exit.");
    println!();

    loop {
        nerd.print_prompt();

        let input = match read_input_raw() {
            Ok(input) => input,
            Err(e) => {
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::UnexpectedEof | std::io::ErrorKind::Interrupted
                ) {
                    println!();
                    nerd.info("Goodbye!");
                    break;
                }
                nerd.error(&format!("Input error: {}", e));
                continue;
            }
        };

        if input.is_empty() {
            continue;
        }

        // Handle built-in commands
        if let Some(action) =
            handle_builtin(&input, &nerd, model, git_branch.as_deref(), &dir_display).await
        {
            match action {
                BuiltinAction::Continue => continue,
                BuiltinAction::Exit => break,
            }
        }

        // Process slash commands
        let action = match process_slash_command(&input, console, working_dir, jsonl_storage).await
        {
            Ok(action) => action,
            Err(e) => {
                nerd.error(&format!("Command error: {}", e));
                continue;
            }
        };

        let task_description = match action {
            SlashCommandAction::Prompt(desc) => desc,
            SlashCommandAction::Handled => continue,
            SlashCommandAction::ResumeSession(session_id) => {
                match resume_session_inline(executor, &session_id, jsonl_storage, &nerd).await {
                    Ok(_) => nerd.success("Session resumed. Continue your conversation."),
                    Err(e) => nerd.error(&format!("Failed to resume session: {}", e)),
                }
                continue;
            }
        };

        let task = TaskMetadata::new(&task_description, &working_dir.display().to_string());
        let start_time = std::time::Instant::now();

        match executor.execute(task).await {
            Ok(outcome) => {
                let duration = start_time.elapsed();
                let usage = &outcome.execution().total_usage;
                nerd.print_summary(
                    outcome.is_success(),
                    outcome.execution().steps.len(),
                    usage.prompt_tokens as u64,
                    usage.completion_tokens as u64,
                    duration.as_secs_f64(),
                );
            }
            Err(e) => nerd.error(&format!("Execution error: {}", e)),
        }
    }

    Ok(())
}

enum BuiltinAction {
    Continue,
    Exit,
}

async fn handle_builtin(
    input: &str,
    nerd: &NerdConsole,
    model: &str,
    git_branch: Option<&str>,
    dir_display: &str,
) -> Option<BuiltinAction> {
    if matches!(input, "/exit" | "/quit" | "exit" | "quit" | "q") {
        nerd.info("Goodbye!");
        return Some(BuiltinAction::Exit);
    }

    if matches!(input, "/clear" | "clear" | "cls") {
        print!("\x1B[2J\x1B[1;1H\x1B[3J");
        std::io::stdout().flush().ok();
        nerd.print_header(model, git_branch, dir_display);
        nerd.success("Conversation cleared.");
        return Some(BuiltinAction::Continue);
    }

    if matches!(input, "/help" | "help" | "?") {
        nerd.print_help();
        return Some(BuiltinAction::Continue);
    }

    if matches!(input, "/login" | "login") {
        use crate::commands::interactive::CliOnboarding;
        let mut onboarding = CliOnboarding::new();
        match onboarding.run_login().await {
            Ok(true) => {
                nerd.success("API key updated! Restart sage to use the new key.");
                return Some(BuiltinAction::Exit);
            }
            Ok(false) => nerd.info("API key not changed."),
            Err(e) => nerd.error(&format!("Login failed: {}", e)),
        }
        return Some(BuiltinAction::Continue);
    }

    if matches!(input, "/logout" | "logout") {
        if let Some(home) = dirs::home_dir() {
            let creds_path = home.join(".sage/credentials.json");
            if creds_path.exists() {
                match std::fs::remove_file(&creds_path) {
                    Ok(_) => nerd.success("Credentials cleared. Run /login to configure."),
                    Err(e) => nerd.error(&format!("Failed to remove credentials: {}", e)),
                }
            } else {
                nerd.info("No credentials file found.");
            }
        }
        return Some(BuiltinAction::Continue);
    }

    None
}
