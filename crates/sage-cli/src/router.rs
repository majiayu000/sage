//! Command routing logic for CLI
//!
//! Unified routing: all execution modes go through UnifiedExecutor

use crate::args::{Cli, Commands, ConfigAction, TrajectoryAction};
use crate::{commands, ipc, ui_launcher};
use sage_core::error::SageResult;

/// Route CLI commands to their respective handlers
pub async fn route(cli: Cli) -> SageResult<()> {
    // Handle subcommands first (utility commands and legacy support)
    if let Some(command) = &cli.command {
        return match command {
            // Utility commands
            Commands::Config { action } => route_config(action.clone()).await,
            Commands::Trajectory { action } => route_trajectory(action.clone()).await,
            Commands::Tools => commands::tools::show_tools().await,
            Commands::Ipc { config_file } => ipc::run_ipc_mode(Some(config_file)).await,

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
    // Determine execution mode
    let non_interactive = cli.print_mode;

    // Use modern UI if requested
    if cli.modern_ui {
        return ui_launcher::launch_modern_ui(
            &cli.config_file,
            None, // trajectory_file
            cli.working_dir.as_ref().and_then(|p| p.to_str()),
        )
        .await;
    }

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
    })
    .await
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
        modern_ui: _,
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
        modern_ui,
    }) = &cli.command
    {
        if *modern_ui {
            ui_launcher::launch_modern_ui(
                config_file,
                None,
                working_dir.as_ref().and_then(|p| p.to_str()),
            )
            .await
        } else {
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
            })
            .await
        }
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
