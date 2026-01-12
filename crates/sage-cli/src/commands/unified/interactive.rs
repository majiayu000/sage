//! Interactive REPL loop for the unified command

use crate::console::CliConsole;
use colored::*;
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
use super::utils::get_git_branch;

/// Interactive REPL loop (legacy mode)
pub async fn execute_interactive_loop(
    executor: &mut UnifiedExecutor,
    console: &CliConsole,
    config: &Config,
    working_dir: &std::path::Path,
    jsonl_storage: &Arc<JsonlSessionStorage>,
) -> SageResult<()> {
    let model = config
        .model_providers
        .get(&config.default_provider)
        .map(|p| p.model.as_str())
        .unwrap_or("unknown");

    let git_branch = get_git_branch(working_dir);
    let dir_display = working_dir.display().to_string();

    // Print header
    println!();
    println!(
        "  {} sage    {}    {}   {}",
        "󰚩".cyan(),
        git_branch.as_deref().unwrap_or("no-branch").yellow(),
        dir_display.dimmed(),
        format!("󰧑 {}", model).blue()
    );
    println!("{}", "━".repeat(80).dimmed());
    println!();

    // Show recent sessions
    if let Ok(sessions) = jsonl_storage.list_sessions().await {
        if !sessions.is_empty() {
            println!("   {}", "Recent Sessions".bold());
            println!();
            for (i, session) in sessions.iter().take(5).enumerate() {
                let prefix = if i == sessions.len().min(5) - 1 { "└──" } else { "├──" };
                let time_ago = super::utils::format_time_ago(&session.updated_at);
                println!(
                    "  {} 󰆍 {} ({}, {} msgs)",
                    prefix.dimmed(),
                    session.display_title(),
                    time_ago.dimmed(),
                    session.message_count
                );
            }
            println!();
        }
    }

    console.info("Type your message, or /help for commands. Press Ctrl+C to exit.");
    println!();

    loop {
        print!("  {} ", "sage ❯".cyan().bold());
        std::io::stdout().flush().ok();

        let input = match read_input_raw() {
            Ok(input) => input,
            Err(e) => {
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::UnexpectedEof | std::io::ErrorKind::Interrupted
                ) {
                    println!();
                    console.info("Goodbye!");
                    break;
                }
                console.error(&format!("Input error: {}", e));
                continue;
            }
        };

        if input.is_empty() {
            continue;
        }

        // Handle built-in commands
        if let Some(action) =
            handle_builtin(&input, console, model, git_branch.as_deref(), &dir_display).await
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
                console.error(&format!("Command error: {}", e));
                continue;
            }
        };

        let task_description = match action {
            SlashCommandAction::Prompt(desc) => desc,
            SlashCommandAction::Handled => continue,
            SlashCommandAction::ResumeSession(session_id) => {
                match resume_session_inline(executor, &session_id, jsonl_storage, console).await {
                    Ok(_) => console.success("Session resumed. Continue your conversation."),
                    Err(e) => console.error(&format!("Failed to resume session: {}", e)),
                }
                continue;
            }
            SlashCommandAction::SetOutputMode(mode) => {
                executor.set_output_mode(mode.clone());
                console.success(&format!("Output mode set to: {:?}", mode));
                continue;
            }
        };

        let task = TaskMetadata::new(&task_description, &working_dir.display().to_string());
        let start_time = std::time::Instant::now();

        match executor.execute(task).await {
            Ok(outcome) => {
                let duration = start_time.elapsed();
                let usage = &outcome.execution().total_usage;
                // Print summary
                let status = if outcome.is_success() { "✓".green() } else { "✗".red() };
                println!();
                println!(
                    "  {} {} steps | {} in | {} out | {:.1}s",
                    status,
                    outcome.execution().steps.len(),
                    usage.prompt_tokens,
                    usage.completion_tokens,
                    duration.as_secs_f64()
                );
                println!();
            }
            Err(e) => console.error(&format!("Execution error: {}", e)),
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
    console: &CliConsole,
    model: &str,
    git_branch: Option<&str>,
    dir_display: &str,
) -> Option<BuiltinAction> {
    if matches!(input, "/exit" | "/quit" | "exit" | "quit" | "q") {
        console.info("Goodbye!");
        return Some(BuiltinAction::Exit);
    }

    if matches!(input, "/clear" | "clear" | "cls") {
        print!("\x1B[2J\x1B[1;1H\x1B[3J");
        std::io::stdout().flush().ok();
        // Reprint header
        println!();
        println!(
            "  {} sage    {}    {}   {}",
            "󰚩".cyan(),
            git_branch.unwrap_or("no-branch").yellow(),
            dir_display.dimmed(),
            format!("󰧑 {}", model).blue()
        );
        println!("{}", "━".repeat(80).dimmed());
        console.success("Conversation cleared.");
        return Some(BuiltinAction::Continue);
    }

    if matches!(input, "/help" | "help" | "?") {
        println!();
        println!("  {}", "Available Commands".bold());
        println!("  {} Show this help", "/help".cyan());
        println!("  {} Clear conversation", "/clear".cyan());
        println!("  {} Resume a previous session", "/resume".cyan());
        println!("  {} Configure API key", "/login".cyan());
        println!("  {} Exit", "/exit".cyan());
        println!();
        return Some(BuiltinAction::Continue);
    }

    if matches!(input, "/login" | "login") {
        use crate::commands::interactive::CliOnboarding;
        let mut onboarding = CliOnboarding::new();
        match onboarding.run_login().await {
            Ok(true) => {
                console.success("API key updated! Restart sage to use the new key.");
                return Some(BuiltinAction::Exit);
            }
            Ok(false) => console.info("API key not changed."),
            Err(e) => console.error(&format!("Login failed: {}", e)),
        }
        return Some(BuiltinAction::Continue);
    }

    if matches!(input, "/logout" | "logout") {
        if let Some(home) = dirs::home_dir() {
            let creds_path = home.join(".sage/credentials.json");
            if creds_path.exists() {
                match std::fs::remove_file(&creds_path) {
                    Ok(_) => console.success("Credentials cleared. Run /login to configure."),
                    Err(e) => console.error(&format!("Failed to remove credentials: {}", e)),
                }
            } else {
                console.info("No credentials file found.");
            }
        }
        return Some(BuiltinAction::Continue);
    }

    None
}
