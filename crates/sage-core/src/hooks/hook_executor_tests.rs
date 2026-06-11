use super::super::types::{HookMatcher, HookType};
use super::*;
use std::collections::HashMap;

#[test]
fn test_parse_output_empty() {
    let output = HookExecutor::parse_output("");
    assert!(output.should_continue);
    assert_eq!(output.reason, None);
}

#[test]
fn test_parse_output_plain_text() {
    let output = HookExecutor::parse_output("Hello, world!");
    assert!(output.should_continue);
    assert_eq!(output.reason.as_deref(), Some("Hello, world!"));
}

#[test]
fn test_parse_output_json() {
    let json = r#"{"should_continue": false, "reason": "Blocked"}"#;
    let output = HookExecutor::parse_output(json);
    assert!(!output.should_continue);
    assert_eq!(output.reason.as_deref(), Some("Blocked"));
}

#[test]
fn test_parse_output_json_with_data() {
    let json = r#"{
        "should_continue": true,
        "reason": "Success",
        "additional_context": ["context1"]
    }"#;
    let output = HookExecutor::parse_output(json);
    assert!(output.should_continue);
    assert_eq!(output.reason.as_deref(), Some("Success"));
    assert_eq!(output.additional_context.len(), 1);
}

fn create_test_hook_config(command: &str) -> HookConfig {
    HookConfig {
        name: "test_hook".to_string(),
        hook_type: HookType::PreToolExecution,
        implementation: HookImplementation::Command(CommandHook {
            command: command.to_string(),
            timeout_secs: 60,
            status_message: None,
            working_dir: None,
            env: HashMap::new(),
        }),
        can_block: false,
        timeout_secs: 60,
        enabled: true,
    }
}

fn create_test_hook_config_with_blocking(command: &str, can_block: bool) -> HookConfig {
    HookConfig {
        can_block,
        ..create_test_hook_config(command)
    }
}

#[tokio::test]
async fn test_execute_command_success() {
    let registry = HookRegistry::new();
    let hook_config = create_test_hook_config("echo test");

    let executor = HookExecutor::new(registry);
    let input = HookInput::new(HookEvent::PreToolUse, "test-session");
    let cancel = CancellationToken::new();

    let cmd = match &hook_config.implementation {
        HookImplementation::Command(cmd) => cmd,
        _ => panic!("Expected command hook"),
    };

    let result = executor.execute_command(cmd, &input, &cancel).await;

    match result {
        HookExecutionResult::Success(output) => {
            assert!(output.should_continue);
            assert_eq!(output.reason.as_deref(), Some("test"));
        }
        _ => panic!("Expected success result, got: {:?}", result),
    }
}

#[tokio::test]
async fn test_execute_command_failure() {
    let registry = HookRegistry::new();
    let hook_config = create_test_hook_config("exit 1");

    let executor = HookExecutor::new(registry);
    let input = HookInput::new(HookEvent::PreToolUse, "test-session");
    let cancel = CancellationToken::new();

    let cmd = match &hook_config.implementation {
        HookImplementation::Command(cmd) => cmd,
        _ => panic!("Expected command hook"),
    };

    let result = executor.execute_command(cmd, &input, &cancel).await;

    match result {
        HookExecutionResult::Error(_) => {}
        _ => panic!("Expected error result, got: {:?}", result),
    }
}

#[tokio::test]
async fn test_non_blocking_hook_cannot_stop_execution() -> SageResult<()> {
    let registry = HookRegistry::new();
    registry.register(
        HookEvent::PreToolUse,
        HookMatcher::new(
            Some("bash".to_string()),
            create_test_hook_config_with_blocking(
                "printf '%s' '{\"should_continue\":false,\"reason\":\"not blocking\"}'",
                false,
            ),
        ),
    )?;
    registry.register(
        HookEvent::PreToolUse,
        HookMatcher::new(
            Some("bash".to_string()),
            create_test_hook_config("printf '%s' 'after'"),
        ),
    )?;

    let executor = HookExecutor::new(registry);
    let input = HookInput::new(HookEvent::PreToolUse, "test-session");
    let results = executor
        .execute(
            HookEvent::PreToolUse,
            "bash",
            input,
            CancellationToken::new(),
        )
        .await?;

    assert_eq!(results.len(), 2);
    assert!(results.iter().all(HookExecutionResult::should_continue));
    assert_eq!(results[0].message(), Some("not blocking"));
    assert_eq!(results[1].message(), Some("after"));
    Ok(())
}

#[tokio::test]
async fn test_blocking_hook_stops_execution() -> SageResult<()> {
    let registry = HookRegistry::new();
    registry.register(
        HookEvent::PreToolUse,
        HookMatcher::new(
            Some("bash".to_string()),
            create_test_hook_config_with_blocking(
                "printf '%s' '{\"should_continue\":false,\"reason\":\"blocked\"}'",
                true,
            ),
        ),
    )?;
    registry.register(
        HookEvent::PreToolUse,
        HookMatcher::new(
            Some("bash".to_string()),
            create_test_hook_config("printf '%s' 'after'"),
        ),
    )?;

    let executor = HookExecutor::new(registry);
    let input = HookInput::new(HookEvent::PreToolUse, "test-session");
    let results = executor
        .execute(
            HookEvent::PreToolUse,
            "bash",
            input,
            CancellationToken::new(),
        )
        .await?;

    assert_eq!(results.len(), 1);
    assert!(!results[0].should_continue());
    assert_eq!(results[0].message(), Some("blocked"));
    Ok(())
}

#[test]
fn test_hook_result_should_continue() {
    let success = HookExecutionResult::Success(HookOutput::allow());
    assert!(success.should_continue());

    let blocked = HookExecutionResult::Success(HookOutput::deny("Not allowed"));
    assert!(!blocked.should_continue());

    let error = HookExecutionResult::Error("Failed".to_string());
    assert!(error.should_continue());

    let timeout = HookExecutionResult::Timeout;
    assert!(timeout.should_continue());

    let cancelled = HookExecutionResult::Cancelled;
    assert!(!cancelled.should_continue());
}

#[test]
fn test_hook_result_message() {
    let success = HookExecutionResult::Success(HookOutput::deny("Custom message"));
    assert_eq!(success.message(), Some("Custom message"));

    let error = HookExecutionResult::Error("Failed".to_string());
    assert_eq!(error.message(), Some("Failed"));

    let timeout = HookExecutionResult::Timeout;
    assert!(timeout.message().is_some());
}
