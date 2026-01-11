//! Tests for command types

use super::*;

#[test]
fn test_command_creation() {
    let cmd = SlashCommand::new("test", "Run tests").with_description("Run all tests");

    assert_eq!(cmd.name, "test");
    assert_eq!(cmd.description, Some("Run all tests".to_string()));
}

#[test]
fn test_command_expand_arguments() {
    let cmd = SlashCommand::new("greet", "Hello $ARGUMENTS!");
    let expanded = cmd.expand(&["World".to_string()]);
    assert_eq!(expanded, "Hello World!");
}

#[test]
fn test_command_expand_numbered_args() {
    let cmd = SlashCommand::new("swap", "Swap $ARG1 with $ARG2");
    let expanded = cmd.expand(&["foo".to_string(), "bar".to_string()]);
    assert_eq!(expanded, "Swap foo with bar");
}

#[test]
fn test_command_expand_json_args() {
    let cmd = SlashCommand::new("list", "Items: $ARGUMENTS_JSON");
    let expanded = cmd.expand(&["a".to_string(), "b".to_string()]);
    assert!(expanded.contains("\"a\""));
    assert!(expanded.contains("\"b\""));
}

#[test]
fn test_command_requires_arguments() {
    let cmd1 = SlashCommand::new("simple", "Just text");
    assert!(!cmd1.requires_arguments());

    let cmd2 = SlashCommand::new("with_args", "Process $ARGUMENTS");
    assert!(cmd2.requires_arguments());
}

#[test]
fn test_command_min_args() {
    let cmd1 = SlashCommand::new("none", "No args");
    assert_eq!(cmd1.min_args(), 0);

    let cmd2 = SlashCommand::new("two", "$ARG1 and $ARG2");
    assert_eq!(cmd2.min_args(), 2);
}

#[test]
fn test_parse_invocation() {
    let inv = CommandInvocation::parse("/test arg1 arg2").unwrap();
    assert_eq!(inv.command_name, "test");
    assert_eq!(inv.arguments, vec!["arg1", "arg2"]);
}

#[test]
fn test_parse_invocation_no_args() {
    let inv = CommandInvocation::parse("/help").unwrap();
    assert_eq!(inv.command_name, "help");
    assert!(inv.arguments.is_empty());
}

#[test]
fn test_parse_invocation_quoted_args() {
    let inv = CommandInvocation::parse("/search \"hello world\"").unwrap();
    assert_eq!(inv.command_name, "search");
    assert_eq!(inv.arguments, vec!["hello world"]);
}

#[test]
fn test_parse_invocation_not_slash() {
    assert!(CommandInvocation::parse("test").is_none());
    assert!(CommandInvocation::parse("").is_none());
}

#[test]
fn test_is_slash_command() {
    assert!(CommandInvocation::is_slash_command("/test"));
    assert!(CommandInvocation::is_slash_command("/help arg"));
    assert!(!CommandInvocation::is_slash_command("test"));
    assert!(!CommandInvocation::is_slash_command("/"));
    assert!(!CommandInvocation::is_slash_command("/123"));
}

#[test]
fn test_command_result() {
    let result = CommandResult::prompt("Hello")
        .show()
        .with_context("Context 1")
        .with_status("Running...");

    assert_eq!(result.expanded_prompt, "Hello");
    assert!(result.show_expansion);
    assert_eq!(result.context_messages.len(), 1);
    assert_eq!(result.status_message, Some("Running...".to_string()));
}

#[test]
fn test_command_argument() {
    let arg = CommandArgument::required("file").with_description("The file to process");

    assert_eq!(arg.name, "file");
    assert!(arg.required);
    assert!(arg.default.is_none());

    let opt = CommandArgument::optional("format").with_default("json");

    assert!(!opt.required);
    assert_eq!(opt.default, Some("json".to_string()));
}

#[test]
fn test_command_with_allowed_tools() {
    let cmd = SlashCommand::new("review", "Review the code")
        .with_allowed_tools(vec!["Read".to_string(), "Grep".to_string()]);

    assert_eq!(
        cmd.allowed_tools,
        Some(vec!["Read".to_string(), "Grep".to_string()])
    );
}

#[test]
fn test_command_with_model() {
    let cmd = SlashCommand::new("fast", "Quick task").with_model("gpt-4o-mini");

    assert_eq!(cmd.model_override, Some("gpt-4o-mini".to_string()));
}

#[test]
fn test_command_result_tool_restrictions() {
    let result =
        CommandResult::prompt("test").with_tool_restrictions(vec!["Read".to_string(), "Write".to_string()]);

    assert!(result.has_tool_restrictions());
    assert_eq!(
        result.tool_restrictions,
        Some(vec!["Read".to_string(), "Write".to_string()])
    );
}

#[test]
fn test_command_result_model_override() {
    let result = CommandResult::prompt("test").with_model("claude-opus-4-5-20251101");

    assert!(result.has_model_override());
    assert_eq!(
        result.model_override,
        Some("claude-opus-4-5-20251101".to_string())
    );
}

#[test]
fn test_command_result_no_restrictions() {
    let result = CommandResult::prompt("test");

    assert!(!result.has_tool_restrictions());
    assert!(!result.has_model_override());
    assert!(result.tool_restrictions.is_none());
    assert!(result.model_override.is_none());
}

#[test]
fn test_command_source_display() {
    assert_eq!(CommandSource::Builtin.to_string(), "builtin");
    assert_eq!(CommandSource::Project.to_string(), "project");
    assert_eq!(CommandSource::User.to_string(), "user");
}

#[test]
fn test_interactive_command_variants() {
    let resume = InteractiveCommand::Resume {
        session_id: Some("abc123".to_string()),
        show_all: true,
    };
    assert!(matches!(resume, InteractiveCommand::Resume { .. }));

    let title = InteractiveCommand::Title {
        title: "My Session".to_string(),
    };
    assert!(matches!(title, InteractiveCommand::Title { .. }));

    let login = InteractiveCommand::Login;
    assert!(matches!(login, InteractiveCommand::Login));
}
