//! Command routing logic for CLI
//!
//! Unified routing: all execution modes go through UnifiedExecutor

use crate::args::{Cli, Commands, ConfigAction, TrajectoryAction};
use crate::commands::interactive::{CliOnboarding, check_config_status};
use crate::commands::unified::OutputModeArg;
use crate::console::CliConsole;
use crate::{app, commands};
use sage_core::config::credential::ConfigStatus;
use sage_core::error::SageResult;
use std::io::IsTerminal;

/// Route CLI commands to their respective handlers
pub async fn route(cli: Cli) -> SageResult<()> {
    // Handle subcommands first (utility commands and legacy support)
    if let Some(command) = &cli.command {
        return match command {
            // Utility commands
            Commands::Config { action } => route_config(action.clone()).await,
            Commands::Trajectory { action } => route_trajectory(action.clone()).await,
            Commands::Tools => commands::tools::show_tools().await,

            // Diagnostic commands
            Commands::Doctor { config_file } => {
                commands::diagnostics::doctor(config_file).await
            }
            Commands::Status { config_file } => {
                commands::diagnostics::status(config_file).await
            }
            Commands::Usage {
                session_dir,
                detailed,
            } => commands::diagnostics::usage(session_dir.as_deref(), *detailed).await,

            // Legacy commands (hidden but still supported for backward compatibility)
            Commands::Run { .. } => route_legacy_run(&cli).await,
            Commands::Interactive { .. } => route_legacy_interactive(&cli).await,
            Commands::Unified { .. } => route_legacy_unified(&cli).await,
        };
    }

    // Main unified execution path
    route_main(cli).await
}

/// Main execution route - unified entry point for all execution modes
async fn route_main(cli: Cli) -> SageResult<()> {
    // Check configuration status and run onboarding if needed
    let (config_status, _status_hint) = check_config_status();
    if config_status == ConfigStatus::Unconfigured {
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

    // Determine execution mode
    let non_interactive = cli.print_mode;

    // Check if stdin is a TTY (required for rnk App mode to read user input)
    let is_tty = std::io::stdin().is_terminal();

    // Debug: log TTY detection
    tracing::debug!(
        "TTY detection: is_tty={}, non_interactive={}",
        is_tty,
        non_interactive
    );

    // Use non-UI execution if:
    // 1. Print mode (non-interactive)
    // 2. Not running in a TTY (rnk requires interactive terminal)
    // Otherwise use new rnk UI as default
    if non_interactive || !is_tty {
        // Execute using UnifiedExecutor (the single execution path)
        // Session resume is handled by unified_execute when continue_recent or resume_session_id is set
        commands::unified_execute(commands::UnifiedArgs {
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
        .await
    } else {
        // New rnk App mode is the default (fullscreen with fixed-bottom layout)
        app::run_app_mode().await.map_err(|e| sage_core::error::SageError::Io {
            message: e.to_string(),
            path: None,
            context: Some("Running rnk App mode".to_string()),
        })
    }
}

/// Route legacy `sage run "task"` command
async fn route_legacy_run(cli: &Cli) -> SageResult<()> {
    if let Some(Commands::Run {
        task,
        provider: _,
        model: _,
        model_base_url: _,
        api_key: _,
        max_steps,
        working_dir,
        config_file,
        trajectory_file: _,
        patch_path: _,
        must_patch: _,
        verbose,
    }) = &cli.command
    {
        // Route to unified executor in non-interactive mode
        commands::unified_execute(commands::UnifiedArgs {
            task: Some(task.clone()),
            config_file: config_file.clone(),
            working_dir: working_dir.clone(),
            max_steps: *max_steps,
            verbose: *verbose,
            non_interactive: true, // Legacy run is always non-interactive
            resume_session_id: None,
            continue_recent: false,
            stream_json: false,
            output_mode: OutputModeArg::default(),
        })
        .await
    } else {
        unreachable!()
    }
}

/// Route legacy `sage interactive` command
async fn route_legacy_interactive(cli: &Cli) -> SageResult<()> {
    if let Some(Commands::Interactive {
        config_file,
        trajectory_file: _,
        working_dir,
        verbose,
    }) = &cli.command
    {
        // Route to unified executor in interactive mode
        commands::unified_execute(commands::UnifiedArgs {
            task: None,
            config_file: config_file.clone(),
            working_dir: working_dir.clone(),
            max_steps: None,
            verbose: *verbose,
            non_interactive: false,
            resume_session_id: None,
            continue_recent: false,
            stream_json: false,
            output_mode: OutputModeArg::default(),
        })
        .await
    } else {
        unreachable!()
    }
}

/// Route legacy `sage unified` command
async fn route_legacy_unified(cli: &Cli) -> SageResult<()> {
    if let Some(Commands::Unified {
        task,
        config_file,
        working_dir,
        max_steps,
        verbose,
        non_interactive,
    }) = &cli.command
    {
        commands::unified_execute(commands::UnifiedArgs {
            task: task.clone(),
            config_file: config_file.clone(),
            working_dir: working_dir.clone(),
            max_steps: *max_steps,
            verbose: *verbose,
            non_interactive: *non_interactive,
            resume_session_id: None,
            continue_recent: false,
            stream_json: false,
            output_mode: OutputModeArg::default(),
        })
        .await
    } else {
        unreachable!()
    }
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

async fn route_trajectory(action: TrajectoryAction) -> SageResult<()> {
    match action {
        TrajectoryAction::List { directory } => commands::trajectory::list(&directory).await,
        TrajectoryAction::Show { trajectory_file } => {
            commands::trajectory::show(&trajectory_file).await
        }
        TrajectoryAction::Stats { path } => commands::trajectory::stats(&path).await,
        TrajectoryAction::Analyze { path } => commands::trajectory::analyze(&path).await,
    }
}
