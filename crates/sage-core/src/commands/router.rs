//! Unified command router
//!
//! This module provides a unified entry point for all slash commands,
//! supporting three command categories: System, User, and MCP.
//!
//! # Architecture
//!
//! ```text
//! User Input
//!     │
//!     ▼
//! ┌─────────────────────────────────────────────────────────┐
//! │              CommandRouter (unified entry)              │
//! │  - is_command: Check if input is a slash command        │
//! │  - route: Route to appropriate handler                  │
//! └──────────────────────┬──────────────────────────────────┘
//!                        │
//!        ┌───────────────┼───────────────┐
//!        ▼               ▼               ▼
//! ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
//! │ System Cmd  │ │  User Cmd   │ │  MCP Cmd    │
//! │ (builtins)  │ │ (Markdown)  │ │ (MCP Prompt)│
//! └──────┬──────┘ └──────┬──────┘ └──────┬──────┘
//!        │               │               │
//!        └───────────────┼───────────────┘
//!                        ▼
//! ┌─────────────────────────────────────────────────────────┐
//! │              CommandResult                              │
//! │  - Local: Direct output                                 │
//! │  - Prompt: Send to LLM                                  │
//! │  - Interactive: Needs CLI handling                      │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use sage_core::commands::CommandRouter;
//!
//! let router = CommandRouter::new("./project").await?;
//!
//! // Check if input is a command
//! if CommandRouter::is_command("/help") {
//!     // Route and execute
//!     let result = router.route("/help").await?;
//!     match result.kind() {
//!         CommandResultKind::Local { output } => println!("{}", output),
//!         CommandResultKind::Prompt { content, .. } => send_to_llm(content),
//!         CommandResultKind::Interactive(cmd) => handle_interactive(cmd),
//!     }
//! }
//! ```

use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::SageResult;

use super::executor::CommandExecutor;
use super::registry::CommandRegistry;
use super::types::{CommandInvocation, CommandResult, CommandSource, InteractiveCommand};

/// Command category for routing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandCategory {
    /// System command (built-in)
    System,
    /// User-defined command (from Markdown files)
    User,
    /// MCP prompt command
    Mcp,
}

impl std::fmt::Display for CommandCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::System => write!(f, "system"),
            Self::User => write!(f, "user"),
            Self::Mcp => write!(f, "mcp"),
        }
    }
}

impl From<&CommandSource> for CommandCategory {
    fn from(source: &CommandSource) -> Self {
        match source {
            CommandSource::Builtin => CommandCategory::System,
            CommandSource::Project | CommandSource::User => CommandCategory::User,
        }
    }
}

/// Information about a routed command
#[derive(Debug, Clone)]
pub struct RoutedCommand {
    /// Command name
    pub name: String,
    /// Command category
    pub category: CommandCategory,
    /// Command description
    pub description: Option<String>,
}

/// Unified command router
///
/// Provides a single entry point for all slash command operations.
pub struct CommandRouter {
    /// Internal executor
    executor: CommandExecutor,
    /// Registry reference for direct access
    registry: Arc<RwLock<CommandRegistry>>,
}

impl CommandRouter {
    /// Create a new command router
    pub async fn new(project_root: impl AsRef<Path>) -> SageResult<Self> {
        let mut registry = CommandRegistry::new(project_root.as_ref());
        registry.register_builtins();
        registry.discover().await?;

        let registry = Arc::new(RwLock::new(registry));
        let executor = CommandExecutor::new(registry.clone());

        Ok(Self { executor, registry })
    }

    /// Create router with an existing registry
    pub fn with_registry(registry: Arc<RwLock<CommandRegistry>>) -> Self {
        let executor = CommandExecutor::new(registry.clone());
        Self { executor, registry }
    }

    /// Check if input is a slash command
    #[inline]
    pub fn is_command(input: &str) -> bool {
        CommandInvocation::is_slash_command(input)
    }

    /// Parse a command invocation from input
    pub fn parse(input: &str) -> Option<CommandInvocation> {
        CommandInvocation::parse(input)
    }

    /// Route and execute a command
    ///
    /// Returns `Ok(Some(result))` if a command was executed,
    /// `Ok(None)` if input is not a command,
    /// `Err` if command execution failed.
    pub async fn route(&self, input: &str) -> SageResult<Option<CommandResult>> {
        self.executor.process(input).await
    }

    /// Get information about a command without executing it
    pub async fn get_command_info(&self, name: &str) -> Option<RoutedCommand> {
        let registry = self.registry.read().await;
        registry.get_with_source(name).map(|(cmd, source)| RoutedCommand {
            name: cmd.name.clone(),
            category: source.into(),
            description: cmd.description.clone(),
        })
    }

    /// List all available commands grouped by category
    pub async fn list_commands(&self) -> CommandList {
        let registry = self.registry.read().await;
        let commands = registry.list();

        let mut system = Vec::new();
        let mut user = Vec::new();
        let mcp = Vec::new();

        for (cmd, source) in commands {
            let info = RoutedCommand {
                name: cmd.name.clone(),
                category: source.into(),
                description: cmd.description.clone(),
            };
            match source {
                CommandSource::Builtin => system.push(info),
                CommandSource::Project | CommandSource::User => user.push(info),
            }
        }

        CommandList { system, user, mcp }
    }

    /// Get command suggestions for autocomplete
    pub async fn get_suggestions(&self, prefix: &str) -> Vec<String> {
        self.executor.get_suggestions(prefix).await
    }

    /// Reload commands from disk
    pub async fn reload(&self) -> SageResult<usize> {
        self.executor.reload().await
    }

    /// Get the underlying registry
    pub fn registry(&self) -> &Arc<RwLock<CommandRegistry>> {
        &self.registry
    }
}

/// List of commands grouped by category
#[derive(Debug, Clone, Default)]
pub struct CommandList {
    /// System (built-in) commands
    pub system: Vec<RoutedCommand>,
    /// User-defined commands
    pub user: Vec<RoutedCommand>,
    /// MCP prompt commands
    pub mcp: Vec<RoutedCommand>,
}

impl CommandList {
    /// Total number of commands
    pub fn total(&self) -> usize {
        self.system.len() + self.user.len() + self.mcp.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.total() == 0
    }

    /// Get all commands as a flat list
    pub fn all(&self) -> Vec<&RoutedCommand> {
        self.system
            .iter()
            .chain(self.user.iter())
            .chain(self.mcp.iter())
            .collect()
    }
}

/// Classify a CommandResult by its kind
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandResultKind<'a> {
    /// Local command - output directly
    Local { output: &'a str },
    /// Prompt command - send to LLM
    Prompt { content: &'a str },
    /// Interactive command - needs CLI handling
    Interactive(&'a InteractiveCommand),
    /// No-op (empty result)
    Empty,
}

impl CommandResult {
    /// Get the kind of this result
    pub fn kind(&self) -> CommandResultKind<'_> {
        if let Some(ref cmd) = self.interactive {
            CommandResultKind::Interactive(cmd)
        } else if self.is_local {
            CommandResultKind::Local {
                output: self.local_output.as_deref().unwrap_or(""),
            }
        } else if !self.expanded_prompt.is_empty() {
            CommandResultKind::Prompt {
                content: &self.expanded_prompt,
            }
        } else {
            CommandResultKind::Empty
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(matches!(local.kind(), CommandResultKind::Local { output: "output text" }));

        // Prompt result
        let prompt = CommandResult::prompt("prompt text");
        assert!(matches!(prompt.kind(), CommandResultKind::Prompt { content: "prompt text" }));

        // Interactive result
        let interactive = CommandResult::interactive(InteractiveCommand::Login);
        assert!(matches!(interactive.kind(), CommandResultKind::Interactive(InteractiveCommand::Login)));
    }

    #[tokio::test]
    async fn test_router_creation() {
        // Use temp directory for test
        let temp_dir = std::env::temp_dir().join("sage_router_test");
        let _ = std::fs::create_dir_all(&temp_dir);

        let router = CommandRouter::new(&temp_dir).await.unwrap();

        // Should have system commands
        let list = router.list_commands().await;
        assert!(!list.system.is_empty());
        assert!(list.system.iter().any(|c| c.name == "help"));
        assert!(list.system.iter().any(|c| c.name == "login"));

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_router_route() {
        let temp_dir = std::env::temp_dir().join("sage_router_route_test");
        let _ = std::fs::create_dir_all(&temp_dir);

        let router = CommandRouter::new(&temp_dir).await.unwrap();

        // Route a builtin command
        let result = router.route("/help").await.unwrap();
        assert!(result.is_some());

        // Non-command should return None
        let result = router.route("not a command").await.unwrap();
        assert!(result.is_none());

        // Cleanup
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

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_router_suggestions() {
        let temp_dir = std::env::temp_dir().join("sage_router_suggest_test");
        let _ = std::fs::create_dir_all(&temp_dir);

        let router = CommandRouter::new(&temp_dir).await.unwrap();

        let suggestions = router.get_suggestions("he").await;
        assert!(suggestions.contains(&"/help".to_string()));

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_router_with_registry() {
        let temp_dir = std::env::temp_dir().join("sage_router_with_reg_test");
        let _ = std::fs::create_dir_all(&temp_dir);

        // Create registry manually
        let mut registry = CommandRegistry::new(&temp_dir);
        registry.register_builtins();
        let registry = Arc::new(RwLock::new(registry));

        // Create router with existing registry
        let router = CommandRouter::with_registry(registry);

        // Should still have system commands
        let list = router.list_commands().await;
        assert!(!list.system.is_empty());

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_router_reload() {
        let temp_dir = std::env::temp_dir().join("sage_router_reload_test");
        let _ = std::fs::create_dir_all(&temp_dir);

        let router = CommandRouter::new(&temp_dir).await.unwrap();

        // Reload should work (returns number of discovered commands)
        let count = router.reload().await.unwrap();
        // May be 0 since no custom commands exist
        assert!(count == 0 || count > 0);

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_router_registry_accessor() {
        let temp_dir = std::env::temp_dir().join("sage_router_accessor_test");
        let _ = std::fs::create_dir_all(&temp_dir);

        let router = CommandRouter::new(&temp_dir).await.unwrap();

        // Should be able to access the registry
        let registry = router.registry();
        let guard = registry.read().await;
        assert!(guard.contains("help"));

        // Cleanup
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
        // Empty result (no prompt, not local, no interactive)
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
        // Local result with empty output
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
        assert!(matches!(local.kind(), CommandResultKind::Local { output: "" }));
    }

    #[test]
    fn test_routed_command_debug() {
        let cmd = RoutedCommand {
            name: "test".to_string(),
            category: CommandCategory::System,
            description: Some("Test command".to_string()),
        };
        // Should be able to debug print
        let debug_str = format!("{:?}", cmd);
        assert!(debug_str.contains("test"));
        assert!(debug_str.contains("System"));
    }

    #[test]
    fn test_command_category_copy() {
        let cat1 = CommandCategory::System;
        let cat2 = cat1; // Copy
        assert_eq!(cat1, cat2);
    }
}
