//! Tests for sandbox executor

#[cfg(test)]
mod tests {
    use crate::sandbox::executor::{ExecutionBuilder, SandboxExecutor};
    use crate::sandbox::limits::ResourceLimits;
    use std::time::Duration;

    #[tokio::test]
    async fn test_simple_execution() {
        let result = SandboxExecutor::execute(
            "echo",
            &["hello".to_string()],
            None,
            None,
            &ResourceLimits::default(),
            Duration::from_secs(10),
        )
        .await
        .unwrap();

        assert!(result.success());
        assert!(result.stdout.contains("hello"));
        assert!(!result.timed_out);
    }

    #[tokio::test]
    async fn test_exit_code() {
        let result = SandboxExecutor::execute(
            "false",
            &[],
            None,
            None,
            &ResourceLimits::default(),
            Duration::from_secs(10),
        )
        .await
        .unwrap();

        assert!(!result.success());
        assert_eq!(result.exit_code, Some(1));
    }

    #[tokio::test]
    async fn test_timeout() {
        let result = SandboxExecutor::execute(
            "sleep",
            &["10".to_string()],
            None,
            None,
            &ResourceLimits::default(),
            Duration::from_millis(100),
        )
        .await
        .unwrap();

        assert!(result.timed_out);
        assert!(!result.success());
    }

    #[tokio::test]
    async fn test_shell_execution() {
        let result = SandboxExecutor::execute_shell(
            "echo $((1 + 1))",
            None,
            None,
            &ResourceLimits::default(),
            Duration::from_secs(10),
        )
        .await
        .unwrap();

        assert!(result.success());
        assert!(result.stdout.contains("2"));
    }

    #[tokio::test]
    async fn test_execution_builder() {
        let result = ExecutionBuilder::new("echo")
            .arg("hello")
            .arg("world")
            .timeout(Duration::from_secs(10))
            .execute()
            .await
            .unwrap();

        assert!(result.success());
        assert!(result.stdout.contains("hello world"));
    }

    #[tokio::test]
    async fn test_working_directory() {
        let result = ExecutionBuilder::new("pwd")
            .working_dir("/tmp")
            .timeout(Duration::from_secs(10))
            .execute()
            .await
            .unwrap();

        assert!(result.success());
        // On macOS /tmp is symlinked to /private/tmp
        assert!(result.stdout.contains("tmp"));
    }

    #[tokio::test]
    async fn test_environment_variables() {
        let result = ExecutionBuilder::new("sh")
            .arg("-c")
            .arg("echo $TEST_VAR")
            .env("TEST_VAR", "test_value")
            .timeout(Duration::from_secs(10))
            .execute()
            .await
            .unwrap();

        assert!(result.success());
        assert!(result.stdout.contains("test_value"));
    }

    #[tokio::test]
    async fn test_stderr_capture() {
        let result = ExecutionBuilder::new("sh")
            .arg("-c")
            .arg("echo error >&2")
            .timeout(Duration::from_secs(10))
            .execute()
            .await
            .unwrap();

        assert!(result.success());
        assert!(result.stderr.contains("error"));
    }

    #[tokio::test]
    async fn test_combined_output() {
        let result = ExecutionBuilder::new("sh")
            .arg("-c")
            .arg("echo stdout; echo stderr >&2")
            .timeout(Duration::from_secs(10))
            .execute()
            .await
            .unwrap();

        let combined = result.combined_output();
        assert!(combined.contains("stdout"));
        assert!(combined.contains("stderr"));
    }
}
