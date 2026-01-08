//! Help and information display functions

use crate::console::CliConsole;
use sage_core::config::format_api_key_status_for_provider;
use sage_sdk::SageAgentSdk;
use std::path::PathBuf;

/// Print help information
pub fn print_help(console: &CliConsole) {
    console.print_header("Available Commands");
    console.info("help, h          - Show this help message");
    console.info("config           - Show current configuration");
    console.info("status           - Show system status");
    console.info("clear, cls       - Clear the screen");
    console.info("reset, refresh   - Reset terminal display (fixes backspace issues)");
    console.info("input-help, ih   - Show input troubleshooting help");
    console.info("new, new-task    - Start a new conversation (clears previous context)");
    console.info("conversation, conv - Show current conversation summary");
    console.info("exit, quit, q    - Exit interactive mode");
    console.info("");
    console.info("🗣️  Conversation Mode:");
    console.info("Any other input will be treated as part of an ongoing conversation.");
    console
        .info("The AI will remember previous messages and context within the same conversation.");
    console.info("Use 'new' to start fresh if you want to change topics completely.");
    console.info("");
    console.info("Example conversation:");
    console.info("  You: Create a hello world Python script");
    console.info("  AI: [Creates the script]");
    console.info("  You: Now add error handling to it");
    console.info("  AI: [Modifies the existing script with error handling]");
    console.info("");
    console.info("📜 Slash Commands:");
    console.info("  /help           - Show AI help information");
    console.info("  /commands       - List all available slash commands");
    console.info("  /resume         - Resume a previous session (interactive)");
    console.info("  /resume <id>    - Resume a specific session by ID");
    console.info("  /cost           - Show session cost and usage");
    console.info("  /context        - Show context window usage");
    console.info("  /status         - Show agent status");
    console.info("  /undo           - Undo last file changes");
    console.info("  /checkpoint     - Create a checkpoint");
    console.info("  /plan           - View/manage execution plan");
}

/// Print input troubleshooting help
pub fn print_input_help(console: &CliConsole) {
    console.print_header("退格键问题解决方案");

    console.info("如果遇到退格键删除后仍显示字符的问题：");
    console.info("");
    console.info("立即解决方案：");
    console.info("  reset          - 重置终端显示（推荐）");
    console.info("  clear          - 清屏重新开始");
    console.info("  Ctrl+U         - 清除当前行");
    console.info("");
    console.info("常见问题和解决方法：");
    console.info("  • 中文输入残留:    输入 'reset' 重置显示");
    console.info("  • 退格键异常:      切换到英文输入法");
    console.info("  • 字符显示错乱:    使用 Ctrl+U 清除整行");
    console.info("  • 输入法问题:      重启输入法或切换输入法");
    console.info("");
    console.info("预防措施：");
    console.info("  • 输入命令时使用英文输入法");
    console.info("  • 避免在输入过程中频繁切换输入法");
    console.info("  • 使用支持中文较好的终端（如 iTerm2）");
    console.info("");
    console.info("终端快捷键：");
    console.info("  • Ctrl+U         - 清除当前行");
    console.info("  • Ctrl+A         - 移动到行首");
    console.info("  • Ctrl+E         - 移动到行尾");
    console.info("  • Ctrl+C         - 取消当前输入");
}

/// Print current configuration
pub fn print_config(console: &CliConsole, sdk: &SageAgentSdk) {
    console.print_header("Current Configuration");
    let config = sdk.config();

    console.info(&format!("Provider: {}", config.default_provider));

    if let Ok(params) = config.default_model_parameters() {
        console.info(&format!("Model: {}", params.model));
    }

    let max_steps_display = match config.max_steps {
        Some(n) => n.to_string(),
        None => "unlimited".to_string(),
    };
    console.info(&format!("Max Steps: {}", max_steps_display));

    if let Some(working_dir) = &config.working_directory {
        console.info(&format!("Working Directory: {}", working_dir.display()));
    }

    // All default tools are always enabled
    let tools_count = sage_tools::get_default_tools().len();
    console.info(&format!("Tools Available: {}", tools_count));
}

/// Print system status
pub fn print_status(console: &CliConsole, sdk: &SageAgentSdk) {
    console.print_header("Agent Status");

    let config = sdk.config();

    console.info(&format!("Provider: {}", config.get_default_provider()));

    if let Ok(params) = config.default_model_parameters() {
        console.info(&format!("Model: {}", params.model));
    }

    // All default tools are always available
    let tools_count = sage_tools::get_default_tools().len();
    console.info(&format!("Available Tools: {}", tools_count));
    let max_steps_display = match config.max_steps {
        Some(n) => n.to_string(),
        None => "unlimited".to_string(),
    };
    console.info(&format!("Max Steps: {}", max_steps_display));

    match sdk.validate_config() {
        Ok(()) => console.success("Configuration is valid"),
        Err(e) => console.error(&format!("Configuration error: {e}")),
    }

    // Show API key status for each provider
    console.info("");
    console.info("API Key Status:");
    for (provider, params) in &config.model_providers {
        let key_info = params.get_api_key_info_for_provider(provider);
        let status = format_api_key_status_for_provider(provider, &key_info);
        console.info(&format!("  {}", status));

        // Also validate the key format if present
        if key_info.is_valid() {
            if let Err(e) = params.validate_api_key_format_for_provider(provider) {
                console.warn(&format!("    ⚠ {}", e));
            }
        }
    }

    if let Some(working_dir) = &config.working_directory {
        if working_dir.exists() {
            console.success(&format!(
                "Working directory accessible: {}",
                working_dir.display()
            ));
        } else {
            console.error(&format!(
                "Working directory not found: {}",
                working_dir.display()
            ));
        }
    } else {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        console.info(&format!(
            "Using current directory: {}",
            current_dir.display()
        ));
    }
}
