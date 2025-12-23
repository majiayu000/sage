//! Tests for the unified executor

#[cfg(test)]
mod tests {
    use crate::agent::{ExecutionMode, ExecutionOptions};

    #[test]
    fn test_unified_executor_builder() {
        // This test would need a valid config, so we just test the builder pattern
        let options = ExecutionOptions::interactive().with_step_limit(50);
        assert_eq!(options.max_steps, Some(50));
        assert!(options.is_interactive());
    }

    #[test]
    fn test_execution_modes() {
        let interactive = ExecutionMode::interactive();
        assert!(interactive.is_interactive());

        let non_interactive = ExecutionMode::non_interactive();
        assert!(non_interactive.is_non_interactive());

        let batch = ExecutionMode::batch();
        assert!(batch.is_batch());
    }

    #[test]
    fn test_builder_with_options() {
        let options = ExecutionOptions::interactive()
            .with_step_limit(100)
            .with_verbose(true);

        assert_eq!(options.max_steps, Some(100));
        assert!(options.verbose);
        assert!(options.is_interactive());
    }

    #[test]
    fn test_builder_batch_mode() {
        let options = ExecutionOptions::batch();
        assert!(options.mode.is_batch());
        assert!(!options.is_interactive());
    }

    #[test]
    fn test_builder_non_interactive_mode() {
        let options = ExecutionOptions::non_interactive("auto answer");
        assert!(options.mode.is_non_interactive());
        assert!(!options.is_interactive());
    }

    #[test]
    fn test_builder_max_steps() {
        let options = ExecutionOptions::interactive().with_max_steps(None);
        assert_eq!(options.max_steps, None);

        let options = ExecutionOptions::interactive().with_step_limit(50);
        assert_eq!(options.max_steps, Some(50));
    }

    #[test]
    fn test_builder_working_directory() {
        let options = ExecutionOptions::interactive().with_working_directory("/tmp");
        assert!(options.working_directory.is_some());
        assert_eq!(
            options
                .working_directory
                .expect("working_directory should be Some in test context")
                .to_str()
                .expect("working_directory path should be valid UTF-8"),
            "/tmp"
        );
    }

    #[test]
    fn test_options_trajectory() {
        let options = ExecutionOptions::interactive().with_trajectory(true);
        assert!(options.should_record_trajectory());

        let options = ExecutionOptions::interactive().with_trajectory(false);
        assert!(!options.should_record_trajectory());
    }

    #[test]
    fn test_options_continue_on_error() {
        let options = ExecutionOptions::interactive().with_continue_on_error(true);
        assert!(options.continue_on_error);

        let options = ExecutionOptions::interactive().with_continue_on_error(false);
        assert!(!options.continue_on_error);
    }

    #[test]
    fn test_builder_fluent_api() {
        let options = ExecutionOptions::default()
            .with_mode(ExecutionMode::batch())
            .with_step_limit(10)
            .with_verbose(true)
            .with_continue_on_error(true);

        assert!(options.mode.is_batch());
        assert_eq!(options.max_steps, Some(10));
        assert!(options.verbose);
        assert!(options.continue_on_error);
    }
}
