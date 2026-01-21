//! Doctor command implementation

use super::checks::{
    check_api_config, check_config_file, check_environment_variables, check_git_repository,
    check_required_tools, check_working_directory,
};
use super::types::CheckStatus;
use crate::console::CliConsole;
use colored::*;
use sage_core::config::load_config_from_file;
use sage_core::error::SageResult;

/// Run system health checks (doctor command)
pub async fn doctor(config_file: &str) -> SageResult<()> {
    let console = CliConsole::new(true);

    println!();
    println!("{}", "Sage Agent Health Check".bold().underline());
    println!("{}", "=".repeat(50).dimmed());
    println!();

    let mut checks = Vec::new();

    // 1. Check configuration file
    checks.push(check_config_file(config_file));

    // 2. Check environment variables
    checks.extend(check_environment_variables());

    // 3. Check required tools
    checks.extend(check_required_tools().await);

    // 4. Check API connectivity (if config loaded)
    if let Ok(config) = load_config_from_file(config_file) {
        checks.extend(check_api_config(&config));
    }

    // 5. Check working directory
    checks.push(check_working_directory());

    // 6. Check Git repository
    checks.push(check_git_repository());

    // Print results
    let mut pass_count = 0;
    let mut warn_count = 0;
    let mut fail_count = 0;

    for check in &checks {
        println!("{} {} - {}", check.icon(), check.name.bold(), check.message);

        if let Some(hint) = &check.hint {
            println!("    {} {}", "→".dimmed(), hint.dimmed());
        }

        match check.status {
            CheckStatus::Pass => pass_count += 1,
            CheckStatus::Warn => warn_count += 1,
            CheckStatus::Fail => fail_count += 1,
        }
    }

    // Summary
    println!();
    println!("{}", "-".repeat(50).dimmed());
    println!(
        "Summary: {} passed, {} warnings, {} failed",
        pass_count.to_string().green(),
        warn_count.to_string().yellow(),
        fail_count.to_string().red()
    );

    if fail_count > 0 {
        println!();
        console.error("Some checks failed. Please fix the issues above.");
    } else if warn_count > 0 {
        println!();
        console.warn("Some checks have warnings. Consider addressing them.");
    } else {
        println!();
        console.success("All checks passed! Sage Agent is ready to use.");
    }

    Ok(())
}
