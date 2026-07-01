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
            Commands::Feedback {
                config_file,
                output,
                yes,
            } => commands::diagnostics::feedback(config_file, output, *yes).await,
        };
    }

    // Main unified execution path
    route_main(cli).await
}

/// Main execution route - unified entry point for all execution modes
async fn route_main(mut cli: Cli) -> SageResult<()> {
    // Initialize icon mode only for the main execution path.
    // Utility subcommands do not use the shared UI icon layer.
    sage_core::ui::init_icons();

    // Check if stdin is a TTY (required for rnk App mode to read user input)
    let is_tty = std::io::stdin().is_terminal();

    // Only check onboarding status for interactive TTY flows.
    if is_tty && !cli.print_mode {
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

    let decision = main_route_decision(&cli, is_tty)?;

    if decision.should_use_unified {
        let unified_args = unified_args_from_cli(cli, decision.non_interactive);
        // Execute using the runtime facade path. Session resume is handled by
        // unified_execute when continue_recent or resume_session_id is set.
        return commands::unified_execute(unified_args).await;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MainRouteDecision {
    non_interactive: bool,
    should_use_unified: bool,
}

fn main_route_decision(cli: &Cli, is_tty: bool) -> SageResult<MainRouteDecision> {
    let non_interactive = cli.print_mode || !is_tty;

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

    Ok(MainRouteDecision {
        non_interactive,
        should_use_unified,
    })
}

fn unified_args_from_cli(cli: Cli, non_interactive: bool) -> commands::UnifiedArgs {
    commands::UnifiedArgs {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::DEFAULT_CONFIG_FILE;
    use crate::commands::unified::OutputModeArg;
    use clap::Parser;

    fn parse_cli(args: &[&str]) -> Cli {
        match Cli::try_parse_from(args.iter().copied()) {
            Ok(cli) => cli,
            Err(err) => panic!("expected CLI args to parse: {err}"),
        }
    }

    #[test]
    fn print_mode_routes_to_unified_non_interactive_task() {
        let cli = parse_cli(&["sage", "-p", "summarize"]);
        let decision = match main_route_decision(&cli, true) {
            Ok(decision) => decision,
            Err(err) => panic!("expected route decision: {err}"),
        };

        assert_eq!(
            decision,
            MainRouteDecision {
                non_interactive: true,
                should_use_unified: true,
            }
        );

        let unified = unified_args_from_cli(cli, decision.non_interactive);
        assert_eq!(unified.task.as_deref(), Some("summarize"));
        assert!(unified.non_interactive);
        assert!(!unified.continue_recent);
        assert!(unified.resume_session_id.is_none());
        assert!(!unified.stream_json);
    }

    #[test]
    fn continue_and_resume_route_to_unified_with_session_identity() {
        let continue_cli = parse_cli(&["sage", "-c"]);
        let continue_decision = match main_route_decision(&continue_cli, true) {
            Ok(decision) => decision,
            Err(err) => panic!("expected continue route decision: {err}"),
        };
        let continue_args = unified_args_from_cli(continue_cli, continue_decision.non_interactive);
        assert!(continue_args.continue_recent);
        assert!(continue_args.resume_session_id.is_none());
        assert!(!continue_args.non_interactive);

        let resume_cli = parse_cli(&["sage", "-r", "session-123"]);
        let resume_decision = match main_route_decision(&resume_cli, true) {
            Ok(decision) => decision,
            Err(err) => panic!("expected resume route decision: {err}"),
        };
        let resume_args = unified_args_from_cli(resume_cli, resume_decision.non_interactive);
        assert!(!resume_args.continue_recent);
        assert_eq!(
            resume_args.resume_session_id.as_deref(),
            Some("session-123")
        );
        assert!(!resume_args.non_interactive);
    }

    #[test]
    fn stream_json_routes_to_unified_without_changing_legacy_flag() {
        let cli = parse_cli(&[
            "sage",
            "--stream-json",
            "--output-mode",
            "silent",
            "summarize",
        ]);
        let decision = match main_route_decision(&cli, false) {
            Ok(decision) => decision,
            Err(err) => panic!("expected stream-json route decision: {err}"),
        };
        let unified = unified_args_from_cli(cli, decision.non_interactive);

        assert!(decision.should_use_unified);
        assert!(unified.non_interactive);
        assert!(unified.stream_json);
        assert_eq!(unified.task.as_deref(), Some("summarize"));
        assert_eq!(unified.output_mode, OutputModeArg::Silent);
    }

    #[test]
    fn empty_non_interactive_route_still_errors_before_execution() {
        let cli = Cli {
            task: None,
            print_mode: true,
            continue_session: false,
            resume_session: None,
            max_steps: None,
            config_file: DEFAULT_CONFIG_FILE.to_string(),
            working_dir: None,
            verbose: false,
            stream_json: false,
            output_mode: OutputModeArg::Streaming,
            command: None,
        };

        let err = match main_route_decision(&cli, true) {
            Ok(_) => panic!("expected missing task to error"),
            Err(err) => err,
        };
        assert!(err.to_string().contains("No task provided"));
    }
}
