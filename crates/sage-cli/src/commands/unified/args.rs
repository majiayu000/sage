//! Arguments for the unified command

use clap::ValueEnum;
use std::path::PathBuf;

/// Output mode for display
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum OutputModeArg {
    /// Real-time streaming output (default)
    #[default]
    Streaming,
    /// Batch output (collect then display)
    Batch,
    /// Silent (no output)
    Silent,
}

/// Arguments for the unified command
pub struct UnifiedArgs {
    /// The task to execute (None for interactive mode with prompt)
    pub task: Option<String>,
    /// Path to configuration file
    pub config_file: String,
    /// Working directory for the agent
    pub working_dir: Option<PathBuf>,
    /// Maximum number of execution steps
    pub max_steps: Option<u32>,
    /// Enable verbose output
    pub verbose: bool,
    /// Non-interactive mode (auto-respond to questions)
    pub non_interactive: bool,
    /// Resume a specific session by ID (for -r flag)
    pub resume_session_id: Option<String>,
    /// Resume the most recent session (for -c flag)
    pub continue_recent: bool,
    /// Stream JSON output mode (for SDK/programmatic use)
    pub stream_json: bool,
    /// Output mode (streaming, batch, silent)
    pub output_mode: OutputModeArg,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unified_args_defaults() {
        let args = UnifiedArgs {
            task: None,
            config_file: "sage_config.json".to_string(),
            working_dir: None,
            max_steps: None,
            verbose: false,
            non_interactive: false,
            resume_session_id: None,
            continue_recent: false,
            stream_json: false,
            output_mode: OutputModeArg::default(),
        };

        assert!(!args.non_interactive);
        assert!(!args.verbose);
        assert!(!args.continue_recent);
        assert!(!args.stream_json);
        assert_eq!(args.output_mode, OutputModeArg::Streaming);
    }

    #[test]
    fn test_output_mode_value_enum() {
        use clap::ValueEnum;
        assert_eq!(OutputModeArg::value_variants().len(), 3);
        let streaming = OutputModeArg::Streaming.to_possible_value();
        assert!(streaming.is_some());
        if let Some(streaming) = streaming {
            assert_eq!(streaming.get_name(), "streaming");
        }
        let batch = OutputModeArg::Batch.to_possible_value();
        assert!(batch.is_some());
        if let Some(batch) = batch {
            assert_eq!(batch.get_name(), "batch");
        }
        let silent = OutputModeArg::Silent.to_possible_value();
        assert!(silent.is_some());
        if let Some(silent) = silent {
            assert_eq!(silent.get_name(), "silent");
        }
    }
}
