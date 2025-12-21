//! Slash command registry
//!
//! This module provides the command registry for discovering and
//! managing slash commands from various sources.

use crate::error::{SageError, SageResult};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncReadExt;

use super::types::{CommandSource, SlashCommand};

/// Command registry for managing slash commands
pub struct CommandRegistry {
    /// Registered commands by name
    commands: HashMap<String, (SlashCommand, CommandSource)>,
    /// Project root directory
    project_root: PathBuf,
    /// User config directory
    user_config_dir: PathBuf,
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

    /// Register built-in commands
    pub fn register_builtins(&mut self) {
        // Help command
        self.register(
            SlashCommand::new(
                "help",
                "Show available commands and how to use them. List all slash commands with /commands.",
            )
            .with_description("Show help information")
            .builtin(),
            CommandSource::Builtin,
        );

        // Clear command
        self.register(
            SlashCommand::new("clear", "Clear the conversation history and start fresh.")
                .with_description("Clear conversation")
                .builtin(),
            CommandSource::Builtin,
        );

        // Compact command
        self.register(
            SlashCommand::new(
                "compact",
                "Summarize the conversation and compact the context to save tokens.",
            )
            .with_description("Compact conversation context")
            .builtin(),
            CommandSource::Builtin,
        );

        // Init command
        self.register(
            SlashCommand::new(
                "init",
                "Initialize a .sage directory in the current project with default settings.",
            )
            .with_description("Initialize Sage in project")
            .builtin(),
            CommandSource::Builtin,
        );

        // Config command
        self.register(
            SlashCommand::new("config", "Show or modify Sage configuration settings.")
                .with_description("Manage configuration")
                .builtin(),
            CommandSource::Builtin,
        );

        // Checkpoint command
        self.register(
            SlashCommand::new(
                "checkpoint",
                "Create a checkpoint of the current state. Usage: /checkpoint [name]",
            )
            .with_description("Create state checkpoint")
            .builtin(),
            CommandSource::Builtin,
        );

        // Restore command
        self.register(
            SlashCommand::new(
                "restore",
                "Restore to a previous checkpoint. Usage: /restore [checkpoint-id]",
            )
            .with_description("Restore from checkpoint")
            .builtin(),
            CommandSource::Builtin,
        );

        // Tasks command
        self.register(
            SlashCommand::new("tasks", "Show all running and completed background tasks.")
                .with_description("List background tasks")
                .builtin(),
            CommandSource::Builtin,
        );

        // Commands command
        self.register(
            SlashCommand::new("commands", "List all available slash commands.")
                .with_description("List slash commands")
                .builtin(),
            CommandSource::Builtin,
        );

        // Undo command
        self.register(
            SlashCommand::new(
                "undo",
                "Undo the last file changes by restoring files to their previous state. Usage: /undo [message-id]",
            )
            .with_description("Undo file changes")
            .builtin(),
            CommandSource::Builtin,
        );

        // Cost command - show session cost and token usage
        self.register(
            SlashCommand::new(
                "cost",
                "Show the total cost and token usage for the current session.",
            )
            .with_description("Show session cost and usage")
            .builtin(),
            CommandSource::Builtin,
        );

        // Context command - show context/token breakdown
        self.register(
            SlashCommand::new(
                "context",
                "Show the current context window usage and token breakdown.",
            )
            .with_description("Show context usage")
            .builtin(),
            CommandSource::Builtin,
        );

        // Status command - show agent status
        self.register(
            SlashCommand::new(
                "status",
                "Show Sage status including version, provider, model, and connection status.",
            )
            .with_description("Show agent status")
            .builtin(),
            CommandSource::Builtin,
        );

        // Resume command - resume previous session
        self.register(
            SlashCommand::new(
                "resume",
                "Resume a previous conversation session. Usage: /resume [session-id]",
            )
            .with_description("Resume a conversation")
            .builtin(),
            CommandSource::Builtin,
        );

        // Plan command - view/manage execution plan
        self.register(
            SlashCommand::new(
                "plan",
                "View or manage the current execution plan. Usage: /plan [open|clear]",
            )
            .with_description("View execution plan")
            .builtin(),
            CommandSource::Builtin,
        );
    }

    /// Discover commands from file system
    pub async fn discover(&mut self) -> SageResult<usize> {
        let mut count = 0;

        // Discover project commands
        let project_dir = self.project_root.join(".sage").join("commands");
        count += self
            .discover_from_dir(&project_dir, CommandSource::Project)
            .await?;

        // Discover user commands
        let user_dir = self.user_config_dir.join("commands");
        count += self
            .discover_from_dir(&user_dir, CommandSource::User)
            .await?;

        Ok(count)
    }

    /// Discover commands from a directory
    async fn discover_from_dir(&mut self, dir: &Path, source: CommandSource) -> SageResult<usize> {
        if !dir.exists() {
            return Ok(0);
        }

        let mut count = 0;
        let mut entries = fs::read_dir(dir)
            .await
            .map_err(|e| SageError::Storage(format!("Failed to read commands directory: {}", e)))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| SageError::Storage(format!("Failed to read directory entry: {}", e)))?
        {
            let path = entry.path();

            // Only process .md files
            if path.extension().map_or(false, |ext| ext == "md") {
                if let Some(command) = self.load_command_from_file(&path).await? {
                    // Don't override builtins
                    if !self
                        .commands
                        .get(&command.name)
                        .map_or(false, |(_, src)| *src == CommandSource::Builtin)
                    {
                        // Project commands override user commands
                        let should_register = match source {
                            CommandSource::Project => true,
                            CommandSource::User => !self
                                .commands
                                .get(&command.name)
                                .map_or(false, |(_, src)| *src == CommandSource::Project),
                            CommandSource::Builtin => true,
                        };

                        if should_register {
                            self.register(command, source.clone());
                            count += 1;
                        }
                    }
                }
            }
        }

        Ok(count)
    }

    /// Load a command from a markdown file
    async fn load_command_from_file(&self, path: &Path) -> SageResult<Option<SlashCommand>> {
        // Get command name from filename (without .md extension)
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| SageError::InvalidInput("Invalid command file name".to_string()))?
            .to_string();

        // Read file content
        let mut file = fs::File::open(path)
            .await
            .map_err(|e| SageError::Storage(format!("Failed to open command file: {}", e)))?;

        let mut content = String::new();
        file.read_to_string(&mut content)
            .await
            .map_err(|e| SageError::Storage(format!("Failed to read command file: {}", e)))?;

        // Parse frontmatter if present
        let (metadata, prompt_template) = self.parse_command_file(&content);

        let mut command =
            SlashCommand::new(name, prompt_template).with_source_path(path.to_path_buf());

        // Apply metadata
        if let Some(desc) = metadata.get("description") {
            command = command.with_description(desc.clone());
        }

        Ok(Some(command))
    }

    /// Parse command file with optional YAML frontmatter
    fn parse_command_file(&self, content: &str) -> (HashMap<String, String>, String) {
        let mut metadata = HashMap::new();

        // Check for YAML frontmatter (--- ... ---)
        if content.starts_with("---") {
            if let Some(end) = content[3..].find("---") {
                let frontmatter = &content[3..3 + end];
                let prompt_template = content[3 + end + 3..].trim().to_string();

                // Parse simple YAML key: value pairs
                for line in frontmatter.lines() {
                    if let Some(colon) = line.find(':') {
                        let key = line[..colon].trim().to_string();
                        let value = line[colon + 1..].trim().to_string();
                        metadata.insert(key, value);
                    }
                }

                return (metadata, prompt_template);
            }
        }

        (metadata, content.to_string())
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
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new(".")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs::File;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn test_registry_creation() {
        let registry = CommandRegistry::new("/project");
        assert_eq!(registry.count(), 0);
    }

    #[tokio::test]
    async fn test_register_command() {
        let mut registry = CommandRegistry::new("/project");
        let cmd = SlashCommand::new("test", "Test command");

        registry.register(cmd, CommandSource::Project);

        assert!(registry.contains("test"));
        assert_eq!(registry.count(), 1);
    }

    #[tokio::test]
    async fn test_get_command() {
        let mut registry = CommandRegistry::new("/project");
        registry.register(
            SlashCommand::new("test", "Test prompt").with_description("Test"),
            CommandSource::Project,
        );

        let cmd = registry.get("test").unwrap();
        assert_eq!(cmd.name, "test");
        assert_eq!(cmd.description, Some("Test".to_string()));
    }

    #[tokio::test]
    async fn test_register_builtins() {
        let mut registry = CommandRegistry::new("/project");
        registry.register_builtins();

        assert!(registry.contains("help"));
        assert!(registry.contains("clear"));
        assert!(registry.contains("checkpoint"));
        assert!(registry.builtin_count() > 0);
    }

    #[tokio::test]
    async fn test_list_commands() {
        let mut registry = CommandRegistry::new("/project");
        registry.register(SlashCommand::new("a", "A"), CommandSource::Project);
        registry.register(SlashCommand::new("b", "B"), CommandSource::User);

        let list = registry.list();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_list_by_source() {
        let mut registry = CommandRegistry::new("/project");
        registry.register(SlashCommand::new("p1", "P1"), CommandSource::Project);
        registry.register(SlashCommand::new("p2", "P2"), CommandSource::Project);
        registry.register(SlashCommand::new("u1", "U1"), CommandSource::User);

        let project = registry.list_by_source(CommandSource::Project);
        assert_eq!(project.len(), 2);

        let user = registry.list_by_source(CommandSource::User);
        assert_eq!(user.len(), 1);
    }

    #[tokio::test]
    async fn test_remove_command() {
        let mut registry = CommandRegistry::new("/project");
        registry.register(SlashCommand::new("test", "Test"), CommandSource::Project);

        assert!(registry.contains("test"));
        registry.remove("test");
        assert!(!registry.contains("test"));
    }

    #[tokio::test]
    async fn test_discover_from_directory() {
        let temp_dir = TempDir::new().unwrap();
        let commands_dir = temp_dir.path().join(".sage").join("commands");
        fs::create_dir_all(&commands_dir).await.unwrap();

        // Create a command file
        let cmd_file = commands_dir.join("greet.md");
        let mut file = File::create(&cmd_file).await.unwrap();
        file.write_all(b"Say hello to $ARGUMENTS").await.unwrap();

        let mut registry = CommandRegistry::new(temp_dir.path());
        let count = registry.discover().await.unwrap();

        assert_eq!(count, 1);
        assert!(registry.contains("greet"));
    }

    #[tokio::test]
    async fn test_discover_with_frontmatter() {
        let temp_dir = TempDir::new().unwrap();
        let commands_dir = temp_dir.path().join(".sage").join("commands");
        fs::create_dir_all(&commands_dir).await.unwrap();

        let cmd_file = commands_dir.join("fancy.md");
        let mut file = File::create(&cmd_file).await.unwrap();
        file.write_all(b"---\ndescription: A fancy command\n---\nDo something fancy")
            .await
            .unwrap();

        let mut registry = CommandRegistry::new(temp_dir.path());
        registry.discover().await.unwrap();

        let cmd = registry.get("fancy").unwrap();
        assert_eq!(cmd.description, Some("A fancy command".to_string()));
    }

    #[tokio::test]
    async fn test_project_overrides_user() {
        let temp_dir = TempDir::new().unwrap();

        // Create user command
        let user_dir = temp_dir.path().join("user").join("commands");
        fs::create_dir_all(&user_dir).await.unwrap();
        let mut f1 = File::create(user_dir.join("test.md")).await.unwrap();
        f1.write_all(b"User version").await.unwrap();

        // Create project command
        let project_dir = temp_dir
            .path()
            .join("project")
            .join(".sage")
            .join("commands");
        fs::create_dir_all(&project_dir).await.unwrap();
        let mut f2 = File::create(project_dir.join("test.md")).await.unwrap();
        f2.write_all(b"Project version").await.unwrap();

        let mut registry = CommandRegistry::new(temp_dir.path().join("project"))
            .with_user_config_dir(temp_dir.path().join("user"));

        registry.discover().await.unwrap();

        let cmd = registry.get("test").unwrap();
        assert_eq!(cmd.prompt_template, "Project version");
    }

    #[tokio::test]
    async fn test_builtin_not_overridden() {
        let temp_dir = TempDir::new().unwrap();
        let commands_dir = temp_dir.path().join(".sage").join("commands");
        fs::create_dir_all(&commands_dir).await.unwrap();

        // Try to override builtin
        let mut file = File::create(commands_dir.join("help.md")).await.unwrap();
        file.write_all(b"Overridden help").await.unwrap();

        let mut registry = CommandRegistry::new(temp_dir.path());
        registry.register_builtins();
        registry.discover().await.unwrap();

        let (cmd, source) = registry.get_with_source("help").unwrap();
        assert_eq!(*source, CommandSource::Builtin);
        assert!(!cmd.prompt_template.contains("Overridden"));
    }

    #[test]
    fn test_parse_frontmatter() {
        let registry = CommandRegistry::new("/project");

        let content = "---\ndescription: Test\nauthor: Me\n---\nPrompt content";
        let (metadata, prompt) = registry.parse_command_file(content);

        assert_eq!(metadata.get("description"), Some(&"Test".to_string()));
        assert_eq!(metadata.get("author"), Some(&"Me".to_string()));
        assert_eq!(prompt, "Prompt content");
    }

    #[test]
    fn test_parse_no_frontmatter() {
        let registry = CommandRegistry::new("/project");

        let content = "Just a prompt";
        let (metadata, prompt) = registry.parse_command_file(content);

        assert!(metadata.is_empty());
        assert_eq!(prompt, "Just a prompt");
    }
}
