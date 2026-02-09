//! Tests for command executor

use super::types::CommandExecutor;
use crate::commands::registry::CommandRegistry;
use crate::commands::types::SlashCommand;
use std::sync::Arc;
use tokio::sync::RwLock;

async fn create_test_executor() -> CommandExecutor {
    let mut registry = CommandRegistry::new("/project");
    registry.register_builtins();
    CommandExecutor::new(Arc::new(RwLock::new(registry)))
}

#[tokio::test]
async fn test_is_command() {
    assert!(CommandExecutor::is_command("/help"));
    assert!(CommandExecutor::is_command("/test arg"));
    assert!(!CommandExecutor::is_command("help"));
    assert!(!CommandExecutor::is_command(""));
}

#[tokio::test]
async fn test_process_builtin() {
    let executor = create_test_executor().await;

    let result = executor.process("/help").await.unwrap();
    assert!(result.is_some());
}

#[tokio::test]
async fn test_process_unknown_command() {
    let executor = create_test_executor().await;

    let result = executor.process("/nonexistent").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_process_not_command() {
    let executor = create_test_executor().await;

    let result = executor.process("just text").await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_execute_help() {
    let executor = create_test_executor().await;
    let result = executor.process("/help").await.unwrap().unwrap();

    assert!(result.is_local);
    assert!(result.local_output.is_some());
    assert!(!result.local_output.as_ref().unwrap().is_empty());
}

#[tokio::test]
async fn test_execute_clear() {
    let executor = create_test_executor().await;
    let result = executor.process("/clear").await.unwrap().unwrap();

    assert!(result.interactive.is_some());
}

#[tokio::test]
async fn test_execute_checkpoint_with_name() {
    let executor = create_test_executor().await;
    let result = executor
        .process("/checkpoint my-save")
        .await
        .unwrap()
        .unwrap();

    assert!(result.is_local);
    assert!(result.local_output.is_some());
    let output = result.local_output.as_ref().unwrap();
    assert!(
        output.contains("my-save")
            || output.contains("checkpoint")
            || output.contains("No changes")
    );
}

#[tokio::test]
async fn test_execute_commands() {
    let executor = create_test_executor().await;
    let result = executor.process("/commands").await.unwrap().unwrap();

    assert!(result.is_local);
    assert!(result.local_output.is_some());
    let output = result.local_output.as_ref().unwrap();
    assert!(output.contains("help") || output.contains("Available") || output.contains("commands"));
}

#[tokio::test]
async fn test_custom_command_execution() {
    let mut registry = CommandRegistry::new("/project");
    registry.register(
        SlashCommand::new("greet", "Say hello to $ARGUMENTS"),
        crate::commands::types::CommandSource::Project,
    );

    let executor = CommandExecutor::new(Arc::new(RwLock::new(registry)));
    let result = executor.process("/greet World").await.unwrap().unwrap();

    assert_eq!(result.expanded_prompt, "Say hello to World");
}

#[tokio::test]
async fn test_command_min_args_validation() {
    let mut registry = CommandRegistry::new("/project");
    registry.register(
        SlashCommand::new("swap", "Swap $ARG1 with $ARG2"),
        crate::commands::types::CommandSource::Project,
    );

    let executor = CommandExecutor::new(Arc::new(RwLock::new(registry)));

    // Should fail with insufficient args
    let result = executor.process("/swap only-one").await;
    assert!(result.is_err());

    // Should succeed with enough args
    let result = executor.process("/swap a b").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_get_suggestions() {
    let executor = create_test_executor().await;

    let suggestions = executor.get_suggestions("/he").await;
    assert!(suggestions.contains(&"/help".to_string()));

    let suggestions = executor.get_suggestions("/ch").await;
    assert!(suggestions.contains(&"/checkpoint".to_string()));
}

#[tokio::test]
async fn test_reload() {
    let executor = create_test_executor().await;

    // Should preserve builtins
    let _count = executor.reload().await.unwrap();

    let result = executor.process("/help").await.unwrap();
    assert!(result.is_some()); // Builtins still work
}
