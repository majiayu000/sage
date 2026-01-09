//! Built-in slash commands registration

use super::super::types::{CommandSource, SlashCommand};
use super::types::CommandRegistry;

impl CommandRegistry {
    /// Register built-in commands
    pub fn register_builtins(&mut self) {
        // Help command
        self.register(
            SlashCommand::new(
                "help",
                "Show available commands and how to use them. List all slash commands with /commands.",
            )
            .with_description("Show help information")
            .builtin(),
            CommandSource::Builtin,
        );

        // Clear command
        self.register(
            SlashCommand::new("clear", "Clear the conversation history and start fresh.")
                .with_description("Clear conversation")
                .builtin(),
            CommandSource::Builtin,
        );

        // Compact command
        self.register(
            SlashCommand::new(
                "compact",
                "Summarize the conversation and compact the context to save tokens.",
            )
            .with_description("Compact conversation context")
            .builtin(),
            CommandSource::Builtin,
        );

        // Init command
        self.register(
            SlashCommand::new(
                "init",
                "Initialize a .sage directory in the current project with default settings.",
            )
            .with_description("Initialize Sage in project")
            .builtin(),
            CommandSource::Builtin,
        );

        // Config command
        self.register(
            SlashCommand::new("config", "Show or modify Sage configuration settings.")
                .with_description("Manage configuration")
                .builtin(),
            CommandSource::Builtin,
        );

        // Checkpoint command
        self.register(
            SlashCommand::new(
                "checkpoint",
                "Create a checkpoint of the current state. Usage: /checkpoint [name]",
            )
            .with_description("Create state checkpoint")
            .builtin(),
            CommandSource::Builtin,
        );

        // Restore command
        self.register(
            SlashCommand::new(
                "restore",
                "Restore to a previous checkpoint. Usage: /restore [checkpoint-id]",
            )
            .with_description("Restore from checkpoint")
            .builtin(),
            CommandSource::Builtin,
        );

        // Tasks command
        self.register(
            SlashCommand::new("tasks", "Show all running and completed background tasks.")
                .with_description("List background tasks")
                .builtin(),
            CommandSource::Builtin,
        );

        // Commands command
        self.register(
            SlashCommand::new("commands", "List all available slash commands.")
                .with_description("List slash commands")
                .builtin(),
            CommandSource::Builtin,
        );

        // Undo command
        self.register(
            SlashCommand::new(
                "undo",
                "Undo the last file changes by restoring files to their previous state. Usage: /undo [message-id]",
            )
            .with_description("Undo file changes")
            .builtin(),
            CommandSource::Builtin,
        );

        // Cost command - show session cost and token usage
        self.register(
            SlashCommand::new(
                "cost",
                "Show the total cost and token usage for the current session.",
            )
            .with_description("Show session cost and usage")
            .builtin(),
            CommandSource::Builtin,
        );

        // Context command - show context/token breakdown
        self.register(
            SlashCommand::new(
                "context",
                "Show the current context window usage and token breakdown.",
            )
            .with_description("Show context usage")
            .builtin(),
            CommandSource::Builtin,
        );

        // Status command - show agent status
        self.register(
            SlashCommand::new(
                "status",
                "Show Sage status including version, provider, model, and connection status.",
            )
            .with_description("Show agent status")
            .builtin(),
            CommandSource::Builtin,
        );

        // Resume command - resume previous session
        self.register(
            SlashCommand::new(
                "resume",
                "Resume a previous conversation session. Usage: /resume [session-id]",
            )
            .with_description("Resume a conversation")
            .builtin(),
            CommandSource::Builtin,
        );

        // Plan command - view/manage execution plan
        self.register(
            SlashCommand::new(
                "plan",
                "View or manage the current execution plan. Usage: /plan [open|clear]",
            )
            .with_description("View execution plan")
            .builtin(),
            CommandSource::Builtin,
        );

        // Title command - set custom session title (Claude Code style)
        self.register(
            SlashCommand::new(
                "title",
                "Set a custom title for the current session. Usage: /title <title>",
            )
            .with_description("Set session title")
            .builtin(),
            CommandSource::Builtin,
        );

        // Login command - configure API credentials
        self.register(
            SlashCommand::new(
                "login",
                "Configure API credentials for AI providers. Opens the credential setup wizard.",
            )
            .with_description("Configure API credentials")
            .builtin(),
            CommandSource::Builtin,
        );
    }
}
