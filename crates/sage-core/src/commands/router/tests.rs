//! Tests for command router

use super::*;
use crate::commands::registry::CommandRegistry;

#[test]
fn test_is_command() {
    assert!(CommandRouter::is_command("/help"));
    assert!(CommandRouter::is_command("/test arg1 arg2"));
    assert!(!CommandRouter::is_command("not a command"));
    assert!(!CommandRouter::is_command(""));
    assert!(!CommandRouter::is_command("/"));
    assert!(!CommandRouter::is_command("/123"));
}

#[test]
fn test_parse() {
    let inv = CommandRouter::parse("/help").unwrap();
    assert_eq!(inv.command_name, "help");
    assert!(inv.arguments.is_empty());

    let inv = CommandRouter::parse("/test arg1 arg2").unwrap();
    assert_eq!(inv.command_name, "test");
    assert_eq!(inv.arguments, vec!["arg1", "arg2"]);

    assert!(CommandRouter::parse("not a command").is_none());
}

#[test]
fn test_command_category_display() {
    assert_eq!(CommandCategory::System.to_string(), "system");
    assert_eq!(CommandCategory::User.to_string(), "user");
    assert_eq!(CommandCategory::Mcp.to_string(), "mcp");
}

#[test]
fn test_command_source_to_category() {
    assert_eq!(
        CommandCategory::from(&CommandSource::Builtin),
        CommandCategory::System
    );
    assert_eq!(
        CommandCategory::from(&CommandSource::Project),
        CommandCategory::User
    );
    assert_eq!(
        CommandCategory::from(&CommandSource::User),
        CommandCategory::User
    );
}

#[test]
fn test_command_list() {
    let list = CommandList {
        system: vec![RoutedCommand {
            name: "help".to_string(),
            category: CommandCategory::System,
            description: Some("Show help".to_string()),
        }],
        user: vec![],
        mcp: vec![],
    };

    assert_eq!(list.total(), 1);
    assert!(!list.is_empty());
    assert_eq!(list.all().len(), 1);
}

#[test]
fn test_command_result_kind() {
    // Local result
    let local = CommandResult::local("output text");
    assert!(matches!(
        local.kind(),
        CommandResultKind::Local {
            output: "output text"
        }
    ));

    // Prompt result
    let prompt = CommandResult::prompt("prompt text");
    assert!(matches!(
        prompt.kind(),
        CommandResultKind::Prompt {
            content: "prompt text"
        }
    ));

    // Interactive result
    let interactive = CommandResult::interactive(InteractiveCommand::Login);
    assert!(matches!(
        interactive.kind(),
        CommandResultKind::Interactive(InteractiveCommand::Login)
    ));
}

#[tokio::test]
async fn test_router_creation() {
    let temp_dir = std::env::temp_dir().join("sage_router_test");
    let _ = std::fs::create_dir_all(&temp_dir);

    let router = CommandRouter::new(&temp_dir).await.unwrap();

    let list = router.list_commands().await;
    assert!(!list.system.is_empty());
    assert!(list.system.iter().any(|c| c.name == "help"));
    assert!(list.system.iter().any(|c| c.name == "login"));

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_router_route() {
    let temp_dir = std::env::temp_dir().join("sage_router_route_test");
    let _ = std::fs::create_dir_all(&temp_dir);

    let router = CommandRouter::new(&temp_dir).await.unwrap();

    let result = router.route("/help").await.unwrap();
    assert!(result.is_some());

    let result = router.route("not a command").await.unwrap();
    assert!(result.is_none());

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_router_get_command_info() {
    let temp_dir = std::env::temp_dir().join("sage_router_info_test");
    let _ = std::fs::create_dir_all(&temp_dir);

    let router = CommandRouter::new(&temp_dir).await.unwrap();

    let info = router.get_command_info("help").await;
    assert!(info.is_some());
    let info = info.unwrap();
    assert_eq!(info.name, "help");
    assert_eq!(info.category, CommandCategory::System);

    let info = router.get_command_info("nonexistent").await;
    assert!(info.is_none());

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_router_suggestions() {
    let temp_dir = std::env::temp_dir().join("sage_router_suggest_test");
    let _ = std::fs::create_dir_all(&temp_dir);

    let router = CommandRouter::new(&temp_dir).await.unwrap();

    let suggestions = router.get_suggestions("he").await;
    assert!(suggestions.contains(&"/help".to_string()));

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_router_with_registry() {
    let temp_dir = std::env::temp_dir().join("sage_router_with_reg_test");
    let _ = std::fs::create_dir_all(&temp_dir);

    let mut registry = CommandRegistry::new(&temp_dir);
    registry.register_builtins();
    let registry = Arc::new(RwLock::new(registry));

    let router = CommandRouter::with_registry(registry);

    let list = router.list_commands().await;
    assert!(!list.system.is_empty());

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_router_reload() {
    let temp_dir = std::env::temp_dir().join("sage_router_reload_test");
    let _ = std::fs::create_dir_all(&temp_dir);

    let router = CommandRouter::new(&temp_dir).await.unwrap();

    let _count = router.reload().await.unwrap();

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_router_registry_accessor() {
    let temp_dir = std::env::temp_dir().join("sage_router_accessor_test");
    let _ = std::fs::create_dir_all(&temp_dir);

    let router = CommandRouter::new(&temp_dir).await.unwrap();

    let registry = router.registry();
    let guard = registry.read().await;
    assert!(guard.contains("help"));

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_command_list_empty() {
    let list = CommandList::default();
    assert_eq!(list.total(), 0);
    assert!(list.is_empty());
    assert!(list.all().is_empty());
}

#[test]
fn test_command_list_all_categories() {
    let list = CommandList {
        system: vec![RoutedCommand {
            name: "help".to_string(),
            category: CommandCategory::System,
            description: None,
        }],
        user: vec![RoutedCommand {
            name: "my-cmd".to_string(),
            category: CommandCategory::User,
            description: Some("My command".to_string()),
        }],
        mcp: vec![RoutedCommand {
            name: "mcp-tool".to_string(),
            category: CommandCategory::Mcp,
            description: None,
        }],
    };

    assert_eq!(list.total(), 3);
    assert!(!list.is_empty());
    assert_eq!(list.all().len(), 3);
}

#[test]
fn test_command_result_kind_empty() {
    let empty = CommandResult {
        expanded_prompt: String::new(),
        show_expansion: false,
        context_messages: Vec::new(),
        status_message: None,
        is_local: false,
        local_output: None,
        interactive: None,
        tool_restrictions: None,
        model_override: None,
    };
    assert!(matches!(empty.kind(), CommandResultKind::Empty));
}

#[test]
fn test_command_result_kind_local_empty_output() {
    let local = CommandResult {
        expanded_prompt: String::new(),
        show_expansion: false,
        context_messages: Vec::new(),
        status_message: None,
        is_local: true,
        local_output: None,
        interactive: None,
        tool_restrictions: None,
        model_override: None,
    };
    assert!(matches!(
        local.kind(),
        CommandResultKind::Local { output: "" }
    ));
}

#[test]
fn test_routed_command_debug() {
    let cmd = RoutedCommand {
        name: "test".to_string(),
        category: CommandCategory::System,
        description: Some("Test command".to_string()),
    };
    let debug_str = format!("{:?}", cmd);
    assert!(debug_str.contains("test"));
    assert!(debug_str.contains("System"));
}

#[test]
fn test_command_category_copy() {
    let cat1 = CommandCategory::System;
    let cat2 = cat1;
    assert_eq!(cat1, cat2);
}
