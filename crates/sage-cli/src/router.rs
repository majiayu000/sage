//! Command routing logic for CLI
//!
//! Unified routing: all execution modes go through UnifiedExecutor

use crate::args::{Cli, Commands, ConfigAction};
use crate::commands;
use crate::commands::interactive::{CliOnboarding, check_config_status};
use crate::console::CliConsole;
use crate::ui;
use sage_core::config::credential::ConfigStatus;
use sage_core::error::{SageError, SageResult};
use std::io::{IsTerminal, Read};

/// Route CLI commands to their respective handlers
pub async fn route(cli: Cli) -> SageResult<()> {
    // Handle subcommands first (utility commands)
    if let Some(command) = &cli.command {
        return match command {
            // Utility commands
            Commands::Config { action } => route_config(action.clone()).await,
            Commands::Tools => commands::tools::show_tools().await,

            // Diagnostic commands
            Commands::Doctor { config_file } => commands::diagnostics::doctor(config_file).await,
            Commands::Status { config_file } => commands::diagnostics::status(config_file).await,
            Commands::Usage {
                session_dir,
                detailed,
            } => commands::diagnostics::usage(session_dir.as_deref(), *detailed).await,
        };
    }

    // Main unified execution path
    route_main(cli).await
}

/// Main execution route - unified entry point for all execution modes
async fn route_main(mut cli: Cli) -> SageResult<()> {
    // Check configuration status and run onboarding if needed
    let (config_status, _status_hint) = check_config_status();

    // Check if stdin is a TTY (required for rnk App mode to read user input)
    let is_tty = std::io::stdin().is_terminal();

    // Only run onboarding if we're in a TTY (can interact with user)
    if config_status == ConfigStatus::Unconfigured && is_tty {
        let console = CliConsole::new(true);
        let mut onboarding = CliOnboarding::new();
        match onboarding.run().await {
            Ok(true) => {
                console.success("Setup complete! Starting sage...");
            }
            Ok(false) => {
                console.warn("Setup incomplete. You can run /login anytime to configure.");
            }
            Err(e) => {
                console.warn(&format!("Setup error: {}. Continuing anyway.", e));
            }
        }
    }

    // If stdin is a pipe and no task was provided, read task from stdin
    if !is_tty && cli.task.is_none() {
        let mut input = String::new();
        if std::io::stdin().read_to_string(&mut input).is_ok() {
            let trimmed = input.trim();
            if !trimmed.is_empty() {
                cli.task = Some(trimmed.to_string());
            }
        }
    }

    // Determine execution mode
    // When stdin is piped, force non-interactive mode
    let non_interactive = cli.print_mode || !is_tty;

    // Debug: log TTY detection
    tracing::debug!(
        "TTY detection: is_tty={}, non_interactive={}",
        is_tty,
        non_interactive
    );

    if non_interactive
        && cli.task.is_none()
        && !cli.continue_session
        && cli.resume_session.is_none()
    {
        return Err(SageError::invalid_input(
            "No task provided. Supply a task argument, pipe input, or use `-c` / `-r` to resume.",
        ));
    }

    let should_use_unified = non_interactive
        || cli.task.is_some()
        || cli.continue_session
        || cli.resume_session.is_some();

    if should_use_unified {
        // Execute using UnifiedExecutor (the single execution path)
        // Session resume is handled by unified_execute when continue_recent or resume_session_id is set
        return commands::unified_execute(commands::UnifiedArgs {
            task: cli.task,
            config_file: cli.config_file,
            working_dir: cli.working_dir,
            max_steps: cli.max_steps,
            verbose: cli.verbose,
            non_interactive,
            resume_session_id: cli.resume_session,
            continue_recent: cli.continue_session,
            stream_json: cli.stream_json,
            output_mode: cli.output_mode,
        })
        .await;
    }

    // No task/resume in TTY: run rnk app frontend.
    ui::run_rnk_app_with_cli(&cli)
        .await
        .map_err(|e| sage_core::error::SageError::Io {
            message: e.to_string(),
            path: None,
            context: Some("Running rnk App mode".to_string()),
        })
}

async fn route_config(action: ConfigAction) -> SageResult<()> {
    match action {
        ConfigAction::Show { config_file } => commands::config::show(&config_file).await,
        ConfigAction::Validate { config_file } => commands::config::validate(&config_file).await,
        ConfigAction::Init { config_file, force } => {
            commands::config::init(&config_file, force).await
        }
    }
}
