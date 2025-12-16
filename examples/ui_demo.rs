//! UI Enhancement Demo - Showcase all the beautiful new features

use sage_core::ui::{DisplayManager, EnhancedConsole};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎨 Sage Agent UI Enhancement Demo");
    println!("{}", "=".repeat(60));
    println!();

    // 1. Welcome Banner
    println!("1️⃣ Welcome Banner:");
    EnhancedConsole::print_welcome_banner();

    // 2. Section Headers
    println!("2️⃣ Section Headers:");
    EnhancedConsole::print_section_header(
        "Task Execution",
        Some("AI-powered software engineering"),
    );

    // 3. Task Status Examples
    println!("3️⃣ Task Status Examples:");
    EnhancedConsole::print_task_status("Analyzing codebase", "starting", None);
    tokio::time::sleep(Duration::from_millis(500)).await;

    EnhancedConsole::print_task_status("Processing files", "thinking", Some((3, 10)));
    tokio::time::sleep(Duration::from_millis(500)).await;

    EnhancedConsole::print_task_status("Running tests", "executing", Some((7, 10)));
    tokio::time::sleep(Duration::from_millis(500)).await;

    EnhancedConsole::print_task_status("Code generation", "completed", Some((10, 10)));
    tokio::time::sleep(Duration::from_millis(500)).await;

    EnhancedConsole::print_task_status("Deployment", "failed", None);
    println!();

    // 4. Code Block Example
    println!("4️⃣ Code Block Example:");
    let rust_code = r#"fn main() {
    println!("Hello, Sage Agent!");
    let agent = SageAgent::new()?;
    agent.run().await?;
}"#;
    EnhancedConsole::print_code_block(rust_code, "rust");
    println!();

    // 5. Result Summary
    println!("5️⃣ Result Summary:");
    EnhancedConsole::print_result_summary(true, Duration::from_secs_f64(23.45), 5, 8432);

    // 6. Error Example
    println!("6️⃣ Error Example:");
    EnhancedConsole::print_error(
        "Configuration Error",
        "The API key for OpenAI is missing or invalid. Please check your configuration file.",
        Some("Set the OPENAI_API_KEY environment variable or update your sage_config.json file."),
    );

    // 7. Info Box
    println!("7️⃣ Info Box:");
    EnhancedConsole::print_info_box(
        "Available Features",
        &[
            "Multi-model LLM support",
            "Interactive terminal mode",
            "Beautiful markdown rendering",
            "Comprehensive tool integration",
            "Real-time progress tracking",
        ],
    );
    println!();

    // 8. Display Manager Examples
    println!("8️⃣ Display Manager Examples:");
    let display = DisplayManager::new();

    display.print_separator_styled("Modern Separator", "primary");
    display.print_status("🚀", "Enhanced UI is ready!", "success");
    display.print_progress(8, 10, "Loading components");
    println!();

    // 9. Gradient Header
    println!("9️⃣ Gradient Header:");
    display.print_gradient_header("🎯 Mission Complete");

    // 10. Separator
    println!("🔟 Beautiful Separator:");
    EnhancedConsole::print_separator();

    println!("✨ Demo completed! Sage Agent now has a beautiful terminal interface! ✨");
    println!();

    Ok(())
}
