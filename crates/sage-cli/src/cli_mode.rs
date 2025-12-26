//! CLI mode definitions and utilities

/// CLI Mode enum for documentation and type safety
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliMode {
    /// Interactive conversation mode with multi-turn context
    Interactive,
    /// One-shot task execution mode
    Run,
    /// Unified execution mode (Claude Code style)
    Unified,
    /// Configuration management
    Config,
    /// Trajectory analysis and management
    Trajectory,
    /// Tool inspection
    Tools,
}

impl CliMode {
    /// Get a human-readable description of the mode
    pub fn description(&self) -> &'static str {
        match self {
            CliMode::Interactive => "Multi-turn conversation mode with context retention",
            CliMode::Run => "Single task execution mode (fire-and-forget)",
            CliMode::Unified => "Advanced execution mode with inline input blocking",
            CliMode::Config => "Configuration file management",
            CliMode::Trajectory => "Execution trajectory analysis",
            CliMode::Tools => "Tool discovery and inspection",
        }
    }

    /// Get usage examples for the mode
    pub fn examples(&self) -> Vec<&'static str> {
        match self {
            CliMode::Interactive => vec![
                "sage interactive",
                "sage interactive --modern-ui",
                "sage interactive --claude-style",
            ],
            CliMode::Run => vec![
                "sage run \"Create a Python hello world\"",
                "sage run \"Fix the bug in main.rs\" --provider anthropic",
                "sage run --task-file task.txt",
            ],
            CliMode::Unified => vec![
                "sage unified \"Create a test suite\"",
                "sage unified --non-interactive \"Run tests\"",
                "sage unified --max-steps 10 \"Refactor code\"",
            ],
            CliMode::Config => vec![
                "sage config show",
                "sage config validate",
                "sage config init",
            ],
            CliMode::Trajectory => vec![
                "sage trajectory list",
                "sage trajectory show <file>",
                "sage trajectory analyze <file>",
            ],
            CliMode::Tools => vec!["sage tools"],
        }
    }
}
