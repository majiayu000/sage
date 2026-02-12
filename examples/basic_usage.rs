//! Basic usage example for Sage Agent SDK

use sage_sdk::{RunOptions, SageAgentSdk};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🤖 Sage Agent SDK Example");
    println!("========================");

    // Create SDK instance with default configuration
    let sdk = SageAgentSdk::new()?
        .with_provider_and_model("openai", "gpt-4", None)? // API key from environment
        .with_working_directory("./examples")
        .with_max_steps(Some(10));

    // Simple task execution
    println!("\n📝 Running a simple task...");
    let result = sdk
        .run("Create a simple hello world Python script named hello.py")
        .await?;

    if result.is_success() {
        println!("✅ Task completed successfully!");

        // Print execution statistics
        let stats = result.statistics();
        println!("📊 Execution Stats:");
        println!("   Steps: {}", stats.total_steps);
        println!("   Tokens: {}", result.execution().total_usage.total_tokens());

        if let Some(duration) = result.execution().duration() {
            println!(
                "   Duration: {:.2}s",
                duration.num_milliseconds() as f64 / 1000.0
            );
        }

        // Print final result
        if let Some(final_result) = result.final_result() {
            println!("\n📋 Final Result:");
            println!("{}", final_result);
        }

        // Print tool usage
        if !stats.tool_usage.is_empty() {
            println!("\n🔧 Tools Used:");
            for (tool, count) in &stats.tool_usage {
                println!("   {}: {} times", tool, count);
            }
        }
    } else {
        println!("❌ Task failed!");

        // Print error steps
        let error_steps = result.error_steps();
        if !error_steps.is_empty() {
            println!("\n🚨 Error Steps:");
            for step in error_steps {
                if let Some(error) = &step.error {
                    println!("   Step {}: {}", step.step_number, error);
                }
            }
        }
    }

    // Advanced usage with custom options
    println!("\n🔧 Running with custom options...");
    let run_options = RunOptions::new()
        .with_max_steps(5)
        .with_metadata("example_type", "basic_usage");

    let result2 = sdk
        .run_with_options(
            "List all Python files in the current directory and show their sizes",
            run_options,
        )
        .await?;

    if result2.is_success() {
        println!("Second task completed!");
    }

    println!("\n🎉 Example completed!");
    Ok(())
}
