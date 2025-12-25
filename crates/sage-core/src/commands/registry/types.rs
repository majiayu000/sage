//! Command registry types and core implementation

use crate::error::SageResult;
use std::collections::HashMap;
use std::path::PathBuf;

use super::super::types::{CommandSource, SlashCommand};

/// Command registry for managing slash commands
pub struct CommandRegistry {
    /// Registered commands by name
    pub(super) commands: HashMap<String, (SlashCommand, CommandSource)>,
    /// Project root directory
    pub(super) project_root: PathBuf,
    /// User config directory
    pub(super) user_config_dir: PathBuf,
}

impl CommandRegistry {
    /// Create a new command registry
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        let user_config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("sage");

        Self {
            commands: HashMap::new(),
            project_root: project_root.into(),
            user_config_dir,
        }
    }

    /// Set user config directory
    pub fn with_user_config_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.user_config_dir = dir.into();
        self
    }

    /// Register a command
    pub fn register(&mut self, command: SlashCommand, source: CommandSource) {
        self.commands
            .insert(command.name.clone(), (command, source));
    }

    /// Get a command by name
    pub fn get(&self, name: &str) -> Option<&SlashCommand> {
        self.commands.get(name).map(|(cmd, _)| cmd)
    }

    /// Get a command with its source
    pub fn get_with_source(&self, name: &str) -> Option<(&SlashCommand, &CommandSource)> {
        self.commands.get(name).map(|(cmd, src)| (cmd, src))
    }

    /// List all registered commands
    pub fn list(&self) -> Vec<(&SlashCommand, &CommandSource)> {
        self.commands
            .values()
            .map(|(cmd, src)| (cmd, src))
            .collect()
    }

    /// List commands by source
    pub fn list_by_source(&self, source: CommandSource) -> Vec<&SlashCommand> {
        self.commands
            .values()
            .filter(|(_, src)| *src == source)
            .map(|(cmd, _)| cmd)
            .collect()
    }

    /// Check if a command exists
    pub fn contains(&self, name: &str) -> bool {
        self.commands.contains_key(name)
    }

    /// Remove a command
    pub fn remove(&mut self, name: &str) -> Option<SlashCommand> {
        self.commands.remove(name).map(|(cmd, _)| cmd)
    }

    /// Clear all commands
    pub fn clear(&mut self) {
        self.commands.clear();
    }

    /// Get project commands directory
    pub fn project_commands_dir(&self) -> PathBuf {
        self.project_root.join(".sage").join("commands")
    }

    /// Get user commands directory
    pub fn user_commands_dir(&self) -> PathBuf {
        self.user_config_dir.join("commands")
    }

    /// Get command count
    pub fn count(&self) -> usize {
        self.commands.len()
    }

    /// Get builtin command count
    pub fn builtin_count(&self) -> usize {
        self.commands
            .values()
            .filter(|(_, src)| *src == CommandSource::Builtin)
            .count()
    }

    /// Discover commands from file system
    pub async fn discover(&mut self) -> SageResult<usize> {
        let mut count = 0;

        // Discover project commands
        let project_dir = self.project_commands_dir();
        count += self
            .discover_from_dir(&project_dir, CommandSource::Project)
            .await?;

        // Discover user commands
        let user_dir = self.user_commands_dir();
        count += self
            .discover_from_dir(&user_dir, CommandSource::User)
            .await?;

        Ok(count)
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new(".")
    }
}
