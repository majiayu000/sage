//! Usage command implementation

use super::types::format_number;
use super::usage::extract_usage_from_content;
use crate::console::CliConsole;
use colored::*;
use sage_core::error::SageResult;
use std::path::Path;

/// Show token usage statistics
pub async fn usage_cmd(session_dir: Option<&Path>, detailed: bool) -> SageResult<()> {
    let console = CliConsole::new(true);

    println!();
    println!("{}", "Token Usage Statistics".bold().underline());
    println!("{}", "=".repeat(50).dimmed());
    println!();

    // Determine session directory
    let dir = session_dir
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| Path::new("trajectories").to_path_buf());

    if !dir.exists() {
        console.warn(&format!("Session directory not found: {}", dir.display()));
        console.info("Run some tasks first to generate usage data.");
        return Ok(());
    }

    // Collect trajectory files
    let entries: Vec<_> = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|ext| ext == "json" || ext == "jsonl")
        })
        .collect();

    if entries.is_empty() {
        console.warn("No session files found.");
        console.info("Run some tasks first to generate usage data.");
        return Ok(());
    }

    let mut total_prompt_tokens: u64 = 0;
    let mut total_completion_tokens: u64 = 0;
    let mut total_cache_read_tokens: u64 = 0;
    let mut total_cache_created_tokens: u64 = 0;
    let mut session_count = 0;

    // Process each trajectory file
    for entry in &entries {
        if let Ok(content) = std::fs::read_to_string(entry.path()) {
            // Try to extract usage data from the file
            if let Some(usage) = extract_usage_from_content(&content) {
                total_prompt_tokens += usage.prompt_tokens;
                total_completion_tokens += usage.completion_tokens;
                total_cache_read_tokens += usage.cache_read_tokens;
                total_cache_created_tokens += usage.cache_created_tokens;
                session_count += 1;

                if detailed {
                    println!(
                        "  {} - {} prompt, {} completion",
                        entry
                            .path()
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .cyan(),
                        format_number(usage.prompt_tokens),
                        format_number(usage.completion_tokens)
                    );
                }
            }
        }
    }

    if detailed && session_count > 0 {
        println!();
    }

    // Print summary
    println!("{}", "Summary".cyan().bold());
    println!("  Sessions Analyzed: {}", session_count.to_string().cyan());
    println!(
        "  Total Prompt Tokens: {}",
        format_number(total_prompt_tokens).green()
    );
    println!(
        "  Total Completion Tokens: {}",
        format_number(total_completion_tokens).green()
    );
    println!(
        "  Total Tokens: {}",
        format_number(total_prompt_tokens + total_completion_tokens)
            .yellow()
            .bold()
    );

    if total_cache_read_tokens > 0 || total_cache_created_tokens > 0 {
        println!();
        println!("{}", "Cache Statistics".cyan().bold());
        println!(
            "  Cache Read Tokens: {}",
            format_number(total_cache_read_tokens).green()
        );
        println!(
            "  Cache Created Tokens: {}",
            format_number(total_cache_created_tokens).cyan()
        );

        // Calculate savings percentage
        if total_prompt_tokens > 0 {
            let savings_pct = (total_cache_read_tokens as f64 / total_prompt_tokens as f64) * 100.0;
            println!("  Estimated Savings: {:.1}%", savings_pct);
        }
    }

    Ok(())
}
