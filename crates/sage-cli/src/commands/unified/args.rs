//! Arguments for the unified command

use std::path::PathBuf;

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
        };

        assert!(!args.non_interactive);
        assert!(!args.verbose);
        assert!(!args.continue_recent);
        assert!(!args.stream_json);
    }
}
