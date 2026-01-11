//! Unified command router
//!
//! This module provides a unified entry point for all slash commands,
//! supporting three command categories: System, User, and MCP.

mod category;

pub use category::{CommandCategory, CommandList, RoutedCommand};

use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::SageResult;

use super::executor::CommandExecutor;
use super::registry::CommandRegistry;
use super::types::{CommandInvocation, CommandResult, CommandSource, InteractiveCommand};

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
    pub async fn route(&self, input: &str) -> SageResult<Option<CommandResult>> {
        self.executor.process(input).await
    }

    /// Get information about a command without executing it
    pub async fn get_command_info(&self, name: &str) -> Option<RoutedCommand> {
        let registry = self.registry.read().await;
        registry
            .get_with_source(name)
            .map(|(cmd, source)| RoutedCommand {
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
mod tests;
