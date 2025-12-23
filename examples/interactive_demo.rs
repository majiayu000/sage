//! Interactive Demo for Claude Code Style Agent
//!
//! This demonstrates the core interactive functionality without complex UI

use sage_core::{Config, ReactiveExecutionManager};
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🚀 Sage Agent - Interactive Mode Demo");
    println!("Type 'help' for commands, 'exit' to quit\n");

    // Load configuration
    let config = load_config().await?;

    // Create execution manager
    let mut manager = ReactiveExecutionManager::new(config)?;

    // Interactive loop
    loop {
        print!("sage> ");
        io::stdout().flush()?;

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                let command = input.trim();

                if command.is_empty() {
                    continue;
                }

                match command {
                    "exit" | "quit" => {
                        println!("👋 Goodbye!");
                        break;
                    }
                    "help" => {
                        show_help();
                    }
                    "test" => {
                        test_basic_functionality(&mut manager).await?;
                    }
                    _ => {
                        process_user_input(&mut manager, command).await?;
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ Error reading input: {}", e);
                break;
            }
        }

        println!(); // Add spacing
    }

    Ok(())
}

async fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    println!("📄 Loading configuration...");

    // Try to load from config file, fall back to defaults
    match std::fs::read_to_string("sage_config.json") {
        Ok(content) => match serde_json::from_str::<Config>(&content) {
            Ok(config) => {
                println!("✅ Loaded configuration from sage_config.json");
                Ok(config)
            }
            Err(e) => {
                println!("⚠️  Config file invalid: {}", e);
                println!("📝 Using default configuration");
                Ok(Config::default())
            }
        },
        Err(_) => {
            println!("⚠️  sage_config.json not found");
            println!("📝 Using default configuration");
            Ok(Config::default())
        }
    }
}

async fn process_user_input(
    manager: &mut ReactiveExecutionManager,
    input: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🤖 Processing: {}", input);

    let start_time = std::time::Instant::now();

    match manager.interactive_mode(input).await {
        Ok(response) => {
            let duration = start_time.elapsed();

            // Display response
            if !response.content.trim().is_empty() {
                println!("💬 Response:");
                println!("{}", response.content);
            }

            // Show tool execution summary
            if !response.tool_calls.is_empty() {
                println!("🔧 Executed {} tools:", response.tool_calls.len());
                for (i, call) in response.tool_calls.iter().enumerate() {
                    let result = response.tool_results.get(i);
                    let status = match result {
                        Some(r) if r.success => "✅",
                        Some(_) => "❌",
                        None => "⏳",
                    };
                    println!("  {} {}", status, call.name);
                }
            }

            // Show timing
            println!("⏱️  Duration: {:.2}s", duration.as_secs_f32());

            // Show completion status
            if response.completed {
                println!("✅ Task completed");
            } else if response.continuation_prompt.is_some() {
                println!("🔄 Ready for continuation");
            }
        }
        Err(e) => {
            println!("❌ Error: {}", e);
        }
    }

    Ok(())
}

async fn test_basic_functionality(
    manager: &mut ReactiveExecutionManager,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Running basic functionality test...");

    let test_requests = vec![
        "Hello, introduce yourself",
        "What is the current working directory?",
        "List files in the current directory",
    ];

    for (i, request) in test_requests.iter().enumerate() {
        println!("\n--- Test {} ---", i + 1);
        println!("📝 Request: {}", request);

        match manager.interactive_mode(request).await {
            Ok(response) => {
                println!("✅ Success");
                println!("📄 Response length: {} chars", response.content.len());
                println!("🔧 Tools used: {}", response.tool_calls.len());
                println!("⏱️  Duration: {:?}", response.duration);
            }
            Err(e) => {
                println!("❌ Failed: {}", e);
            }
        }
    }

    println!("\n🎯 Test completed!");
    Ok(())
}

fn show_help() {
    println!("📚 Available commands:");
    println!("  help     - Show this help message");
    println!("  test     - Run basic functionality test");
    println!("  exit     - Exit the program");
    println!("  quit     - Exit the program");
    println!("  <text>   - Process as agent request");
    println!();
    println!("💡 Examples:");
    println!("  sage> Hello, who are you?");
    println!("  sage> List files in current directory");
    println!("  sage> Create a simple Python script");
}
