//! Command routing logic for CLI

use crate::args::{Cli, Commands, ConfigAction, TrajectoryAction};
use crate::{claude_mode, commands, ipc, ui_launcher};
use sage_core::error::SageResult;

/// Route CLI commands to their respective handlers
pub async fn route(cli: Cli) -> SageResult<()> {
    match cli.command {
        // DEFAULT BEHAVIOR: If no subcommand is provided, start interactive mode
        None => route_default(cli).await,
        Some(Commands::Run { .. }) => route_run(cli).await,
        Some(Commands::Interactive { .. }) => route_interactive(cli).await,
        Some(Commands::Config { action }) => route_config(action).await,
        Some(Commands::Trajectory { action }) => route_trajectory(action).await,
        Some(Commands::Tools) => commands::tools::show_tools().await,
        Some(Commands::Unified { .. }) => route_unified(cli).await,
        Some(Commands::Ipc { config_file }) => ipc::run_ipc_mode(Some(&config_file)).await,
    }
}

async fn route_default(cli: Cli) -> SageResult<()> {
    if cli.modern_ui {
        ui_launcher::launch_modern_ui(
            &cli.config_file,
            cli.trajectory_file.as_ref().and_then(|p| p.to_str()),
            cli.working_dir.as_ref().and_then(|p| p.to_str()),
        )
        .await
    } else {
        commands::interactive::execute(commands::interactive::InteractiveArgs {
            config_file: cli.config_file,
            trajectory_file: cli.trajectory_file,
            working_dir: cli.working_dir,
        })
        .await
    }
}

async fn route_run(cli: Cli) -> SageResult<()> {
    if let Some(Commands::Run {
        task,
        provider,
        model,
        model_base_url,
        api_key,
        max_steps,
        working_dir,
        config_file,
        trajectory_file,
        patch_path,
        must_patch,
        verbose,
        modern_ui: _,
    }) = cli.command
    {
        commands::run::execute(commands::run::RunArgs {
            task,
            provider,
            model,
            model_base_url,
            api_key,
            max_steps,
            working_dir,
            config_file,
            trajectory_file,
            patch_path,
            must_patch,
            verbose,
        })
        .await
    } else {
        unreachable!()
    }
}

async fn route_interactive(cli: Cli) -> SageResult<()> {
    if let Some(Commands::Interactive {
        config_file,
        trajectory_file,
        working_dir,
        verbose: _,
        modern_ui,
        claude_style,
    }) = cli.command
    {
        if claude_style {
            claude_mode::run_claude_interactive(&config_file).await
        } else if modern_ui {
            ui_launcher::launch_modern_ui(
                &config_file,
                trajectory_file.as_ref().and_then(|p| p.to_str()),
                working_dir.as_ref().and_then(|p| p.to_str()),
            )
            .await
        } else {
            commands::interactive::execute(commands::interactive::InteractiveArgs {
                config_file,
                trajectory_file,
                working_dir,
            })
            .await
        }
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

async fn route_unified(cli: Cli) -> SageResult<()> {
    if let Some(Commands::Unified {
        task,
        config_file,
        working_dir,
        max_steps,
        verbose,
        non_interactive,
    }) = cli.command
    {
        commands::unified_execute(commands::UnifiedArgs {
            task,
            config_file,
            working_dir,
            max_steps,
            verbose,
            non_interactive,
        })
        .await
    } else {
        unreachable!()
    }
}
