//! Simple Reactive Agent Demo
//!
//! Demonstrates basic reactive agent functionality without UI complexity

use sage_core::{ClaudeStyleAgent, Config, ReactiveAgent};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🚀 Simple Reactive Agent Demo\n");

    // Create a basic config
    let config = create_basic_config();

    // Create reactive agent
    match ClaudeStyleAgent::new(config) {
        Ok(mut agent) => {
            println!("✅ Reactive agent created successfully");

            // Test basic interaction
            let request = "Hello, please tell me about yourself";
            println!("📝 Request: {}", request);

            match agent.process_request(request, None).await {
                Ok(response) => {
                    println!("🤖 Response: {}", response.content);
                    println!("⏱ Duration: {:?}", response.duration);
                    println!("🔧 Tool calls: {}", response.tool_calls.len());
                    println!("✅ Completed: {}", response.completed);
                }
                Err(e) => {
                    println!("❌ Error: {}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to create agent: {}", e);
        }
    }

    println!("\n🎯 Demo completed!");
    Ok(())
}

fn create_basic_config() -> Config {
    // Create a minimal config for testing
    // In real use, this would be loaded from sage_config.json
    Config::default()
}
