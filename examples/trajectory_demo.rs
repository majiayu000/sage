//! Trajectory configuration demo

use std::error::Error;
use sage_sdk::{SageAgentSDK, RunOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🤖 Sage Agent Trajectory Demo");
    println!("=============================");

    // Create SDK instance with trajectory enabled
    let sdk = SageAgentSDK::new()?
        .with_provider_and_model("openai", "gpt-4", None)?
        .with_working_directory("./examples");

    // Run with trajectory recording enabled
    println!("\n📝 Running task with trajectory recording...");
    let run_options = RunOptions::new()
        .with_max_steps(3)
        .with_trajectory(true)  // Enable trajectory recording
        .with_metadata("demo_type", "trajectory_test");

    let result = sdk.run_with_options(
        "List the files in the current directory",
        run_options,
    ).await?;

    if result.is_success() {
        println!("✅ Task completed successfully!");
        
        // Show trajectory path
        if let Some(trajectory_path) = result.trajectory_path() {
            println!("📊 Trajectory saved to: {}", trajectory_path.display());
            println!("   You can find this file in the trajectories/ directory");
        }
        
        // Print execution statistics
        let stats = result.statistics();
        println!("📈 Execution Stats:");
        println!("   Steps: {}", stats.total_steps);
        println!("   Tokens: {}", stats.total_tokens);
        if let Some(duration) = stats.execution_time {
            println!("   Duration: {:.2}s", duration.num_milliseconds() as f64 / 1000.0);
        }
    } else {
        println!("❌ Task failed!");
    }

    println!("\n💡 Tip: Check the trajectories/ directory for the generated trajectory file!");
    println!("   It contains the complete execution history including LLM interactions.");

    Ok(())
}
